use binrw::binrw;

use crate::{array::EXGeoCommonArrayElement, common::EXRelPtr, versions::Platform};

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
#[brw(import(version: u32, platform: Platform))]
pub struct EXGeoTexture {
    #[brw(if(version <= 205))]
    unk0: u32,

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

    // TODO(cohae): G-Force only for now, check other games
    /// If set, contains the hashcode of another file
    /// The first frame offset will be replaced with a texture hashcode from that file
    #[br(map = |x: i32| if x == -1 { None } else { Some(x as u32) } )]
    #[br(if(version == 259))]
    pub external_file: Option<u32>, // 0x1c

    #[br(if(version != 259 && version != 163 && version != 174))]
    _unk1: u32, // 0x1c

    _unk2: EXRelPtr, // 0x20

    #[brw(if(platform != Platform::GameCube))]
    _unk3: u32, // 0x24

    #[brw(if(platform == Platform::Ps2))]
    pub clut_offset: Option<EXRelPtr>,

    /// Newer games calculate data size from other parameters.
    /// For general usage it is not recommended to rely on this field exlusively for data size.
    // #[brw(if((version <= 252 && version != 221 && version != 236 && version != 240 && version != 248 && version != 213 && version != 205 && version != 163 && version != 174) || (platform == Platform::GameCube || platform == Platform::Wii || platform == Platform::Xbox360)))]
    #[brw(if((version >= 252) || (platform == Platform::GameCube || platform == Platform::Wii || platform == Platform::Xbox360)))]
    pub data_size: Option<u32>,

    #[br(count = image_count)]
    pub frame_offsets: Vec<EXRelPtr>,
}
