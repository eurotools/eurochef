use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Cursor, Read, Seek, Write},
    path::Path,
};

use anyhow::Context;
use base64::Engine;
use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    common::{EXVector2, EXVector3},
    entity::{EXGeoBaseEntity, EXGeoEntity_TriStrip},
    header::EXGeoHeader,
    texture::EXGeoTexture,
    versions::Platform,
};
use eurochef_shared::platform::texture;
use image::{EncodableLayout, ImageOutputFormat, RgbaImage};
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};

use crate::{
    edb::{gltf_export::dump_single_mesh_to_scene, TICK_STRINGS},
    PlatformArg,
};

use super::gltf_export::{TriStrip, UXVertex};

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

    if platform != Platform::Pc && platform != Platform::Xbox {
        anyhow::bail!("Entity extraction is only supported for PC and Xbox (for now)")
    }

    println!("Selected platform {platform:?}");

    let mut texture_uri_map: HashMap<u32, String> = HashMap::new();
    if dont_embed_textures {
        for t in &header.texture_list.data {
            texture_uri_map.insert(
                t.common.hashcode,
                format!("{:08x}_frame0.png", t.common.hashcode),
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

        // TODO: Library needs to do this and whatnot
        let mut data = vec![];
        let texture_decoder = texture::create_for_platform(platform);
        for t in header.texture_list.data.iter().progress_with(pb) {
            reader.seek(std::io::SeekFrom::Start(t.common.address as u64))?;
            let uri = format!("{:08x}_frame0.png", t.common.hashcode);

            let tex = reader
                .read_type_args::<EXGeoTexture>(endian, (header.version, platform))
                .context("Failed to read basetexture")?;

            let calculated_size = texture_decoder.get_data_size(
                tex.width as u32,
                tex.height as u32,
                tex.depth as u32,
                tex.format,
            );

            if let Err(e) = calculated_size {
                println!("Failed to extract texture {:x}: {:?}", t.common.hashcode, e);
                texture_uri_map.insert(t.common.hashcode, uri);
                continue;
            }

            data.clear();
            data.resize(
                tex.data_size
                    .map(|v| v as usize)
                    .unwrap_or(calculated_size.unwrap()),
                0u8,
            );

            std::fs::create_dir_all(output_folder)?;

            let mut output = RgbaImage::new(tex.width as u32, tex.height as u32);
            reader.seek(std::io::SeekFrom::Start(
                tex.frame_offsets[0].offset_absolute(),
            ))?;

            if let Err(e) = reader.read_exact(&mut data) {
                println!("Failed to read texture {:x}: {}", t.common.hashcode, e);
                texture_uri_map.insert(t.common.hashcode, uri);
                continue;
            }

            if let Err(e) = texture_decoder.decode(
                &data,
                &mut output,
                tex.width as u32,
                tex.height as u32,
                tex.depth as u32,
                tex.format,
            ) {
                println!("Texture {:08x} failed to decode: {}", t.common.hashcode, e);
                texture_uri_map.insert(t.common.hashcode, uri);
                continue;
            }

            let mut cur = Cursor::new(Vec::new());
            image::write_buffer_with_format(
                &mut cur,
                output.as_bytes(),
                output.width(),
                output.height(),
                image::ColorType::Rgba8,
                ImageOutputFormat::Png,
            )?;
            let mut uri = "data:image/png;base64,".to_string();
            base64::engine::general_purpose::STANDARD.encode_string(&cur.into_inner(), &mut uri);
            texture_uri_map.insert(t.common.hashcode, uri);
        }
    }

    let pb = ProgressBar::new(header.entity_list.data.len() as u64)
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

    std::fs::create_dir_all(output_folder)?;
    for e in header.entity_list.data.iter().progress_with(pb) {
        reader.seek(std::io::SeekFrom::Start(e.common.address as u64))?;

        println!("{:x}", e.common.address);
        let ent = reader
            .read_type_args::<EXGeoBaseEntity>(endian, (header.version,))
            .context("Failed to read entity");

        if let Err(err) = ent {
            println!("Failed to read entity {:x} {:?}", e.common.hashcode, err);
            continue;
        }

        let ent = ent.unwrap();

        let mut vertex_data = vec![];
        let mut indices = vec![];
        let mut strips = vec![];

        read_entity(
            &ent,
            &mut vertex_data,
            &mut indices,
            &mut strips,
            endian,
            header.version,
            &mut reader,
            4,
            remove_transparent,
        )?;

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

        if strips.len() <= 0 {
            println!(
                "Skipping entity {:x} because it doesnt have any tristrips (v={}/i={})",
                e.common.hashcode,
                vertex_data.len(),
                indices.len()
            );
            continue;
        }

        let gltf = dump_single_mesh_to_scene(
            &vertex_data,
            &indices,
            &strips,
            ![252, 250, 240, 221].contains(&header.version),
            &texture_uri_map,
            e.common.hashcode,
        );
        let mut outfile =
            File::create(output_folder.join(format!("{:x}.gltf", e.common.hashcode)))?;

        let json_string =
            gltf::json::serialize::to_string(&gltf).context("glTF serialization error")?;

        outfile.write_all(json_string.as_bytes())?;
    }

    Ok(())
}

fn read_entity<R: Read + Seek>(
    ent: &EXGeoBaseEntity,
    vertex_data: &mut Vec<UXVertex>,
    indices: &mut Vec<u32>,
    strips: &mut Vec<TriStrip>,
    endian: Endian,
    version: u32,
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
                data,
                depth_limit - 1,
                remove_transparent,
            )?;
        }
    } else if ent.object_type == 0x601 {
        let vertex_offset = vertex_data.len() as u32;
        let nent = ent.normal_entity.as_ref().unwrap();

        data.seek(std::io::SeekFrom::Start(nent.vertex_data.offset_absolute()))?;
        // TODO: Should probably not fall back to 3-3-2 but raise an error instead?
        for _ in 0..nent.vertex_count {
            match version {
                252 | 250 | 240 | 221 => {
                    let d = data.read_type::<(EXVector3, u32, EXVector2)>(endian)?;
                    vertex_data.push(UXVertex {
                        pos: d.0,
                        norm: [0f32, 0f32, 0f32],
                        uv: d.2,
                    });
                }
                _ => {
                    vertex_data.push(UXVertex {
                        pos: data.read_type(endian)?,
                        norm: data.read_type(endian)?,
                        uv: data.read_type(endian)?,
                    });
                }
            }
        }

        data.seek(std::io::SeekFrom::Start(nent.index_data.offset_absolute()))?;
        let new_indices: Vec<u32> = (0..nent.index_count)
            .map(|_| data.read_type::<u16>(endian).unwrap() as u32)
            .collect();

        // indices.extend(&new_indices);

        let mut tristrips: Vec<EXGeoEntity_TriStrip> = vec![];
        data.seek(std::io::SeekFrom::Start(
            nent.tristrip_data.offset_absolute(),
        ))?;
        for _ in 0..nent.tristrip_count {
            tristrips.push(data.read_type_args(endian, (version,))?);
        }

        let mut index_offset_local = 0;
        for t in tristrips {
            if t.tricount < 1 {
                break;
            }

            if remove_transparent && t.trans_type != 0 {
                index_offset_local += t.tricount + 2;
                continue;
            }

            let texture_index = if ent.flags & 0x1 != 0 {
                // Index from texture list instead of the "global" array
                if t.texture_index < nent.texture_list.data.textures.len() as i32 {
                    nent.texture_list.data.textures[t.texture_index as usize] as i32
                } else {
                    println!("Warning: tried to get texture #{} from texture list, but list only has {} elements!", t.texture_index, nent.texture_list.data.textures.len());
                    -1
                }
            } else {
                t.texture_index
            };

            strips.push(TriStrip {
                start_index: indices.len() as u32,
                index_count: t.tricount * 3,
                texture_hash: texture_index as u32,
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
        anyhow::bail!("Invalid obj type 0x{:x}", ent.object_type)
    }

    Ok(())
}
