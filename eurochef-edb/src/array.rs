use binrw::{binrw, BinRead, BinWrite};
use std::{fmt::Debug, slice::Iter};

use crate::{common::EXRelPtr, structure_size_tests};

pub struct EXGeoHashArray<T: BinRead + 'static> {
    pub array_size: i16,

    // ? What is this used for?
    pub hash_size: i16,

    pub rel_offset: EXRelPtr,

    pub data: Vec<T>,
}

impl<T: BinRead> EXGeoHashArray<T> {
    pub fn data_offset_absolute(&self) -> u64 {
        self.rel_offset.offset_absolute()
    }
}

impl<T: BinRead> BinRead for EXGeoHashArray<T> {
    type Args = T::Args;
    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        options: &binrw::ReadOptions,
        args: Self::Args,
    ) -> binrw::BinResult<Self> {
        let mut array = EXGeoHashArray {
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

impl<T: BinRead> BinWrite for EXGeoHashArray<T> {
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

impl<'a, T: BinRead> IntoIterator for &'a EXGeoHashArray<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<T: BinRead + Debug> Debug for EXGeoHashArray<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EXGeoHashArray(")?;
        f.debug_list().entries(self.data.iter()).finish()?;
        f.write_str(")")
    }
}

pub struct EXRelArray<T: BinRead + 'static> {
    pub array_size: i32,

    pub rel_offset: EXRelPtr,

    pub data: Vec<T>,
}

impl<T: BinRead> EXRelArray<T> {
    pub fn data_offset_absolute(&self) -> u64 {
        self.rel_offset.offset_absolute()
    }
}

impl<T: BinRead> BinRead for EXRelArray<T> {
    type Args = T::Args;
    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        options: &binrw::ReadOptions,
        args: Self::Args,
    ) -> binrw::BinResult<Self> {
        let mut array = EXRelArray {
            array_size: BinRead::read_options(reader, options, ())?,
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

impl<T: BinRead> BinWrite for EXRelArray<T> {
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

// FIXME: into_iter consumes, we dont/shouldnt consume
impl<'a, T: BinRead> IntoIterator for &'a EXRelArray<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<T: BinRead + Debug> Debug for EXRelArray<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EXGeoHashArray(")?;
        f.debug_list().entries(self.data.iter()).finish()?;
        f.write_str(")")
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

structure_size_tests!(
    // EXGeoCommonArray<()> = 8,
    EXGeoCommonArrayElement = 16
);
