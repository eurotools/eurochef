#![allow(non_camel_case_types)]
use binrw::binrw;
use serde::Serialize;

use crate::common::{EXRelPtr, EXVector};

#[binrw]
#[derive(Debug, Serialize)]
#[brw(import(version: u32))]
// TODO: Format is slightly different on versions 248 and below
pub struct EXGeoBaseEntity {
    #[brw(assert(object_type.eq(&0x601) || object_type.eq(&0x603)))]
    pub object_type: u32, // 0x0
    pub flags: u32,       // 0x4
    pub sort_value: u16,  // 0x8
    pub render_order: u8, // 0xa
    #[serde(skip)]
    _pad0: u8, // 0xb
    pub surface_area: f32, // 0xc
    pub bounds_box: [EXVector; 2], // 0x10
    _unk30: [u32; 8],     // 0x30
    pub gdi_count: u16,   // 0x50
    pub gdi_index: u16,   // 0x52

    #[brw(if(object_type.eq(&0x601)))]
    #[brw(args(version))]
    pub normal_entity: Option<EXGeoEntity>,
    #[brw(if(object_type.eq(&0x603)))]
    #[brw(args(version))]
    pub split_entity: Option<EXGeoSplitEntity>,
}

#[binrw]
#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
#[brw(import(_version: u32))]
pub struct EXGeoSplitEntity {
    // pub base: EXGeoBaseEntity, // 0x0
    #[brw(assert(entity_count.le(&32)))]
    pub entity_count: u32, // 0x54
    _unk58: u32,

    // Make sure all sub entities are normal entities
    #[brw(assert(entities.iter().find(|e| e.data.object_type != 0x601).is_none()))]
    #[br(count = entity_count)]
    pub entities: Vec<EXRelPtr<EXGeoBaseEntity>>, // 0x5c
}

#[binrw]
#[derive(Debug, Serialize)]
pub struct EXGeoEntity_TextureList {
    #[serde(skip)]
    pub texture_count: u16,

    #[br(count = texture_count)]
    pub textures: Vec<u16>,
}

#[binrw]
#[derive(Debug, Serialize)]
pub struct EXGeoEntity_VtxData {}
