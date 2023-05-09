use binrw::binrw;
use serde::Serialize;

use crate::{
    array::EXRelArray,
    common::{EXRelPtr, EXVector},
};

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(_version: u32))]
pub struct EXGeoBaseAnimSkin {
    pub object_type: u32,          // 0x0
    pub _unk4: u32,                // 0x4, some size
    pub _unkc: [u32; 2],           // 0xc
    pub bounds_box: [EXVector; 2], // 0x10
    pub _unk30: [u32; 4],          // 0x40
    pub _unk40: EXRelPtr<()>,
    pub _unk44: EXRelPtr<()>,
    pub _unk48: EXRelPtr<u16>, // EXItemRender_SkinAnim::BuildRemapTable
    pub _unk4c: EXRelPtr<()>,
    pub _unk50: [u32; 2],
    pub _unk58: EXRelPtr<u16>,
    pub _unk5c: EXRelPtr<()>,
    pub _unk60: EXRelArray<()>,
    pub entities: EXRelArray<EXGeoAnimSkinEntityList>, // 0x68
    pub more_entities: EXRelArray<EXGeoAnimSkinEntityList>, // 0x70, face-related entities?
    pub _unk78: EXRelArray<()>,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoAnimSkinEntityList {
    _pad0: u32,
    _pad1: u32,
    pub section_index: u32,
    pub entity_index: u32,
    pub morph_index: i32,
}
