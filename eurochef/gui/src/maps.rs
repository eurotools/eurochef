use std::{
    io::{Read, Seek},
    sync::Arc,
};

use anyhow::{Context, Result};

use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    entity::{EXGeoEntity, EXGeoMapZoneEntity},
    header::EXGeoHeader,
    map::{EXGeoMap, EXGeoPlacement},
    versions::Platform,
};
use eurochef_shared::{
    maps::{parse_trigger_data, UXGeoTrigger},
    textures::UXGeoTexture,
};

use crate::{
    entities::{EntityListPanel, ProcessedEntityMesh},
    entity_frame::RenderableTexture,
    map_frame::MapFrame,
    render::viewer::CameraType,
};

pub struct MapViewerPanel {
    _gl: Arc<glow::Context>,

    map: ProcessedMap,
    _entities: Vec<(u32, EXGeoEntity, ProcessedEntityMesh)>,
    _ref_entities: Vec<(u32, EXGeoEntity, ProcessedEntityMesh)>,
    _textures: Vec<RenderableTexture>,

    // TODO(cohae): Replace so we can do funky stuff
    frame: MapFrame,
}

#[derive(Clone)]
pub struct ProcessedMap {
    pub mapzone_entities: Vec<EXGeoMapZoneEntity>,
    pub placements: Vec<EXGeoPlacement>,
}

impl MapViewerPanel {
    pub fn new(
        _ctx: &egui::Context,
        gl: Arc<glow::Context>,
        map: ProcessedMap,
        entities: Vec<(u32, EXGeoEntity, ProcessedEntityMesh)>,
        ref_entities: Vec<(u32, EXGeoEntity, ProcessedEntityMesh)>,
        textures: &[UXGeoTexture],
    ) -> Self {
        let textures = EntityListPanel::load_textures(&gl, textures);
        MapViewerPanel {
            frame: Self::load_map_meshes(&gl, &map, &ref_entities, &entities, &textures),
            _textures: textures,
            _gl: gl,
            map,
            _entities: entities,
            _ref_entities: ref_entities,
        }
    }

    fn load_map_meshes(
        gl: &glow::Context,
        map: &ProcessedMap,
        ref_entities: &Vec<(u32, EXGeoEntity, ProcessedEntityMesh)>,
        entities: &Vec<(u32, EXGeoEntity, ProcessedEntityMesh)>,
        textures: &[RenderableTexture],
    ) -> MapFrame {
        let mut map_entities = vec![];

        for v in &map.mapzone_entities {
            if let Some((_, _, e)) = &ref_entities.iter().find(|(i, _, _)| *i == v.entity_refptr) {
                map_entities.push(e);
            } else {
                error!(
                    "Couldn't find ref entity #{} for mapzone entitiy!",
                    v.entity_refptr
                );
            }
        }

        let ef = MapFrame::new(gl, &map_entities, textures, entities);
        ef.viewer
            .lock()
            .map(|mut v| {
                v.selected_camera = CameraType::Fly;
                v.show_grid = false;
            })
            .unwrap();

        ef
    }

    pub fn show(&mut self, _context: &egui::Context, ui: &mut egui::Ui) {
        self.frame.show(ui, &self.map)
    }
}

// TODO(cohae): EdbFile struct so we dont have to read endianness separately
pub fn read_from_file<R: Read + Seek>(reader: &mut R, platform: Platform) -> ProcessedMap {
    reader.seek(std::io::SeekFrom::Start(0)).ok();
    let endian = if reader.read_ne::<u8>().unwrap() == 0x47 {
        Endian::Big
    } else {
        Endian::Little
    };
    reader.seek(std::io::SeekFrom::Start(0)).unwrap();

    let header = reader
        .read_type::<EXGeoHeader>(endian)
        .expect("Failed to read header");

    let mut xmap: Option<EXGeoMap> = None;
    for m in header.map_list.iter() {
        reader
            .seek(std::io::SeekFrom::Start(m.address as u64))
            .unwrap();

        let nmap = reader
            .read_type_args::<EXGeoMap>(endian, (header.version,))
            .context("Failed to read map")
            .unwrap();

        if let Some(oxmap) = &xmap {
            if nmap.placements.len() > oxmap.placements.len() {
                xmap = Some(nmap);
            }
        } else {
            xmap = Some(nmap)
        }
    }

    let xmap = xmap.unwrap();

    let mut map = ProcessedMap {
        mapzone_entities: vec![],
        placements: xmap.placements.data().clone(),
    };

    for z in &xmap.zones {
        let entity_offset = header.refpointer_list[z.entity_refptr as usize].address;
        reader
            .seek(std::io::SeekFrom::Start(entity_offset as u64))
            .context("Mapzone refptr pointer to a non-entity object!")
            .unwrap();

        let ent = reader
            .read_type_args::<EXGeoEntity>(endian, (header.version, platform))
            .unwrap();

        if let EXGeoEntity::MapZone(mapzone) = ent {
            map.mapzone_entities.push(mapzone);
        } else {
            Result::<()>::Err(anyhow::anyhow!(
                "Refptr entity does not have a mapzone entity!"
            ))
            .unwrap();
        }
    }

    for (i, t) in xmap.trigger_header.triggers.iter().enumerate() {
        let trig = &t.trigger;
        let (ttype, tsubtype) = {
            let t = &xmap.trigger_header.trigger_types[trig.type_index as usize];

            (t.trig_type, t.trig_subtype)
        };

        // FIXME: This requires triggers to be sorted. This is the case in official EDB files but it is not a requirement
        let trigdata_size = {
            if (i + 1) == xmap.trigger_header.triggers.len() {
                32
            } else {
                let current_addr = trig.offset_absolute();
                let next_addr = xmap.trigger_header.triggers.data()[i + 1]
                    .trigger
                    .offset_absolute();

                ((next_addr - current_addr - 0x30) / 4) as usize
            }
        };

        let (data, links, extra_data) =
            parse_trigger_data(header.version, trig.trig_flags, &trig.data[..trigdata_size]);
        let _trigger = UXGeoTrigger {
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
            raw_data: trig.data[..trigdata_size].to_vec(),
            extra_data,
            data,
            links,
        };
    }

    map
}
