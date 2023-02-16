use binrw::{binrw, NullWideString};

use crate::{common::EXRelPtr, structure_size_tests};

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

    /// Pointer to UTF16 string data
    pub string: EXRelPtr<NullWideString>,
    pub userdata: EXRelPtr,
    pub sound_hashcode: u32,
}

structure_size_tests!(EXGeoTextSection = 8);
