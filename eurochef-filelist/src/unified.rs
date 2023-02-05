use std::collections::HashMap;

use binrw::Endian;

pub struct UXFileList {
    /// `None` when using a single '.dat' file
    pub num_filelists: Option<u16>,
    pub build_type: Option<u16>,
    pub endian: Endian,
    pub files: HashMap<String, UXFileInfo>,
}

pub struct UXFileInfo {
    pub addr: u32,
    pub filelist_num: Option<u32>,

    pub length: u32,
    pub hashcode: u32,
    pub version: u32,
    pub flags: u32,
    // ? Should we consider multiple filelocs?
}
