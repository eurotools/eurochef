use binrw::binrw;

use crate::common::EXRelPtr;

#[binrw]
#[derive(Debug)]
pub struct EXGeoSpreadSheet {
    pub section_count: u32,

    #[br(count = section_count)]
    pub sections: Vec<EXGeoTextSection>,
}

#[binrw]
#[derive(Debug)]
pub struct EXGeoTextSection {
    pub hashcode: u32,
    pub refpointer_index: u32,
}

#[binrw]
#[derive(Debug)]
pub struct EXGeoTextItem {
    pub hashcode: u32,

    // TODO: It would be super convenient if we could have EXRelPtr read the string on it's own. Would like to have a separate struct for it though
    /// Pointer to UTF16 string data
    pub string: EXRelPtr,
    pub userdata: EXRelPtr,
    pub sound_hashcode: u32,
}
