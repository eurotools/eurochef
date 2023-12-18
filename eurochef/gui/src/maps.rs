use std::{io::Seek, sync::Arc};

use anyhow::Context;

use egui::mutex::{Mutex, RwLock};
use eurochef_edb::{
    binrw::BinReaderExt,
    edb::EdbFile,
    entity::{EXGeoEntity, EXGeoMapZoneEntity},
    map::{EXGeoBaseDatum, EXGeoMap, EXGeoMapZone, EXGeoPlacement, EXGeoTriggerEngineOptions},
    versions::Platform,
    Hashcode,
};
use eurochef_shared::IdentifiableResult;
use glam::Vec3;
use nohash_hasher::IntMap;

use crate::{
    entities::ProcessedEntityMesh,
    map_frame::MapFrame,
    render::{entity::EntityRenderer, viewer::CameraType, RenderStore},
};

pub struct MapViewerPanel {
    maps: Vec<ProcessedMap>,

    // TODO(cohae): Replace so we can do funky stuff
    frame: MapFrame,
}

#[derive(Clone)]
pub struct ProcessedMap {
    pub hashcode: u32,
    pub mapzone_entities: Vec<EXGeoMapZoneEntity>,
    pub zones: Vec<EXGeoMapZone>,
    pub skies: Vec<Hashcode>,
    pub placements: Vec<EXGeoPlacement>,
    pub triggers: Vec<ProcessedTrigger>,
    pub trigger_collisions: Vec<EXGeoBaseDatum>,
}

#[derive(Clone)]
pub struct ProcessedTrigger {
    pub link_ref: i32,

    pub ttype: u32,
    pub tsubtype: Option<u32>,

    pub debug: u16,
    pub game_flags: u32,
    pub trig_flags: u32,
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,

    pub data: Vec<Option<u32>>,
    pub links: Vec<i32>,
    pub engine_options: EXGeoTriggerEngineOptions,

    /// Every trigger that links to this one
    pub incoming_links: Vec<i32>,
}

impl MapViewerPanel {
    pub fn new(
        file: Hashcode,
        gl: Arc<glow::Context>,
        maps: Vec<ProcessedMap>,
        ref_entities: Vec<IdentifiableResult<(EXGeoEntity, ProcessedEntityMesh)>>,
        render_store: Arc<RwLock<RenderStore>>,
        platform: Platform,
        hashcodes: Arc<IntMap<u32, String>>,
        game: &str,
    ) -> Self {
        MapViewerPanel {
            frame: {
                let ef = MapFrame::new(
                    file,
                    Self::load_map_meshes(file, &gl, &maps, &ref_entities, platform),
                    gl,
                    render_store,
                    hashcodes,
                    game,
                );

                {
                    let mut e = ef.viewer.lock();
                    e.selected_camera = CameraType::Fly;
                    e.show_grid = false;
                }

                ef
            },
            maps,
        }
    }

    fn load_map_meshes(
        file: Hashcode,
        gl: &glow::Context,
        maps: &[ProcessedMap],
        ref_entities: &[IdentifiableResult<(EXGeoEntity, ProcessedEntityMesh)>],
        platform: Platform,
    ) -> Vec<(u32, Arc<Mutex<EntityRenderer>>)> {
        let mut ref_renderers = vec![];

        // FIXME(cohae): Map picking is a bit dirty at the moment
        for map in maps.iter() {
            for v in &map.mapzone_entities {
                if let Some(Ok((_, e))) = &ref_entities
                    .iter()
                    .find(|ir| ir.hashcode == v.entity_refptr)
                    .map(|v| v.data.as_ref())
                {
                    let r = Arc::new(Mutex::new(EntityRenderer::new(file, platform)));
                    unsafe {
                        r.lock().load_mesh(gl, e);
                    }
                    ref_renderers.push((map.hashcode, r));
                } else {
                    error!(
                        "Couldn't find ref entity #{} for mapzone entity!",
                        v.entity_refptr
                    );
                }
            }
        }

        ref_renderers
    }

    pub fn show(&mut self, context: &egui::Context, ui: &mut egui::Ui) -> anyhow::Result<()> {
        self.frame.show(ui, context, &self.maps)
    }
}

pub fn read_from_file(edb: &mut EdbFile) -> Vec<ProcessedMap> {
    let header = edb.header.clone();

    let mut maps = vec![];
    for m in header.map_list.iter() {
        edb.seek(std::io::SeekFrom::Start(m.address as u64))
            .unwrap();

        let xmap = edb
            .read_type_args::<EXGeoMap>(edb.endian, (header.version,))
            .context("Failed to read map")
            .unwrap();

        let mut map = ProcessedMap {
            hashcode: m.hashcode,
            mapzone_entities: vec![],
            placements: xmap.placements.data().clone(),
            triggers: vec![],
            trigger_collisions: xmap.trigger_header.trigger_collisions.0.clone(),
            skies: xmap.skies.iter().map(|s| s.hashcode).collect(),
            zones: vec![],
        };

        for z in &xmap.zones {
            let entity_offset = header.refpointer_list[z.entity_refptr as usize].address;
            edb.seek(std::io::SeekFrom::Start(entity_offset as u64))
                .context("Mapzone refptr pointer to a non-entity object!")
                .unwrap();

            let ent = edb
                .read_type_args::<EXGeoEntity>(edb.endian, (header.version, edb.platform))
                .unwrap();

            if let EXGeoEntity::MapZone(mapzone) = ent {
                map.mapzone_entities.push(mapzone);
            } else {
                error!("Refptr entity does not have a mapzone entity!");
                // Result::<()>::Err(anyhow::anyhow!(
                //     "Refptr entity does not have a mapzone entity!"
                // ))
                // .unwrap();
            }
        }

        map.zones = xmap.zones;

        for t in xmap.trigger_header.triggers.iter() {
            let trig = &t.trigger;
            let (ttype, tsubtype) = {
                let t = &xmap.trigger_header.trigger_types[trig.type_index as usize];

                (t.trig_type, t.trig_subtype)
            };

            let trigger = ProcessedTrigger {
                link_ref: t.link_ref,
                ttype,
                tsubtype: if tsubtype != 0 && tsubtype != 0x42000001 {
                    Some(tsubtype)
                } else {
                    None
                },
                debug: trig.debug,
                game_flags: trig.game_flags,
                trig_flags: trig.trig_flags,
                position: trig.position.into(),
                rotation: trig.rotation.into(),
                scale: trig.scale.into(),
                engine_options: trig.engine_options.clone(),
                data: trig.data.to_vec(),
                links: trig.links.to_vec(),
                incoming_links: vec![],
            };

            map.triggers.push(trigger);
        }

        for i in 0..map.triggers.len() {
            for ei in 0..map.triggers.len() {
                if i == ei {
                    continue;
                }

                if map.triggers[ei].links.iter().any(|v| *v == i as i32) {
                    map.triggers[i].incoming_links.push(ei as i32);
                }
            }
        }

        maps.push(map);
    }

    maps
}
