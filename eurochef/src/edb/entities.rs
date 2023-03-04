use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
};

use anyhow::Context;
use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    common::{EXVector2, EXVector3},
    entity::{EXGeoBaseEntity, EXGeoEntity_TriStrip},
    header::EXGeoHeader,
    versions::Platform,
};
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
    let endian = if file.read_ne::<u8>().unwrap() == 0x47 {
        Endian::Big
    } else {
        Endian::Little
    };
    file.seek(std::io::SeekFrom::Start(0))?;

    let header = file
        .read_type::<EXGeoHeader>(endian)
        .expect("Failed to read header");

    let platform = platform
        .map(|p| p.into())
        .or(Platform::from_path(&filename))
        .expect("Failed to detect platform");

    if platform != Platform::Pc && platform != Platform::Xbox && platform != Platform::Xbox360 {
        anyhow::bail!("Entity extraction is only supported for PC, Xbox and X360 (for now)")
    }

    println!("Selected platform {platform:?}");

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
        file.seek(std::io::SeekFrom::Start(e.common.address as u64))?;

        let ent = file
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
            &mut file,
            4,
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

        let gltf = dump_single_mesh_to_scene(
            &vertex_data,
            &indices,
            &strips,
            [252, 250, 240, 221].contains(&header.version),
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
            )?;
        }
    } else if ent.object_type == 0x601 {
        let vertex_offset = vertex_data.len() as u32;
        let index_offset = indices.len() as u32; // vertex_data.len() as u32;
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
            .map(|_| data.read_type::<u16>(endian).unwrap() as u32 + vertex_offset)
            .collect();

        indices.extend(&new_indices);

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
                continue;
            }

            strips.push(TriStrip {
                start_index: index_offset + index_offset_local,
                index_count: t.tricount + 2,
                texture_hash: t.texture_index as u32,
            });

            // HACK: Swap the first 2 indices to change the winding order
            // Pleasing the glTF god is no easy feat
            // TODO: Doesnt seem to work?
            // let offset = (index_offset + index_offset_local) as usize;
            // let i0 = indices[offset];
            // indices[offset] = indices[offset + 1];
            // indices[offset + 1] = i0;

            index_offset_local += t.tricount + 2;
        }
    } else {
        anyhow::bail!("Invalid obj type 0x{:x}", ent.object_type)
    }

    Ok(())
}
