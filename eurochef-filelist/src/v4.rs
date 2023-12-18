use std::io::{Read, Seek};

use crate::structures::EXFileListHeader4;
use crate::unified::{UXFileInfo, UXFileList};

use anyhow::Result;
use binrw::{BinReaderExt, Endian};
use itertools::Itertools;

#[derive(Debug)]
pub struct EXFileList4 {
    pub endian: Endian,
    pub header: EXFileListHeader4,
    pub filenames: Vec<String>,
}

impl EXFileList4 {
    pub fn read<R>(reader: &mut R) -> Result<Self>
    where
        R: Read + Seek,
    {
        let endian = if reader.read_ne::<u8>()? != 0 {
            Endian::Little
        } else {
            Endian::Big
        };
        reader.seek(std::io::SeekFrom::Start(0))?;

        let mut res = Self {
            endian,
            header: reader.read_type(endian)?,
            filenames: vec![],
        };

        // TODO: RelPtr? (currently stuck in eurochef-edb)
        reader.seek(std::io::SeekFrom::Start(
            0xc + res.header.filename_list_offset as u64,
        ))?;

        let base_offset = reader.stream_position()?;
        let mut filename_offsets = vec![];
        for i in 0..res.header.fileinfo.len() as u64 {
            filename_offsets.push(reader.read_type::<u32>(endian)? as u64 + base_offset + i * 4);
        }

        // FIXME: If the strings arent encoded linearly this will be a bit inefficient
        for (start, end) in filename_offsets
            .iter()
            .chain([res.header.filesize as u64].iter())
            .tuple_windows()
        {
            let size = end - start;
            let mut data = vec![0u8; size as usize];
            reader.seek(std::io::SeekFrom::Start(*start))?;
            reader.read_exact(&mut data)?;

            let null_pos = data.iter().position(|&p| p == 0).unwrap_or(data.len());
            res.filenames
                .push(String::from_utf8_lossy(&data[0..null_pos]).to_string());
        }

        Ok(res)
    }
}

impl From<EXFileList4> for UXFileList {
    fn from(val: EXFileList4) -> Self {
        UXFileList {
            num_filelists: None,
            build_type: None,
            endian: val.endian,
            files: val
                .filenames
                .into_iter()
                .zip(val.header.fileinfo)
                .map(|(filename, info)| {
                    (
                        filename,
                        UXFileInfo {
                            addr: info.addr,
                            filelist_num: None,
                            flags: info.flags,
                            hashcode: info.hashcode,
                            length: info.length,
                            version: info.version,
                        },
                    )
                })
                .collect(),
        }
    }
}
