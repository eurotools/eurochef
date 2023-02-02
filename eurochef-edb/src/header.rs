use std::io::SeekFrom;

use binrw::binrw;

use crate::{
    array::{EXGeoCommonArray, EXGeoCommonArrayElement},
    common::{EXGeoAnimModeHeader, EXGeoAnimSetHeader, EXGeoEntityHeader, EXGeoSpreadSheetHeader},
    structure_size_tests,
    texture::EXGeoTextureHeader,
    versions::{EDB_VERSION_BOND, EDB_VERSION_GFORCE, EDB_VERSION_ICEAGE3},
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

    #[brw(assert(version.eq(&EDB_VERSION_GFORCE) || version.eq(&EDB_VERSION_BOND) || version.eq(&EDB_VERSION_ICEAGE3), "Unsupported version {version}"))]
    pub version: u32,

    pub flags: u32,
    pub time: u32,
    pub file_size: u32,
    pub base_file_size: u32,

    // pub versions: [u32; 6],
    #[brw(seek_before = SeekFrom::Start(0x40))]
    pub section_list: EXGeoCommonArray<()>,
    pub refpointer_list: EXGeoCommonArray<EXGeoRefPointerHeader>,
    pub entity_list: EXGeoCommonArray<EXGeoEntityHeader>,
    pub anim_list: EXGeoCommonArray<()>,
    pub animskin_list: EXGeoCommonArray<()>,
    pub script_list: EXGeoCommonArray<EXGeoScriptHeader>,
    pub map_list: EXGeoCommonArray<EXGeoMapHeader>,
    pub animmode_list: EXGeoCommonArray<EXGeoAnimModeHeader>,
    pub animset_list: EXGeoCommonArray<EXGeoAnimSetHeader>,
    pub particle_list: EXGeoCommonArray<EXGeoParticleHeader>,
    pub swoosh_list: EXGeoCommonArray<EXGeoSwooshHeader>,
    pub spreadsheet_list: EXGeoCommonArray<EXGeoSpreadSheetHeader>,
    pub font_list: EXGeoCommonArray<EXGeoFontHeader>,
    pub forcefeedback_list: EXGeoCommonArray<()>,
    pub material_list: EXGeoCommonArray<()>,
    pub texture_list: EXGeoCommonArray<EXGeoTextureHeader>,

    pub unk_c0: EXGeoCommonArray<()>,
    pub unk_c8: EXGeoCommonArray<()>,
    pub unk_d0: EXGeoCommonArray<()>,
}

structure_size_tests!(EXGeoHeader = 936);
