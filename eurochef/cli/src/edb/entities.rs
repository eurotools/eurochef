use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Cursor, Seek},
    path::Path,
};

use anyhow::Context;
use base64::Engine;
use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    entity::EXGeoEntity,
    header::EXGeoHeader,
    versions::Platform,
};
use eurochef_shared::{entities::read_entity, textures::UXGeoTexture};
use image::ImageOutputFormat;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};

use crate::{
    edb::{
        gltf_export::{self},
        TICK_STRINGS,
    },
    PlatformArg,
};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Transparency {
    Opaque,
    Blend,
    Additive,
    Cutout,
}

pub fn execute_command(
    filename: String,
    platform: Option<PlatformArg>,
    output_folder: Option<String>,
    dont_embed_textures: bool,
    remove_transparent: bool,
) -> anyhow::Result<()> {
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

    let platform = platform
        .map(|p| p.into())
        .or(Platform::from_path(&filename))
        .expect("Failed to detect platform");

    match platform {
        Platform::Pc | Platform::Xbox | Platform::Xbox360 /* | Platform::GameCube | Platform::Wii */ => {}
        _ => {
            anyhow::bail!("Entity extraction is only supported for PC and Xbox (360) (for now)")
        }
    }

    info!("Selected platform {platform:?}");

    let mut texture_uri_map: HashMap<u32, (String, Transparency)> = HashMap::new();
    if dont_embed_textures {
        for t in &header.texture_list {
            texture_uri_map.insert(
                t.common.hashcode,
                (
                    format!("{:08x}_frame0.png", t.common.hashcode),
                    Transparency::Opaque,
                ),
            );
        }
    } else {
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
        for t in textures.into_iter() {
            let hash_str = format!("0x{:x}", t.hashcode);
            let _span = error_span!("texture", hash = %hash_str);
            let _span_enter = _span.enter();

            if t.frames.len() == 0 {
                error!("Skipping texture with no frames");
                continue;
            }

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
    }

    std::fs::create_dir_all(output_folder)?;
    let mut entity_offsets: Vec<(u64, String)> = header
        .entity_list
        .iter()
        .map(|e| (e.common.address as u64, format!("{:x}", e.common.hashcode)))
        .collect();

    // Find entities in refpointers
    for (i, r) in header.refpointer_list.iter().enumerate() {
        reader.seek(std::io::SeekFrom::Start(r.address as u64))?;
        let etype = reader.read_type::<u32>(endian)?;

        if etype == 0x601 || etype == 0x603 {
            entity_offsets.push((r.address as u64, format!("ref_{i}")))
        }
    }

    let pb = ProgressBar::new(entity_offsets.len() as u64)
        .with_finish(indicatif::ProgressFinish::AndLeave);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {msg} ({pos}/{len})",
        )
        .unwrap()
        .progress_chars("##-")
        .tick_chars(&TICK_STRINGS),
    );
    pb.set_message("Extracting entities");

    for (ent_offset, ent_id) in entity_offsets.iter().progress_with(pb) {
        let _span = error_span!("entity", id = %ent_id);
        let _span_enter = _span.enter();

        reader.seek(std::io::SeekFrom::Start(*ent_offset))?;

        let ent = reader.read_type_args::<EXGeoEntity>(endian, (header.version, platform));
        if let Err(err) = ent {
            error!("Failed to read entity: {err}");
            continue;
        }

        let ent = ent.unwrap();

        if let EXGeoEntity::Mesh(ref mesh) = ent {
            if mesh.vertex_count == 0 {
                warn!(
                    "Skipping entity without vertex data! (v={}/i={}/t={})",
                    mesh.vertex_count, mesh.index_count, mesh.tristrip_count
                );
                continue;
            }
        }

        let mut vertex_data = vec![];
        let mut indices = vec![];
        let mut strips = vec![];

        if let Err(err) = read_entity(
            &ent,
            &mut vertex_data,
            &mut indices,
            &mut strips,
            endian,
            header.version,
            platform,
            &mut reader,
            4,
            remove_transparent,
        ) {
            error!("Failed to extract entity: {err}");
            continue;
        }

        if strips.len() <= 0 {
            warn!(
                "Processed entity doesnt have tristrips! (v={}/i={}/t={})",
                vertex_data.len(),
                indices.len(),
                strips.len()
            );
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

        let mut gltf = gltf_export::create_mesh_scene(&ent_id);
        gltf_export::add_mesh_to_scene(
            &mut gltf,
            &vertex_data,
            &indices,
            &strips,
            ![252, 250, 240, 221].contains(&header.version),
            &texture_uri_map,
            header.hashcode,
        );

        let mut outfile = File::create(output_folder.join(format!("{}.gltf", ent_id)))?;
        gltf::json::serialize::to_writer(&mut outfile, &gltf)
            .context("glTF serialization error")?;
    }

    info!("Successfully extracted entities!");

    Ok(())
}
