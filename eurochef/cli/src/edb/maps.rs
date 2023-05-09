use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Seek, Write},
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
        false,
    )?;

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

            let ent = file.read_type_args::<EXGeoBaseEntity>(endian, (header.version,))?;
            if ent.mapzone_entity.is_none() {
                anyhow::bail!("Refptr entity does not have a mapzone entity!");
            }

            export.mapzone_entities.push(ent.mapzone_entity.unwrap());
        }

        for t in map.trigger_header.triggers.iter() {
            let trig = &t.trigger;
            let (ttype, tsubtype) = {
                let t = &map.trigger_header.trigger_types[trig.type_index as usize];

                (t.trig_type, t.trig_subtype)
            };

            let (data, links) = parse_trigger_data(header.version, trig.trig_flags, &trig.data);
            let mut trigger = EurochefMapTrigger {
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
                raw_data: trig.data,
                data,
                links,
            };

            if let Some(ref typemap) = trigger_typemap {
                match typemap.get(&ttype) {
                    Some(t) => trigger.ttype = t.clone(),
                    None => warn!("Couldn't find trigger type {ttype}"),
                }

                if trigger.tsubtype.is_some() {
                    match typemap.get(&tsubtype) {
                        Some(t) => trigger.tsubtype = Some(t.clone()),
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
    pub triggers: Vec<EurochefMapTrigger>,
}

#[derive(Serialize)]
pub struct EurochefMapTrigger {
    // TODO(cohae): Is this related to a refptr?
    pub link_ref: i32,

    pub ttype: String,
    pub tsubtype: Option<String>,

    pub debug: u16,
    pub game_flags: u32,
    pub trig_flags: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],

    pub raw_data: [u32; 32],
    pub data: Vec<u32>,
    pub links: Vec<i32>,
}

fn load_trigger_types<P: AsRef<Path>>(path: P) -> anyhow::Result<HashMap<u32, String>> {
    let mut map = HashMap::new();
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        let mut split = line.split(',');
        let hashcode = parse_int::parse(split.next().unwrap())?;
        let name = split.next().unwrap().to_string();
        map.insert(hashcode, name);
        if split.next().is_some() {
            warn!("Extra data in trigger type file!");
            continue;
        }
    }

    Ok(map)
}

fn parse_trigger_data(_version: u32, trig_flags: u32, raw_data: &[u32]) -> (Vec<u32>, Vec<i32>) {
    let mut data = vec![];
    let mut links = vec![];

    let mut flag_accessor = 1;
    let mut data_offset = 0;

    // TODO(cohae): Some older games use only 8 values instead of 16
    for _ in 0..16 {
        if (trig_flags & flag_accessor) != 0 {
            data.push(raw_data[data_offset]);
            data_offset += 1;
        } else {
            data.push(0);
        }

        flag_accessor <<= 1;
    }

    for _ in 0..8 {
        if (trig_flags & flag_accessor) != 0 {
            links.push(raw_data[data_offset] as i32);
            data_offset += 1;
        } else {
            links.push(-1);
        }

        flag_accessor <<= 1;
    }

    (data, links)
}
