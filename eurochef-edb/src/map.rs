use binrw::{binrw, BinRead, BinReaderExt, BinResult};
use serde::Serialize;

use crate::{
    array::{EXGeoHashArray, EXRelArray},
    common::{EXRelPtr, EXVector, EXVector2, EXVector3},
};

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(version: u32))] // TODO: Seems a bit dirty, no?
pub struct EXGeoMap {
    #[brw(assert(common.eq(&0x500)))]
    pub common: u32,
    pub bsp_tree: EXRelPtr<EXGeoBspTree>,   // EXGeoBspTree, 0x4
    pub paths: EXGeoHashArray<EXGeoPath>,   // 0x8
    pub lights: EXGeoHashArray<EXGeoLight>, // 0x10
    pub cameras: EXRelArray<()>, // EXGeoCamera, 0x18, structure unconfirmed (never used in GForce)
    pub isounds: EXRelArray<u16>, // 0x20
    pub unk28: EXRelArray<()>,   // never used in GForce
    pub sounds: EXGeoHashArray<EXGeoSound>, // 0x30
    #[brw(if(version.eq(&177) || version.eq(&213) || version.eq(&221)))]
    pub unk34: EXGeoHashArray<()>,
    pub portals: EXRelArray<EXGeoPortal>, // EXGeoPortal, 0x38
    pub skies: EXRelArray<EXGeoSky>,      // 0x40
    pub placements: EXRelArray<EXGeoPlacement>, // 0x48
    pub placement_groups: EXRelArray<()>, // EXGeoPlacementGroup, 0x50
    pub trigger_header: EXRelPtr<EXGeoTriggerHeader>, // 0x58
    pub unk_60: [u32; 4],                 // 0x5c

    pub bounds_box: [EXVector3; 2], // 0x6c

    #[serde(skip)]
    num_zones: u32, // 0x84

    #[brw(if(version.eq(&205)))]
    _unk_zonepad: [u32; 6],

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
    pub entity_refptr: u32,       // 0x0
    pub identifier: EXRelPtr<()>, // 0x4
    // TODO(cohae): Inaccurate big time
    pub light_array: EXGeoHashArray<()>, // 0x8
    pub sound_array: EXGeoHashArray<()>, // 0x10
    #[br(if(version.ne(&205)))]
    pub unk18: Option<EXRelArray<()>>, // ???, 0x18 (u16?)
    #[br(if(version.ne(&205)))]
    pub unk20: Option<EXRelArray<()>>, // ???, 0x20
    #[br(if(version.ne(&205)))]
    pub unk28: Option<EXRelPtr<()>>, // PlacementInfo?, 0x28
    pub unk2c: EXRelPtr<()>,             // ???, 0x2c
    pub hash_ref: u32,                   // 0x30
    pub section: u32,                    // 0x34
    pub unk38: [u32; 10],                // 0x38
    #[br(if(version.ne(&213) && version.ne(&221) && version.ne(&177)))]
    pub unk60: [u32; 2],
    pub bounds_box: [EXVector3; 2], // 0x60
    pub unk80: u32,                 // 0x80

    // Robots has 8 less bytes
    #[br(if(!version.le(&248) || (version.eq(&213) || version.eq(&221) || version.eq(&177))))]
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

    #[br(count = triggers.iter().map(|t| t.trigger.engine_data[2].map(|v| v+1).unwrap_or(0)).max().unwrap_or(0))]
    #[br(dbg)]
    pub trigger_scripts: EXRelPtr<Vec<(EXRelPtr, u32)>>,

    #[br(count = triggers.iter().map(|v| v.trigger.type_index+1).max().unwrap_or(0))]
    pub trigger_types: EXRelPtr<Vec<EXGeoTriggerType>>,

    pub trigger_collisions: EXRelPtr<EXGeoTriggerCollision>,
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

    #[br(count = script_count)]
    pub offsets: Vec<(EXRelPtr, u32)>,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoBaseDatum {
    pub hashref: u32,
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

    #[br(parse_with = parse_trigdata_values, args(trig_flags))]
    pub data: [Option<u32>; 16],
    #[br(parse_with = parse_trigdata_link, args(trig_flags))]
    pub links: [i32; 8],
    #[br(parse_with = parse_trigdata_engine, args(trig_flags))]
    pub engine_data: [Option<u32>; 8],
}

#[binrw::parser(reader, endian)]
fn parse_trigdata_values((trig_flags,): (u32,)) -> BinResult<[Option<u32>; 16]> {
    let mut res = [None; 16];
    for i in 0..16 {
        if (trig_flags & (1 << i)) != 0 {
            res[i] = Some(reader.read_type(endian)?);
        }
    }

    Ok(res)
}

#[binrw::parser(reader, endian)]
fn parse_trigdata_link((trig_flags,): (u32,)) -> BinResult<[i32; 8]> {
    let mut res = [-1; 8];
    for i in 16..24 {
        if (trig_flags & (1 << i)) != 0 {
            res[i - 16] = reader.read_type(endian)?;
        }
    }

    Ok(res)
}

#[binrw::parser(reader, endian)]
fn parse_trigdata_engine((trig_flags,): (u32,)) -> BinResult<[Option<u32>; 8]> {
    let mut res = [None; 8];
    for i in 24..32 {
        if (trig_flags & (1 << i)) != 0 {
            res[i - 24] = Some(reader.read_type(endian)?);
        }
    }

    Ok(res)
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

#[derive(Debug, Serialize, Clone)]
pub struct EXGeoBspTree(pub Vec<EXGeoBspNode>);

impl BinRead for EXGeoBspTree {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let mut nodes = vec![];
        let mut max_node = 1;
        let mut current_node = 0;

        loop {
            let node: EXGeoBspNode = reader.read_type(endian)?;
            max_node = max_node.max(node.nodes.iter().map(|v| v.unsigned_abs()).max().unwrap());
            nodes.push(node);

            current_node += 1;

            if current_node > max_node {
                break;
            }
        }

        Ok(Self(nodes))
    }
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoBspNode {
    pub pos: EXVector,
    pub nodes: [i16; 2],
    #[serde(skip)]
    pad: [i32; 3], // TODO: Use binrw attribute to pad instead
}

#[derive(Debug, Serialize, Clone)]
pub struct EXGeoTriggerCollision(pub Vec<EXGeoBaseDatum>);

impl BinRead for EXGeoTriggerCollision {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let mut datums = vec![];

        loop {
            let datum: EXGeoBaseDatum = reader.read_type(endian)?;
            if datum.hashref != u32::MAX {
                break;
            }

            datums.push(datum);
        }

        Ok(Self(datums))
    }
}
