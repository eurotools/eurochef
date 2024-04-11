use binrw::binrw;

use crate::{array::EXGeoCommonArrayElement, common::EXRelPtr, versions::Platform};

#[binrw]
#[derive(Debug, Clone)]
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
    pub color: [u8; 4],    // 0x18

    // TODO(cohae): Might apply to predator as well
    /// If set, contains the hashcode of another file
    /// The first frame offset will be replaced with a texture hashcode from that file
    #[br(map = |x: i32| if x == -1 { None } else { Some(x as u32) } )]
    #[br(if(version >= 250))]
    pub external_file: Option<u32>, // 0x1c

    animseq_data: EXRelPtr<(), i16>, // 0x1c, OFFSET.W ANIMSEQDATA
    value_data: EXRelPtr<(), i16>,   // 0x1e, OFFSET.W VALUEDATA
    #[br(if(version > 163))]
    fur_data: Option<EXRelPtr<(), i16>>, // 0x20, OFFSET.W FURDATA
    #[br(if(version > 163))]
    region_data: Option<EXRelPtr<(), i16>>, // 0x22, OFFSET.W REGIONDATA

    #[brw(if(platform == Platform::Ps2 && version != 248 && version != 177 && version != 168))]
    // #[brw(if(platform == Platform::Ps2 && (version <= 163 || version == 213)))]
    _unk2: u32, // 0x24

    // ! FIXME(cohae): Robots hack, this is the same as the above field, check if this works on other platforms and surrounding versions
    #[brw(if(version == 248))]
    _unk2_rbts: u32,

    #[brw(if(platform == Platform::Ps2))]
    pub clut_offset: Option<EXRelPtr>,

    /// Certain platforms such as PC and PS2 calculate data size from other parameters.
    /// Some games (e.g. Chaos Bleeds on Xbox) also seem to do this.
    /// For general usage it is not recommended to rely on this field exlusively for data size.
    #[brw(if(platform != Platform::Pc && platform != Platform::Ps2 && !(version == 170 && platform == Platform::Xbox)))]
    pub data_size: Option<u32>,

    #[br(count = image_count)]
    pub frame_offsets: Vec<EXRelPtr>,
}
