use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Cursor, Seek},
    path::Path,
};

use anyhow::Context;
use base64::Engine;
use eurochef_edb::{
    anim::EXGeoBaseAnimSkin,
    binrw::{BinReaderExt, Endian},
    entity::EXGeoBaseEntity,
    header::EXGeoHeader,
    versions::Platform,
};
use eurochef_shared::textures::UXGeoTexture;
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

    let mut file = File::open(&filename)?;
    let mut reader = BufReader::new(&mut file);
    let endian = if reader.read_ne::<u8>().unwrap() == 0x47 {
        Endian::Big
    } else {
        Endian::Little
    };
    reader.seek(std::io::SeekFrom::Start(0))?;

    let header = reader
        .read_type::<EXGeoHeader>(endian)
        .expect("Failed to read header");

    if header.animskin_list.len() == 0 {
        warn!("File does not contain any animation skins!");
        return Ok(());
    }

    std::fs::create_dir_all(output_folder)?;

    let platform = platform
        .map(|p| p.into())
        .or(Platform::from_path(&filename))
        .expect("Failed to detect platform");

    if platform != Platform::Pc && platform != Platform::Xbox && platform != Platform::Xbox360 {
        anyhow::bail!("Entity extraction is only supported for PC and Xbox (360) (for now)")
    }

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

    let textures = UXGeoTexture::read_all(&header, &mut reader, platform)?;
    for t in textures.into_iter().progress_with(pb) {
        let hash_str = format!("0x{:x}", t.hashcode);
        let _span = error_span!("texture", hash = %hash_str);
        let _span_enter = _span.enter();

        trace!(
            "tex={:x} sg=0b{:016b} flags=0b{:032b}",
            t.hashcode,
            t.flags >> 0x18,
            t.flags
        );

        // cohae: This is wrong on a few levels, but it's just for transparency
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
        texture_uri_map.insert(t.hashcode, (uri, transparency));
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
        reader.seek(std::io::SeekFrom::Start(a.common.address as u64))?;

        let skin = reader
            .read_type_args::<EXGeoBaseAnimSkin>(endian, (header.version,))
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

            reader.seek(std::io::SeekFrom::Start(e.common.address as u64))?;

            let ent = reader.read_type_args::<EXGeoBaseEntity>(endian, (header.version,));

            if let Err(err) = ent {
                error!("Failed to read entity: {err}");
                continue;
            }

            let ent = ent.unwrap();

            let mut vertex_data = vec![];
            let mut indices = vec![];
            let mut strips = vec![];

            if let Err(err) = super::entities::read_entity(
                &ent,
                &mut vertex_data,
                &mut indices,
                &mut strips,
                endian,
                header.version,
                platform,
                &mut reader,
                4,
                false,
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
                if t.texture_hash != u32::MAX {
                    t.texture_hash = header.texture_list[t.texture_hash as usize].common.hashcode;
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
