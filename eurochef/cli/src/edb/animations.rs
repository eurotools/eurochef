use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Cursor, Seek},
    path::Path,
};

use anyhow::Context;
use base64::Engine;
use eurochef_edb::{
    anim::EXGeoBaseAnimSkin, binrw::BinReaderExt, edb::EdbFile, entity::EXGeoEntity,
    versions::Platform,
};
use eurochef_shared::{entities::read_entity, textures::UXGeoTexture};
use image::ImageOutputFormat;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};

use crate::PlatformArg;

use super::{entities::Transparency, gltf_export, TICK_STRINGS};

pub fn execute_command(
    filename: String,
    platform: Option<PlatformArg>,
    output_folder: Option<String>,
) -> anyhow::Result<()> {
    warn!("THIS COMMAND IS A WORK IN PROGRESS");

    let output_folder = output_folder.unwrap_or(format!(
        "./entities/{}/",
        Path::new(&filename)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    ));
    let output_folder = Path::new(&output_folder);

    let platform = platform
        .map(|p| p.into())
        .or(Platform::from_path(&filename))
        .expect("Failed to detect platform");

    if platform != Platform::Pc && platform != Platform::Xbox && platform != Platform::Xbox360 {
        anyhow::bail!("Entity extraction is only supported for PC and Xbox (360) (for now)")
    }

    let mut file = File::open(&filename)?;
    let mut reader = BufReader::new(&mut file);
    let mut edb = EdbFile::new(&mut reader, platform)?;
    let header = edb.header.clone();

    if header.animskin_list.len() == 0 {
        warn!("File does not contain any animation skins!");
        return Ok(());
    }

    std::fs::create_dir_all(output_folder)?;

    let mut texture_uri_map: HashMap<u32, (String, Transparency)> = HashMap::new();
    let pb = ProgressBar::new(header.texture_list.len() as u64)
        .with_finish(indicatif::ProgressFinish::AndLeave);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {msg} ({pos}/{len})",
        )
        .unwrap()
        .progress_chars("##-")
        .tick_chars(&TICK_STRINGS),
    );
    pb.set_message("Extracting textures");

    let textures = UXGeoTexture::read_all(&mut edb);
    for (_, it) in textures.into_iter() {
        let hash_str = format!("0x{:x}", it.hashcode);
        let _span = error_span!("texture", hash = %hash_str);
        let _span_enter = _span.enter();

        if let Ok(t) = it.data {
            if t.frames.len() == 0 {
                error!("Skipping texture with no frames");
                continue;
            }

            // TODO(cohae): This is very wrong, textures only specify whether they're cutout. see GUI entity renderer for more info
            let flags_shift = if header.version == 248 { 0x19 } else { 0x18 };

            let is_transparent_blend = (((t.flags >> flags_shift) >> 6) & 1) != 0;
            let is_transparent_cutout = (((t.flags >> flags_shift) >> 5) & 1) != 0;
            let transparency = match (is_transparent_blend, is_transparent_cutout) {
                (false, false) => Transparency::Opaque,
                (true, false) => Transparency::Blend,
                (false, true) => Transparency::Cutout,
                _ => Transparency::Blend,
            };

            let mut cur = Cursor::new(Vec::new());
            image::write_buffer_with_format(
                &mut cur,
                &t.frames[0],
                t.width as u32,
                t.height as u32,
                image::ColorType::Rgba8,
                ImageOutputFormat::Png,
            )?;

            let mut uri = "data:image/png;base64,".to_string();
            base64::engine::general_purpose::STANDARD.encode_string(&cur.into_inner(), &mut uri);
            texture_uri_map.insert(it.hashcode, (uri, transparency));
        }
    }

    let pb = ProgressBar::new(header.animskin_list.len() as u64)
        .with_finish(indicatif::ProgressFinish::AndLeave);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {msg} ({pos}/{len})",
        )
        .unwrap()
        .progress_chars("##-")
        .tick_chars(&TICK_STRINGS),
    );
    pb.set_message("Extracting animskins");

    for a in header.animskin_list.iter().progress_with(pb) {
        let skin_id = format!("{:x}", a.common.hashcode);
        let _span = error_span!("animskin", id = %skin_id);
        let _span_enter = _span.enter();
        edb.seek(std::io::SeekFrom::Start(a.common.address as u64))?;

        let skin = edb
            .read_type_args::<EXGeoBaseAnimSkin>(edb.endian, (header.version,))
            .context("Failed to read animation")?;

        let entity_indices: Vec<u32> = skin
            .entities
            .iter()
            .chain(skin.more_entities.iter())
            .map(|d| d.entity_index & 0x00ffffff)
            .collect();

        let mut gltf = gltf_export::create_mesh_scene(&skin_id);

        for entity_index in entity_indices {
            let e = &header.entity_list[entity_index as usize];
            let ent_id = format!("{:x}", e.common.hashcode);
            let _espan = error_span!("entity", id = %ent_id);
            let _espan_enter = _espan.enter();

            edb.seek(std::io::SeekFrom::Start(e.common.address as u64))?;

            let ent = edb.read_type_args::<EXGeoEntity>(edb.endian, (header.version, platform));

            if let Err(err) = ent {
                error!("Failed to read entity: {err}");
                continue;
            }

            let ent = ent.unwrap();

            let mut vertex_data = vec![];
            let mut indices = vec![];
            let mut strips = vec![];

            if let Err(err) = read_entity(
                &ent,
                &mut vertex_data,
                &mut indices,
                &mut strips,
                edb.endian,
                header.version,
                platform,
                &mut edb,
                4,
                false,
                true,
            ) {
                error!("Failed to extract entity: {err}");
                continue;
            }

            // Process vertex data (flipping vertex data and UVs)
            for v in &mut vertex_data {
                v.pos[0] = -v.pos[0];
            }

            // Look up texture hashcodes
            for t in &mut strips {
                if t.texture_index != u32::MAX {
                    t.texture_index = header.texture_list[t.texture_index as usize]
                        .common
                        .hashcode;
                }
            }

            if vertex_data.len() == 0 {
                warn!(
                    "Processed entity doesnt have vertex data! (v={}/i={}/t={})",
                    vertex_data.len(),
                    indices.len(),
                    strips.len()
                );
            }

            if strips.len() <= 0 {
                continue;
            }

            gltf_export::add_mesh_to_scene(
                &mut gltf,
                &vertex_data,
                &indices,
                &strips,
                ![252, 250, 240, 221].contains(&header.version),
                &texture_uri_map,
                header.hashcode,
            );
        }

        let mut outfile = File::create(output_folder.join(format!("{}.gltf", skin_id)))?;
        gltf::json::serialize::to_writer(&mut outfile, &gltf)
            .context("glTF serialization error")?;
    }

    Ok(())
}
