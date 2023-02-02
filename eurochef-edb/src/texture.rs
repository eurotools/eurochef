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
pub struct EXGeoTexture {
    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub game_flags: u16,
    pub scroll_u: i16,
    pub scroll_v: i16,
    pub frame_count: u8,
    pub image_count: u8,
    pub frame_rate: u8,
    _pad1: u8,
    pub values_used: u8,
    pub regions_count: u8,
    pub mip_count: u8,
    pub format: u8,
    pub unk_14: u32,
    pub color: u32,

    pad: [u8; 12],

    #[br(count = frame_count)]
    pub frame_offsets: Vec<EXRelPtr>,
}

#[binrw]
#[brw(repr(u8))]
#[derive(Debug)]
// TODO: This is might get quite big the more platforms we add
pub enum EXTexFmt {
    // RGB formats
    ARGB4,
    ARGB8,
    RGBA8,
    RGB565,
    ARGB1555,
    RGBA5551,
    R5G5B5A3,

    // Intensity/Alpha formats
    I4,
    I8,
    /// Technically the same as I8, just used as alpha map instead of greyscale
    A8,
    IA4,
    IA8,

    // TODO: Paletted formats require extra data for their palettes. When texture data is read, this needs to be read as well.
    // Paletted formats
    P16x16,  // 16 * RGBA1555
    P16x32,  // 16 * RGBA8
    P256x16, // 32 * RGBA1555
    P256x32, // 32 * RGBA8

    // Block-based (formats BCn)
    Dxt1,
    Dxt1Alpha,
    Dxt2,
    Dxt3,
    Dxt4,
    Dxt5,
    Cmpr,
}

impl EXTexFmt {
    // TODO: Error handling instead of panic
    pub fn from_platform(fmt: u8, platform: Platform) -> Self {
        match platform {
            Platform::Pc => match fmt {
                0 => Self::RGB565,
                1 => Self::ARGB1555,
                2 => Self::Dxt1,
                3 => Self::Dxt1Alpha,
                4 => Self::Dxt2,
                5 => Self::ARGB4,
                6 => Self::ARGB8,
                7 => Self::Dxt3,
                8 => Self::Dxt4,
                9 => Self::Dxt5,
                _ => panic!("Invalid PC texture format 0x{fmt:x}"),
            },
            Platform::GameCube | Platform::Wii => match fmt {
                0 => Self::I4,
                1 => Self::I8,
                2 => Self::A8,
                3 => Self::IA4,
                4 => Self::IA8,
                5 => Self::RGB565,
                6 => Self::R5G5B5A3,
                7 => Self::RGBA8, // FIXME: This is RGBA8
                8 => Self::Cmpr,
                _ => panic!("Invalid GC/WII texture format 0x{fmt:x}"),
            },
            Platform::Ps2 => match fmt {
                0 => Self::P16x16,
                1 => Self::P16x32,
                2 => Self::P256x16,
                3 => Self::P256x32,
                4 => Self::RGBA5551, // TODO: Check endianness, its most likely wrong
                5 => Self::RGBA8,    // TODO: Check endianness, its most likely wrong
                _ => panic!("Invalid PS2 texture format 0x{fmt:x}"),
            },
            _ => panic!("Couldn't get texture format {fmt} for platform {platform:?}"),
        }
    }

    // TODO: to_platform()

    pub fn bpp(&self) -> usize {
        match self {
            Self::RGB565 | Self::ARGB1555 | Self::R5G5B5A3 | Self::RGBA5551 => 16,
            Self::ARGB4 => 16,
            Self::ARGB8 | Self::RGBA8 => 32,

            Self::I4 => 4,
            Self::I8 | Self::A8 => 8,
            Self::IA4 => 8,
            Self::IA8 => 16,

            Self::P16x16 | Self::P256x16 => 16,
            Self::P16x32 | Self::P256x32 => 32,

            Self::Dxt1 | Self::Dxt1Alpha | Self::Cmpr => 4,
            Self::Dxt2 => 8,
            Self::Dxt3 => 8,
            Self::Dxt4 => 16,
            Self::Dxt5 => 32,
        }
    }

    pub fn calculate_image_size(&self, width: u16, height: u16, depth: u16, mip: u32) -> usize {
        (((width as usize >> mip).max(1)
            * (height as usize >> mip).max(1)
            * (depth as usize >> mip).max(1))
            * self.bpp()
            + 7)
            / 8
    }
}

structure_size_tests!(EXGeoTextureHeader = 28, EXGeoTexture = 64);
