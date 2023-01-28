use binrw::binrw;

use crate::{array::EXGeoCommonArrayElement, common::EXRelPtr};

#[binrw]
#[brw(repr(u8))]
#[derive(Debug)]
pub enum EXTexFmt {
    R5G6B5 = 0,
    A1R5G5B5 = 1,
    Dxt1 = 2,
    Dxt1Alt = 3,
    Dxt2 = 4,
    A4R4G4B4 = 5,
    A8R8G8B8 = 6,
    Dxt3 = 7,
    Dxt4 = 8,
    Dxt5 = 9,
}

impl EXTexFmt {
    pub fn bpp(&self) -> usize {
        match self {
            Self::R5G6B5 => 16,
            Self::A1R5G5B5 => 16,
            Self::Dxt1 | Self::Dxt1Alt => 4,
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
    pub format: EXTexFmt,
    pub unk_14: u32,
    pub color: u32,
    // }

    // #[binrw]
    // #[derive(Debug)]
    // pub struct EXGeoTexture {
    //     // ? TODO: Inheritance like with base structs can't exactly be done in rust
    //     // ? Should we just merge the structs??
    //     pub base: EXBaseGeoTexture,
    pad: [u8; 12],

    #[br(count = frame_count)]
    pub frame_offsets: Vec<EXRelPtr>,
}

#[test]
pub fn assert_struct_size() {
    assert!(std::mem::size_of::<EXGeoTextureHeader>() == 28);
    assert!(std::mem::size_of::<EXBaseGeoTexture>() == 28);
}
