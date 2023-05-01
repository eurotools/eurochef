use std::{
    fs::File,
    io::{Seek, Write},
    path::Path,
};

use anyhow::Context;
use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    entity::{EXGeoBaseEntity, EXGeoMapZoneEntity},
    header::EXGeoHeader,
    map::{EXGeoLight, EXGeoMap, EXGeoPath, EXGeoPlacement},
};

use serde::Serialize;

use crate::PlatformArg;

pub fn execute_command(
    filename: String,
    platform: Option<PlatformArg>,
    output_folder: Option<String>,
) -> anyhow::Result<()> {
    let output_folder = output_folder.unwrap_or(format!(
        "./maps/{}/",
        Path::new(&filename)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    ));

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

    if header.map_list.array_size == 0 {
        warn!("File does not contain any maps!");
        return Ok(());
    }

    // * Almost as hacky as calling eurochef through a subprocess
    crate::edb::entities::execute_command(
        filename.clone(),
        platform.clone(),
        Some(output_folder.clone()),
        false,
        true,
    )?;

    let output_folder = Path::new(&output_folder);
    std::fs::create_dir_all(output_folder)?;

    for m in &header.map_list {
        file.seek(std::io::SeekFrom::Start(m.address as u64))?;

        let map = file
            .read_type_args::<EXGeoMap>(endian, (header.version,))
            .context("Failed to read basetexture")?;

        let mut mapzone_entities = vec![];
        for z in &map.zones {
            let entity_offset = header.refpointer_list.data[z.entity_refptr as usize].address;
            file.seek(std::io::SeekFrom::Start(entity_offset as u64))
                .context("Mapzone refptr pointer to a non-entity object!")?;

            let ent = file.read_type_args::<EXGeoBaseEntity>(endian, (header.version,))?;
            if ent.mapzone_entity.is_none() {
                anyhow::bail!("Refptr entity does not have a mapzone entity!");
            }

            mapzone_entities.push(ent.mapzone_entity.unwrap());
        }

        let root = EurochefMapExport {
            paths: map.paths.data,
            placements: map.placements.data,
            lights: map.lights.data,
            mapzone_entities,
        };

        let mut outfile = File::create(output_folder.join(format!("{:x}.ecm", m.hashcode)))?;

        let json_string =
            gltf::json::serialize::to_string(&root).context("ECM serialization error")?;

        outfile.write_all(json_string.as_bytes())?;
    }

    info!("Successfully extracted maps!");

    Ok(())
}

#[derive(Serialize)]
pub struct EurochefMapExport {
    pub paths: Vec<EXGeoPath>,
    pub placements: Vec<EXGeoPlacement>,
    pub lights: Vec<EXGeoLight>,
    pub mapzone_entities: Vec<EXGeoMapZoneEntity>,
}
