#![allow(non_camel_case_types)]
use binrw::binrw;
use serde::Serialize;

use crate::{
    common::{EXRelPtr, EXVector},
    versions::Platform,
};

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(version: u32))]
// TODO: Format is slightly different on versions 248 and below
pub struct EXGeoBaseEntity {
    #[brw(assert([0x601, 0x603, 0x606, 0x607, 0x608].contains(&object_type)))]
    pub object_type: u32, // 0x0
    pub flags: u32,       // 0x4
    pub sort_value: u16,  // 0x8
    pub render_order: u8, // 0xa
    #[serde(skip)]
    _pad0: u8, // 0xb
    pub surface_area: f32, // 0xc
    pub bounds_box: [EXVector; 2], // 0x10
    _unk30: [u32; 4],     // 0x30
    #[brw(if(version > 221))]
    _unk40: [u32; 4],
    pub gdi_count: u16, // 0x50
    pub gdi_index: u16, // 0x52

    #[brw(if(object_type.eq(&0x601)))]
    #[brw(args(version))]
    pub normal_entity: Option<EXGeoEntity>,
    #[brw(if(object_type.eq(&0x603)))]
    #[brw(args(version))]
    pub split_entity: Option<EXGeoSplitEntity>,
    #[brw(if(object_type.eq(&0x608)))]
    pub mapzone_entity: Option<EXGeoMapZoneEntity>,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(_version: u32))]
pub struct EXGeoEntity {
    // pub base: EXGeoBaseEntity,                       // 0x0
    pub texture_list: EXRelPtr<EXGeoEntity_TextureList>, // 0x54
    pub tristrip_data: EXRelPtr,                         // 0x58
    pub vertex_data: EXRelPtr,                           // 0x5c
    pub _unk60: EXRelPtr,                                // 0x60
    pub _unk64: EXRelPtr,                                // 0x64
    pub _unk68: u32,                                     // 0x68
    pub index_data: EXRelPtr,                            // 0x6c

    pub _unk70: u32,         // 0x70
    pub tristrip_count: u32, // 0x74
    pub vertex_count: u32,   // 0x78
    pub _unk7c: u32,         // 0x7c
    pub index_count: u32,    // 0x80
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoMapZoneEntity {
    // pub base: EXGeoBaseEntity,                       // 0x0
    pub _unk54: u32,        // 0x54
    pub entity_refptr: u32, // 0x58
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(version: u32))]
pub struct EXGeoSplitEntity {
    // TODO(cohae): Older games have different limits, how do we handle that when writing files?
    #[brw(assert(entity_count.le(&512)))]
    pub entity_count: u32, // 0x54
    _unk58: u32,

    #[br(count = entity_count, args { inner: (version,) })]
    pub entities: Vec<EXRelPtr<EXGeoBaseEntity>>, // 0x5c
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoEntity_TextureList {
    #[serde(skip)]
    pub texture_count: u16,

    #[br(count = texture_count)]
    pub textures: Vec<u16>,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(version: u32, platform: Platform))]
pub struct EXGeoEntity_TriStrip {
    pub tricount: u32,
    pub texture_index: i32,
    pub min_index: u16,
    pub num_indices: u16,
    pub flags: u16,
    pub trans_type: u16,
    #[brw(if(version <= 252 && version != 248 || platform == Platform::Xbox360))]
    _unk10: u32,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoEntity_VtxData {}
