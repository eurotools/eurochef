use binrw::{binrw, BinRead, BinWrite};
use serde::Serialize;
use std::{
    fmt::Debug,
    mem::size_of,
    ops::Index,
    slice::{Iter, SliceIndex},
};

use crate::common::EXRelPtr;

#[derive(Clone)]
pub struct EXGeoHashArray<T: BinRead + 'static> {
    array_size: i16,

    // ? What is this used for?
    _hash_size: i16,

    rel_offset: EXRelPtr,

    data: Vec<T>,
}

impl<T: BinRead> EXGeoHashArray<T> {
    pub fn data_offset_absolute(&self) -> u64 {
        self.rel_offset.offset_absolute()
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.data.iter()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn data(&self) -> &Vec<T> {
        return &self.data;
    }
}

impl<T: BinRead, I: SliceIndex<[T]>> Index<I> for EXGeoHashArray<T> {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.data.index(index)
    }
}

impl<T: BinRead> BinRead for EXGeoHashArray<T>
where
    for<'a> <T as BinRead>::Args<'a>: Clone,
{
    type Args<'a> = T::Args<'a>;
    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let mut array = EXGeoHashArray {
            array_size: BinRead::read_options(reader, endian, ())?,
            _hash_size: BinRead::read_options(reader, endian, ())?,
            rel_offset: BinRead::read_options(reader, endian, ())?,
            data: vec![],
        };

        if array.array_size > 0 && size_of::<T>() != 0 {
            let pos_saved = reader.stream_position()?;
            reader.seek(std::io::SeekFrom::Start(array.rel_offset.offset_absolute()))?;

            for _ in 0..array.array_size {
                array
                    .data
                    .push(T::read_options(reader, endian, args.clone())?)
            }

            reader.seek(std::io::SeekFrom::Start(pos_saved))?;
        }

        Ok(array)
    }
}

impl<T: BinRead> BinWrite for EXGeoHashArray<T> {
    type Args<'a> = ();
    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        _writer: &mut W,
        _endian: binrw::Endian,
        _args: Self::Args<'_>,
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
        f.write_str(
            format!(
                ", count={}, addr=0x{:x}",
                self.array_size,
                self.rel_offset.offset_absolute()
            )
            .as_str(),
        )?;
        f.write_str(")")
    }
}

impl<T: BinRead> Default for EXGeoHashArray<T> {
    fn default() -> Self {
        Self {
            rel_offset: EXRelPtr::new(()),
            array_size: 0,
            _hash_size: 0,
            data: vec![],
        }
    }
}

impl<T: BinRead + Serialize> Serialize for EXGeoHashArray<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.data.serialize(serializer)
    }
}

#[derive(Clone)]
pub struct EXRelArray<T: BinRead + 'static> {
    array_size: i32,

    rel_offset: EXRelPtr,

    data: Vec<T>,
}

impl<T: BinRead> EXRelArray<T> {
    pub fn data_offset_absolute(&self) -> u64 {
        self.rel_offset.offset_absolute()
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.data.iter()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn data(&self) -> &Vec<T> {
        return &self.data;
    }
}

impl<T: BinRead> BinRead for EXRelArray<T>
where
    for<'a> T::Args<'a>: Clone,
{
    type Args<'a> = T::Args<'a>;
    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let mut array = EXRelArray {
            array_size: BinRead::read_options(reader, endian, ())?,
            rel_offset: BinRead::read_options(reader, endian, ())?,
            data: vec![],
        };

        if array.array_size > 0 && size_of::<T>() != 0 {
            let pos_saved = reader.stream_position()?;
            reader.seek(std::io::SeekFrom::Start(array.rel_offset.offset_absolute()))?;

            for _ in 0..array.array_size {
                array
                    .data
                    .push(T::read_options(reader, endian, args.clone())?)
            }

            reader.seek(std::io::SeekFrom::Start(pos_saved))?;
        }

        Ok(array)
    }
}

impl<T: BinRead> BinWrite for EXRelArray<T> {
    type Args<'a> = ();
    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        _writer: &mut W,
        _endian: binrw::Endian,
        _args: Self::Args<'_>,
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
        f.write_str(
            format!(
                ", count={}, addr=0x{:x}",
                self.array_size,
                self.rel_offset.offset_absolute()
            )
            .as_str(),
        )?;
        f.write_str(")")
    }
}

impl<T: BinRead + Serialize> Serialize for EXRelArray<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.data.serialize(serializer)
    }
}

#[binrw]
#[derive(Debug, Clone)]
pub struct EXGeoCommonArrayElement {
    pub hashcode: u32,
    pub section: u16,
    pub debug: u16,
    pub address: u32,

    // ? Only used internally in the engine?
    _ptr: u32,
}
