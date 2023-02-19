use std::io::SeekFrom;

use binrw::binrw;

use crate::{
    array::{EXGeoCommonArrayElement, EXGeoHashArray},
    common::{
        EXGeoAnimHeader, EXGeoAnimModeHeader, EXGeoAnimSetHeader, EXGeoEntityHeader,
        EXGeoSpreadSheetHeader,
    },
    structure_size_tests,
    texture::EXGeoTextureHeader,
    // versions::{EDB_VERSION_BOND, EDB_VERSION_GFORCE, EDB_VERSION_ICEAGE3},
};

pub type EXGeoMapHeader = EXGeoCommonArrayElement;
pub type EXGeoParticleHeader = EXGeoCommonArrayElement;
pub type EXGeoRefPointerHeader = EXGeoCommonArrayElement;
pub type EXGeoScriptHeader = EXGeoCommonArrayElement;
pub type EXGeoSwooshHeader = EXGeoCommonArrayElement;
pub type EXGeoFontHeader = EXGeoCommonArrayElement;

#[binrw]
#[brw(magic = 0x47454F4Du32)]
#[derive(Debug)]
pub struct EXGeoHeader {
    pub hashcode: u32,

    #[brw(assert(version.ge(&252) || version.le(&263), "Unsupported version {version}"))]
    pub version: u32,

    pub flags: u32,
    pub time: u32,
    pub file_size: u32,
    pub base_file_size: u32,

    // pub versions: [u32; 6],
    #[brw(seek_before = SeekFrom::Start(0x40))]
    pub section_list: EXGeoHashArray<()>,
    pub refpointer_list: EXGeoHashArray<EXGeoRefPointerHeader>,
    pub entity_list: EXGeoHashArray<EXGeoEntityHeader>, // 0x50
    pub anim_list: EXGeoHashArray<EXGeoAnimHeader>,
    pub animskin_list: EXGeoHashArray<()>, // 0x60
    pub script_list: EXGeoHashArray<EXGeoScriptHeader>,
    pub map_list: EXGeoHashArray<EXGeoMapHeader>, // 0x70
    pub animmode_list: EXGeoHashArray<EXGeoAnimModeHeader>,
    pub animset_list: EXGeoHashArray<EXGeoAnimSetHeader>, // 0x80
    pub particle_list: EXGeoHashArray<EXGeoParticleHeader>,
    pub swoosh_list: EXGeoHashArray<EXGeoSwooshHeader>, // 0x90
    pub spreadsheet_list: EXGeoHashArray<EXGeoSpreadSheetHeader>,
    pub font_list: EXGeoHashArray<EXGeoFontHeader>, // 0xa0
    pub forcefeedback_list: EXGeoHashArray<()>,
    pub material_list: EXGeoHashArray<()>, // 0xb0
    pub texture_list: EXGeoHashArray<EXGeoTextureHeader>,

    pub unk_c0: EXGeoHashArray<()>,
    pub unk_c8: EXGeoHashArray<()>,
    pub unk_d0: EXGeoHashArray<()>,
}

structure_size_tests!(EXGeoHeader = 936);
