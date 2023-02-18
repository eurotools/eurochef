use binrw::binrw;

// Version 4

#[binrw]
#[derive(Debug)]
pub struct EXFileListHeader4 {
    #[brw(assert(version.eq(&4)))]
    pub version: u32,
    pub filesize: u32,
    #[bw(calc = fileinfo.len() as i32)]
    pub num_files: i32,
    pub filename_list_offset: u32,
    #[br(count = num_files)]
    pub fileinfo: Vec<FileInfo4>,
}

#[binrw]
#[derive(Debug)]
pub struct FileInfo4 {
    pub addr: u32,
    pub length: u32,
    pub hashcode: u32,
    pub version: u32,
    pub flags: u32,
}

// Version 5-7

#[binrw]
#[derive(Debug)]
pub struct EXFileListHeader5 {
    #[brw(assert(version.ge(&5) && version.le(&7)))]
    pub version: u32,
    pub filesize: u32,
    #[bw(calc = fileinfo.len() as i32)]
    pub num_files: i32,
    pub build_type: u16,
    pub num_filelists: u16,
    pub filename_list_offset: u32,
    #[br(count = num_files)]
    pub fileinfo: Vec<FileInfo5>,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct FileInfo5 {
    pub length: u32,
    pub hashcode: u32,
    pub version: u32,
    pub flags: u32,

    #[bw(calc = fileloc.len() as u32)]
    pub num_fileloc: u32,

    #[br(count = num_fileloc)]
    pub fileloc: Vec<FileLoc5>,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct FileLoc5 {
    pub addr: u32,
    pub filelist_num: u32,
}

// #[binrw]
// #[derive(Debug)]
// pub struct FileInfo9 {
//     pub length: u32,
//     pub hashcode: u32,
//     pub version: u32,
//     pub flags: u32,

//     #[bw(calc = fileloc.len() as u32)]
//     pub num_fileloc: u32,

//     #[br(count = num_fileloc)]
//     pub fileloc: Vec<FileLoc5>,
// }
