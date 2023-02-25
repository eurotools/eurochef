use binrw::binrw;

use crate::{
    array::EXGeoCommonArrayElement, common::EXRelPtr, structure_size_tests, versions::Platform,
};

#[binrw]
#[derive(Debug)]
pub struct EXGeoTextureHeader {
    pub common: EXGeoCommonArrayElement,
    pub width: u16,
    pub height: u16,
    pub game_flags: u32,
    pub flags: u32,
}

#[binrw]
#[derive(Debug)]
#[brw(import(version: u32, platform: Platform))] // TODO: Seems a bit dirty, no?
pub struct EXGeoTexture {
    pub width: u16,        // 0x0
    pub height: u16,       // 0x2
    pub depth: u16,        // 0x4
    pub game_flags: u16,   // 0x6
    pub scroll_u: i16,     // 0x8
    pub scroll_v: i16,     // 0xa
    pub frame_count: u8,   // 0xc
    pub image_count: u8,   // 0xd
    pub frame_rate: u8,    // 0xe
    _pad0: u8,             // 0xf
    pub values_used: u8,   // 0x10
    pub regions_count: u8, // 0x11
    pub mip_count: u8,     // 0x12
    pub format: u8,        // 0x13
    pub unk_14: u32,       // 0x14
    pub color: u32,        // 0x18

    _pad1: [u8; 8], // 0x1c

    #[brw(if(platform != Platform::GameCube && platform != Platform::Wii))]
    _pad2: u32, // 0x24

    /// Newer games calculate data size from other parameters.
    /// For general usage it is not recommended to rely on this field for data size.
    #[brw(if(version <= 252 && (version != 240 || (platform == Platform::GameCube || platform == Platform::Wii))))]
    pub data_size: Option<u32>,

    #[br(count = frame_count)]
    pub frame_offsets: Vec<EXRelPtr>, // 0x28
}

structure_size_tests!(EXGeoTextureHeader = 28);
