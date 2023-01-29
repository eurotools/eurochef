use binrw::{binrw, BinRead, BinWrite};
use std::{fmt::Debug, ops::Deref, slice::Iter};

use crate::{common::EXRelPtr, util::get_last_path_part};

// #[binrw]
pub struct EXGeoCommonArray<T: BinRead + 'static> {
    pub array_size: i16,

    // ? What is this used for?
    pub hash_size: i16,

    pub rel_offset: EXRelPtr,

    // #[brw(ignore)]
    pub data: Vec<T>,
}

impl<T: BinRead> EXGeoCommonArray<T> {
    pub fn data_offset_absolute(&self) -> u64 {
        self.rel_offset.offset_absolute()
    }
}

impl<T: BinRead> BinRead for EXGeoCommonArray<T> {
    type Args = T::Args;
    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        options: &binrw::ReadOptions,
        args: Self::Args,
    ) -> binrw::BinResult<Self> {
        let mut array = EXGeoCommonArray {
            array_size: BinRead::read_options(reader, options, ())?,
            hash_size: BinRead::read_options(reader, options, ())?,
            rel_offset: BinRead::read_options(reader, options, ())?,
            data: vec![],
        };

        if array.array_size > 0 {
            let pos_saved = reader.stream_position()?;
            reader.seek(std::io::SeekFrom::Start(array.rel_offset.offset_absolute()))?;

            for _ in 0..array.array_size {
                array
                    .data
                    .push(T::read_options(reader, options, args.clone())?)
            }

            reader.seek(std::io::SeekFrom::Start(pos_saved))?;
        }

        Ok(array)
    }
}

impl<T: BinRead> BinWrite for EXGeoCommonArray<T> {
    type Args = ();
    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        _writer: &mut W,
        _options: &binrw::WriteOptions,
        _args: Self::Args,
    ) -> binrw::BinResult<()> {
        todo!()
    }
}

impl<'a, T: BinRead> IntoIterator for &'a EXGeoCommonArray<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<T: BinRead + Debug> Debug for EXGeoCommonArray<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // f.write_fmt(format_args!(
        //     "EXGeoCommonArray<{}>[{}]",
        //     get_last_path_part(std::any::type_name::<T>()).unwrap_or("?"),
        //     self.array_size
        // ));

        f.debug_list().entries(self.data.iter()).finish()
    }
}

#[binrw]
#[derive(Debug)]
pub struct EXGeoCommonArrayElement {
    pub hashcode: u32,
    pub section: u16,
    pub debug: u16,
    pub address: u32,

    // ? Only used internally in the engine?
    _ptr: u32,
}
