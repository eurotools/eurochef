use std::{
    fs::File,
    io::{BufReader, Seek, Write},
    path::Path,
};

use anyhow::Context;
use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    entity::{EXGeoEntity, EXGeoMapZoneEntity},
    header::EXGeoHeader,
    map::{EXGeoLight, EXGeoMap, EXGeoPath, EXGeoPlacement},
    versions::Platform,
};

use eurochef_shared::maps::{TriggerInformation, UXGeoTrigger};
use serde::Serialize;

use crate::PlatformArg;

pub fn execute_command(
    filename: String,
    platform: Option<PlatformArg>,
    output_folder: Option<String>,
    trigger_defs_file: Option<String>,
) -> anyhow::Result<()> {
    let output_folder = output_folder.unwrap_or(format!(
        "./maps/{}/",
        Path::new(&filename)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    ));

    let trigger_typemap = if let Some(path) = trigger_defs_file {
        Some(load_trigger_types(path)?)
    } else {
        None
    };

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

    if header.map_list.len() == 0 {
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

    let platform = platform
        .map(|p| p.into())
        .or(Platform::from_path(&filename))
        .expect("Failed to detect platform");

    let output_folder = Path::new(&output_folder);
    std::fs::create_dir_all(output_folder)?;

    for m in &header.map_list {
        file.seek(std::io::SeekFrom::Start(m.address as u64))?;

        let map = file
            .read_type_args::<EXGeoMap>(endian, (header.version,))
            .context("Failed to read map")?;

        let mut export = EurochefMapExport {
            paths: map.paths.data().clone(),
            placements: map.placements.data().clone(),
            lights: map.lights.data().clone(),
            mapzone_entities: vec![],
            triggers: vec![],
        };

        for z in &map.zones {
            let entity_offset = header.refpointer_list[z.entity_refptr as usize].address;
            file.seek(std::io::SeekFrom::Start(entity_offset as u64))
                .context("Mapzone refptr pointer to a non-entity object!")?;

            let ent = file.read_type_args::<EXGeoEntity>(endian, (header.version, platform))?;

            if let EXGeoEntity::MapZone(mapzone) = ent {
                export.mapzone_entities.push(mapzone);
            } else {
                anyhow::bail!("Refptr entity does not have a mapzone entity!");
            }
        }

        for t in map.trigger_header.triggers.iter() {
            let trig = &t.trigger;
            let (ttype, tsubtype) = {
                let t = &map.trigger_header.trigger_types[trig.type_index as usize];

                (t.trig_type, t.trig_subtype)
            };

            let mut trigger = UXGeoTrigger {
                link_ref: t.link_ref,
                ttype: format!("Trig_{ttype}"),
                tsubtype: if tsubtype != 0 && tsubtype != 0x42000001 {
                    Some(format!("TrigSub_{tsubtype}"))
                } else {
                    None
                },
                debug: trig.debug,
                game_flags: trig.game_flags,
                trig_flags: trig.trig_flags,
                position: trig.position,
                rotation: trig.rotation,
                scale: trig.scale,
                extra_data: trig
                    .engine_data
                    .iter()
                    .map(|v| v.unwrap_or(u32::MAX))
                    .collect(),
                data: trig.data.to_vec(),
                links: trig.links.to_vec(),
            };

            if let Some(ref typemap) = trigger_typemap {
                match typemap.triggers.get(&ttype) {
                    Some(t) => trigger.ttype = t.name.clone(),
                    None => warn!("Couldn't find trigger type {ttype}"),
                }

                if trigger.tsubtype.is_some() {
                    match typemap.triggers.get(&tsubtype) {
                        Some(t) => trigger.tsubtype = Some(t.name.clone()),
                        None => warn!("Couldn't find trigger subtype {tsubtype}"),
                    }
                }
            }

            export.triggers.push(trigger);
        }

        let mut outfile = File::create(output_folder.join(format!("{:x}.ecm", m.hashcode)))?;

        let json_string =
            gltf::json::serialize::to_string(&export).context("ECM serialization error")?;

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
    pub triggers: Vec<UXGeoTrigger>,
}

fn load_trigger_types<P: AsRef<Path>>(path: P) -> anyhow::Result<TriggerInformation> {
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);
    Ok(serde_yaml::from_reader(&mut reader)?)
}
