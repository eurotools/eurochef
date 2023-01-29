use binrw::binrw;

use crate::{array::EXGeoCommonArrayElement, common::EXRelPtr, versions::Platform};

#[binrw]
#[brw(repr(u8))]
#[derive(Debug)]
pub enum EXTexFmt {
    R5G6B5,
    A1R5G5B5,
    Dxt1,
    Dxt1Alpha,
    Dxt2,
    A4R4G4B4,
    A8R8G8B8,
    Dxt3,
    Dxt4,
    Dxt5,
    Cmpr,
}

impl EXTexFmt {
    pub fn from_platform(fmt: u8, platform: Platform) -> Self {
        match platform {
            Platform::Pc => match fmt {
                0 => Self::R5G6B5,
                1 => Self::A1R5G5B5,
                2 => Self::Dxt1,
                3 => Self::Dxt1Alpha,
                4 => Self::Dxt2,
                5 => Self::A4R4G4B4,
                6 => Self::A8R8G8B8,
                7 => Self::Dxt3,
                8 => Self::Dxt4,
                9 => Self::Dxt5,
                _ => panic!("Invalid texture format 0x{fmt:x}"),
            },
            _ => panic!("Couldn't get texture format {fmt} for platform {platform:?}"),
        }
    }

    pub fn bpp(&self) -> usize {
        match self {
            Self::R5G6B5 => 16,
            Self::A1R5G5B5 => 16,
            Self::Dxt1 | Self::Dxt1Alpha | Self::Cmpr => 4,
            Self::Dxt2 => 8,
            Self::A4R4G4B4 => 16,
            Self::A8R8G8B8 => 32,
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

#[test]
pub fn assert_struct_size() {
    assert!(std::mem::size_of::<EXGeoTextureHeader>() == 28);
    assert!(std::mem::size_of::<EXBaseGeoTexture>() == 28);
}
