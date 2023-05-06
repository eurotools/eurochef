use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Cursor, Read, Seek},
    path::Path,
};

use anyhow::Context;
use base64::Engine;
use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    common::{EXVector2, EXVector3},
    entity::{EXGeoBaseEntity, EXGeoEntity_TriStrip},
    header::EXGeoHeader,
    versions::Platform,
};
use eurochef_shared::textures::UXGeoTexture;
use image::ImageOutputFormat;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};

use crate::{
    edb::{
        gltf_export::{self},
        TICK_STRINGS,
    },
    PlatformArg,
};

use super::gltf_export::{TriStrip, UXVertex};

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

    if platform != Platform::Pc && platform != Platform::Xbox && platform != Platform::Xbox360 {
        anyhow::bail!("Entity extraction is only supported for PC and Xbox (360) (for now)")
    }

    info!("Selected platform {platform:?}");

    let mut texture_uri_map: HashMap<u32, (String, Transparency)> = HashMap::new();
    if dont_embed_textures {
        for t in &header.texture_list.data {
            texture_uri_map.insert(
                t.common.hashcode,
                (
                    format!("{:08x}_frame0.png", t.common.hashcode),
                    Transparency::Opaque,
                ),
            );
        }
    } else {
        let pb = ProgressBar::new(header.texture_list.data.len() as u64)
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
        .data
        .iter()
        .map(|e| (e.common.address as u64, format!("{:x}", e.common.hashcode)))
        .collect();

    // Find entities in refpointers
    for (i, r) in header.refpointer_list.data.iter().enumerate() {
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

        let ent = reader.read_type_args::<EXGeoBaseEntity>(endian, (header.version,));

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

        // Process vertex data (flipping vertex data and UVs)
        for v in &mut vertex_data {
            v.pos[0] = -v.pos[0];
        }

        // Look up texture hashcodes
        for t in &mut strips {
            if t.texture_hash != u32::MAX {
                t.texture_hash = header.texture_list.data[t.texture_hash as usize]
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

pub fn read_entity<R: Read + Seek>(
    ent: &EXGeoBaseEntity,
    vertex_data: &mut Vec<UXVertex>,
    indices: &mut Vec<u32>,
    strips: &mut Vec<TriStrip>,
    endian: Endian,
    version: u32,
    platform: Platform,
    data: &mut R,
    depth_limit: u32,
    remove_transparent: bool,
) -> anyhow::Result<()> {
    if depth_limit == 0 {
        anyhow::bail!("Entity recursion limit reached!");
    }

    if ent.object_type == 0x603 {
        for e in ent.split_entity.as_ref().unwrap().entities.iter() {
            read_entity(
                &e.data,
                vertex_data,
                indices,
                strips,
                endian,
                version,
                platform,
                data,
                depth_limit - 1,
                remove_transparent,
            )?;
        }
    } else if ent.object_type == 0x601 {
        let vertex_offset = vertex_data.len() as u32;
        let nent = ent.normal_entity.as_ref().unwrap();

        data.seek(std::io::SeekFrom::Start(nent.vertex_data.offset_absolute()))?;

        // TODO(cohae): 0BADF002 + vertex count???
        if platform == Platform::Xbox360 {
            for _ in 0..2 {
                data.read_type::<u32>(endian).unwrap();
            }
        }

        for _ in 0..nent.vertex_count {
            match version {
                252 | 250 | 251 | 240 | 221 => {
                    let d = data.read_type::<(EXVector3, u32, EXVector2)>(endian)?;
                    vertex_data.push(UXVertex {
                        pos: d.0,
                        norm: [0f32, 0f32, 0f32],
                        uv: d.2,
                    });
                }
                248 | 259 | 260 => {
                    if platform == Platform::Xbox360 {
                        // TODO(cohae): Wacky x360-specific format
                        let d = data.read_type::<(EXVector3, u32, EXVector3, u32)>(endian)?;
                        vertex_data.push(UXVertex {
                            pos: d.0,
                            norm: d.2,
                            uv: [0.0, 0.0],
                        });
                    } else {
                        vertex_data.push(UXVertex {
                            pos: data.read_type(endian)?,
                            norm: data.read_type(endian)?,
                            uv: data.read_type(endian)?,
                        });
                    }
                }
                _ => {
                    panic!(
                        "Vertex format for version {version} is not known yet, report to cohae!"
                    );
                }
            }
        }

        if platform == Platform::Xbox360 {
            for i in 0..nent.vertex_count as usize {
                vertex_data[i].uv = data.read_type(endian)?;
            }
        }

        data.seek(std::io::SeekFrom::Start(nent.index_data.offset_absolute()))?;

        // TODO(cohae): 0BADF001
        if platform == Platform::Xbox360 {
            for _ in 0..2 {
                data.read_type::<u16>(endian).unwrap();
            }
        }

        let new_indices: Vec<u32> = (0..nent.index_count)
            .map(|_| data.read_type::<u16>(endian).unwrap() as u32)
            .collect();

        let mut tristrips: Vec<EXGeoEntity_TriStrip> = vec![];
        data.seek(std::io::SeekFrom::Start(
            nent.tristrip_data.offset_absolute(),
        ))?;
        for _ in 0..nent.tristrip_count {
            tristrips.push(data.read_type_args(endian, (version, platform))?);
        }

        let mut index_offset_local = 0;
        for t in tristrips {
            if t.tricount < 1 {
                break;
            }

            // Skip transparent surfaces that use additive blending
            if t.trans_type == 1 {
                index_offset_local += t.tricount + 2;
                continue;
            }

            let texture_index = if ent.flags & 0x1 != 0 {
                // Index from texture list instead of the "global" array
                if t.texture_index < nent.texture_list.data.textures.len() as i32 {
                    nent.texture_list.data.textures[t.texture_index as usize] as i32
                } else {
                    error!("Tried to get texture #{} from texture list, but list only has {} elements!", t.texture_index, nent.texture_list.data.textures.len());
                    -1
                }
            } else {
                t.texture_index
            };

            strips.push(TriStrip {
                start_index: indices.len() as u32,
                index_count: t.tricount * 3,
                texture_hash: texture_index as u32,
                transparency: t.trans_type,
            });

            for i in (index_offset_local as usize)..(index_offset_local + t.tricount) as usize {
                if (i - index_offset_local as usize) % 2 == 0 {
                    indices.extend([
                        vertex_offset + new_indices[i + 2] as u32,
                        vertex_offset + new_indices[i + 1] as u32,
                        vertex_offset + new_indices[i] as u32,
                    ])
                } else {
                    indices.extend([
                        vertex_offset + new_indices[i] as u32,
                        vertex_offset + new_indices[i + 1] as u32,
                        vertex_offset + new_indices[i + 2] as u32,
                    ])
                }
            }
            index_offset_local += t.tricount + 2;
        }
    } else {
        if (0x600..0x700).contains(&ent.object_type) {
            anyhow::bail!("Unsupported object type 0x{:x}", ent.object_type)
        }

        anyhow::bail!("Invalid obj type 0x{:x}", ent.object_type)
    }

    Ok(())
}
