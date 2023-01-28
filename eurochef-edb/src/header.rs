use std::io::SeekFrom;

use binrw::binrw;

use crate::{array::EXGeoCommonArray, texture::EXGeoTextureHeader};

const EDB_VERSION_GFORCE: u32 = 259;
const EDB_VERSION_BOND: u32 = 263;

#[rustfmt::skip]
#[binrw]
#[brw(magic = 0x47454F4Du32)]
#[derive(Debug)]
pub struct EXGeoHeader {
    pub hashcode: u32,

    #[brw(assert(version.eq(&EDB_VERSION_GFORCE) || version.eq(&EDB_VERSION_BOND), "Unsupported version {version}"))]
    pub version: u32,

    pub flags: u32,
    pub time: u32,
    pub file_size: u32,
    pub base_file_size: u32,

    // pub versions: [u32; 6],

    #[brw(seek_before = SeekFrom::Start(0x40))] 
    pub section_list: EXGeoCommonArray<()>,
    pub refpointer_list: EXGeoCommonArray<()>,
    pub entity_list: EXGeoCommonArray<()>,
    pub anim_list: EXGeoCommonArray<()>,
    pub animskin_list: EXGeoCommonArray<()>,
    pub script_list: EXGeoCommonArray<()>,
    pub map_list: EXGeoCommonArray<()>,
    pub animmode_list: EXGeoCommonArray<()>,
    pub animset_list: EXGeoCommonArray<()>,
    pub particle_list: EXGeoCommonArray<()>,
    pub swoosh_list: EXGeoCommonArray<()>,
    pub spreadsheet_list: EXGeoCommonArray<()>,
    pub font_list: EXGeoCommonArray<()>,
    pub forcefeedback_list: EXGeoCommonArray<()>,
    pub material_list: EXGeoCommonArray<()>,
    pub texture_list: EXGeoCommonArray<EXGeoTextureHeader>,

    pub unk_c0: EXGeoCommonArray<()>,
    pub unk_c8: EXGeoCommonArray<()>,
    pub unk_d0: EXGeoCommonArray<()>,
}
