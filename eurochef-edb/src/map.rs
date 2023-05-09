use binrw::binrw;
use serde::Serialize;

use crate::{
    array::{EXGeoHashArray, EXRelArray},
    common::{EXRelPtr, EXVector, EXVector2, EXVector3},
};

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(version: u32))] // TODO: Seems a bit dirty, no?
pub struct EXGeoMap {
    pub common: u32,
    pub bsp_tree: EXRelPtr<()>,                       // EXGeoBspTree, 0x4
    pub paths: EXGeoHashArray<EXGeoPath>,             // 0x8
    pub lights: EXGeoHashArray<EXGeoLight>,           // 0x10
    pub cameras: EXRelArray<EXGeoCamera>, // 0x18, structure unconfirmed (never used in GForce)
    pub isounds: EXRelArray<u16>,         // 0x20
    pub unk28: EXRelArray<()>,            // never used in GForce
    pub sounds: EXGeoHashArray<EXGeoSound>, // 0x30
    pub portals: EXRelArray<EXGeoPortal>, // EXGeoPortal, 0x38
    pub skies: EXRelArray<EXGeoSky>,      // 0x40
    pub placements: EXRelArray<EXGeoPlacement>, // 0x48
    pub placement_groups: EXRelArray<()>, // EXGeoPlacementGroup, 0x50
    pub trigger_header: EXRelPtr<EXGeoTriggerHeader>, // 0x58
    pub unk_60: [u32; 4],                 // 0x5c

    // TODO(cohae): Workaround for older spyro files like test_wts, need to test offset and other versions
    #[brw(if(version.eq(&221)))]
    _unk6c_pad: [u32; 2],

    pub bounds_box: [EXVector3; 2], // 0x6c

    #[serde(skip)]
    num_zones: u32, // 0x84

    #[br(args {
        count: num_zones as usize,
        inner: (version,)
    })]
    pub zones: Vec<EXGeoMapZone>, // 0x88
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[br(import(version: u32))]
// TODO(cohae): Struct is not accurate below version 248 yet
pub struct EXGeoMapZone {
    pub entity_refptr: u32,              // 0x0
    pub identifier: EXRelPtr<()>,        // 0x4
    pub light_array: EXGeoHashArray<()>, // 0x8
    pub sound_array: EXGeoHashArray<()>, // 0x10
    pub unk18: EXRelArray<()>,           // ???, 0x18 (u16?)
    pub unk20: EXRelArray<()>,           // ???, 0x20
    pub unk28: EXRelPtr<()>,             // PlacementInfo?, 0x28
    pub unk2c: EXRelPtr<()>,             // ???, 0x2c
    pub hash_ref: u32,                   // 0x30
    pub section: u32,                    // 0x34
    pub unk38: [u32; 12],                // 0x38
    pub bounds_box: [EXVector3; 2],      // 0x60
    pub unk80: u32,                      // 0x80

    // Robots has 8 less bytes
    #[br(if(!version.le(&248)))]
    pub unk84: [u32; 2], // 0x84
}

// TODO(cohae): A lot of these structures might need to be split up into separate files

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoPortalInfo {
    pub map_on: u16,
    pub map_to: u16,
    pub index: i16,
    pub portal_count: u8,
    pub flipped: u8,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoTriggerHeader {
    pub triggers: EXRelArray<EXGeoTrigHeader>,

    // pub trigger_scripts: EXRelPtr<EXGeoTrigScriptHeader, -4>,
    pub trigger_scripts: EXRelPtr<(), i32, -4>,

    // TODO(cohae): custom parser
    #[br(count = if triggers.data.len() != 0 { triggers.data.iter().map(|v| v.trigger.data.type_index).max().unwrap()+1 } else { 0 })]
    pub trigger_types: EXRelPtr<Vec<EXGeoTriggerType>>,
    // pub trigger_types: EXRelPtr<()>, // Last element is marked by a trig_type of -1

    // #[br(count = triggers.array_size)]
    pub trigger_collisions: EXRelPtr<()>,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoTriggerType {
    pub trig_type: u32,
    pub trig_subtype: u32,
    #[serde(skip)]
    pad: [u32; 2],
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoTrigScriptHeader {
    pub script_count: u32,
    pub data_ptr: EXRelPtr,

    #[br(count = script_count)]
    pub offsets: Vec<(u32, EXRelPtr)>,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoBaseDatum {
    pub hashcode: u32,
    pub flags: u16,
    pub dtype: u8,
    pub hash_index: u8,
    pub extents: [f32; 3],
    pub position: EXVector3,
    pub q: [f32; 4],
}

pub type EXGeoTriggerDatum = EXGeoBaseDatum;

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoBspNode {
    pub pos: EXVector,
    pub nodes: [i16; 2],
    #[serde(skip)]
    pad: [i32; 3], // TODO: Use binrw attribute to pad instead
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoTrigHeader {
    pub trigger: EXRelPtr<EXGeoTrigger>,
    pub link_ref: i32,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoTrigger {
    pub type_index: u16,
    pub debug: u16,
    pub game_flags: u32,     // 0x4
    pub trig_flags: u32,     // 0x8
    pub position: EXVector3, // 0xc
    pub rotation: EXVector3, // 0x18
    pub scale: EXVector3,    // 0x24
    // TODO: We should make a separate reader for this to prevent over-reads
    pub data: [u32; 32], // 0x30
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoPortal {
    pub map_a: u16,
    pub map_b: u16,
    pub flags: u16,
    #[serde(skip)]
    pad0: u16,
    #[serde(skip)]
    pad1: u32,
    pub distance: f32,
    pub portal_face: EXRelPtr<EXGeoFace>,
    pub vertices: [EXVector3; 4],
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoFace {
    pub common: u32,
    pub texture_ref: u32,
    pub vertex_count: u32,
    pub flags: u32,
    #[br(count = vertex_count)]
    #[brw(if(common.eq(&0x800)))]
    pub vertices: Vec<GeoFaceVtx>,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct GeoFaceVtx {
    pub v: EXVector,
    pub uv: EXVector2,
    pad: u32,
    pub color: [u8; 4],
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoCamera {
    pub hashcode: u32,
    pub position: EXVector3,
    pub flags: u32,
    pub look: EXVector3,
    pub focal_length: f32,
    pub aperture_width: f32,
    pub aperture_height: f32,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoSky {
    pub hashcode: u32,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoSound {
    pub hashcode: u32,
    pub position: EXVector3,
    pub flags: u32,
    pub sound_ref: u32,
    pub color: [u8; 4],
    pub volume: u8,
    pub fade_in: u8,
    pub fade_out: u8,
    pub tracking_type: u8,
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub base_map_on: u32,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoPlacement {
    pub hashcode: u32,
    pub position: EXVector3,
    pub flags: u32,
    pub rotation: EXVector3,
    pub scale: EXVector3,
    pub engine_flags: u16,
    pub map_on: u16,
    pub object_ref: u32,
    pub light_set: u16,
    pub group: i16,
    pub unk: u32,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoLight {
    pub hashcode: u32,
    pub position: EXVector3,
    pub flags: u32,
    pub beam: EXVector3,
    pub ltype: u16,
    pub beam_angle: u16,
    pub colour: [u8; 4],
    pub radius: f32,
    pub max_effect_fraction: f32,
    pub unk: u32,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoPath {
    pub hashcode: u32,
    pub position: EXVector3, // 0x4
    pub flags: u32,          // 0x10
    pub unk14: u32,
    pub unk18: u32,
    pub ptype: u16, // 0x1c
    #[serde(skip)]
    pad0: u16,
    pub bounds_box: [EXVector; 2], // 0x20

    pub links: EXRelArray<EXGeoPathLink>, // EXGeoPathLink, 0x40
    pub nodes: EXRelArray<EXGeoPathNode>, // EXGeoPathNode, 0x48
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoPathLink {
    pub node_a: u16,
    pub node_b: u16,
    pub flags: u32,
    pub length: f32,
    pub value: [u32; 4],
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoPathNode {
    pub position: EXVector3,
    pub size: EXVector2,
    pub value: [u16; 4],
    pub flags: u32,
    pub distance: f32,
    // #[br(count = num_links)]
    pub path_links_table: EXRelPtr<(), i16>,
    pub num_links: u16,
}
