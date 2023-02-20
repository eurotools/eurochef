use std::{any::TypeId, fmt::Debug};

use binrw::{binrw, BinRead, BinWrite};

use crate::{array::EXGeoCommonArrayElement, structure_size_tests};

// TODO: RelPtr16 generic
pub struct EXRelPtr<T: BinRead = (), const OFFSET: i64 = 0> {
    pub offset: i32,
    pub offset_absolute: u64,

    pub data: T,
}

impl<T: BinRead, const OFFSET: i64> EXRelPtr<T, OFFSET> {
    /// Returns the offset relative to the start of the file
    pub fn offset_absolute(&self) -> u64 {
        self.offset_absolute
    }

    /// Returns the offset to the data relative to the start of the pointer
    pub fn offset_relative(&self) -> i32 {
        self.offset
    }
}

impl<T: BinRead, const OFFSET: i64> BinRead for EXRelPtr<T, OFFSET> {
    type Args = T::Args;

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        options: &binrw::ReadOptions,
        args: Self::Args,
    ) -> binrw::BinResult<Self> {
        let offset = i32::read_options(reader, options, ())?;
        let offset_absolute =
            (reader.stream_position()? as i64 + offset as i64 + OFFSET) as u64 - 4;

        let data = if TypeId::of::<T>() != TypeId::of::<()>() {
            let pos_saved = reader.stream_position()?;
            reader.seek(std::io::SeekFrom::Start(offset_absolute))?;

            let inner = T::read_options(reader, options, args.clone())?;
            reader.seek(std::io::SeekFrom::Start(pos_saved))?;

            inner
        } else {
            // Hack to return () (no-op)
            T::read_options(reader, options, args)?
        };

        binrw::BinResult::Ok(Self {
            offset,
            offset_absolute,
            data,
        })
    }
}

impl<T: BinRead, const OFFSET: i64> BinWrite for EXRelPtr<T, OFFSET> {
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

impl<T: BinRead + Debug, const OFFSET: i64> Debug for EXRelPtr<T, OFFSET> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EXRelPtr(")?;
        self.data.fmt(f)?;
        f.write_str(")")
    }
}

impl<T: BinRead + Debug> EXRelPtr<T> {
    /// This method is only meant as a hack for Default implementations
    pub fn new(v: T) -> Self {
        Self {
            data: v,
            offset: 0,
            offset_absolute: 0,
        }
    }
}

#[binrw]
#[derive(Debug)]
pub struct EXGeoSpreadSheetHeader {
    pub common: EXGeoCommonArrayElement,
    pub m_type: u32,
}

#[binrw]
#[derive(Debug)]
pub struct EXGeoEntityHeader {
    pub common: EXGeoCommonArrayElement,
    pub unk_4: u32,
}

#[binrw]
#[derive(Debug)]
pub struct EXGeoAnimModeHeader {
    pub common: EXGeoCommonArrayElement,
    pub num_anim_modes: u32,
}

#[binrw]
#[derive(Debug)]
pub struct EXGeoAnimHeader {
    pub common: EXGeoCommonArrayElement,
    pub motiondata_info_addr: u32,
    _ptr: u32,
    pub datasize: u32,
    pub skin_num: u32,
}

#[binrw]
#[derive(Debug)]
pub struct EXGeoAnimSetHeader {
    pub common: EXGeoCommonArrayElement,
    pub num_anim_sets: u32,
}

#[binrw]
#[derive(Debug)]
pub struct EXGeoAnimSkinHeader {
    pub common: EXGeoCommonArrayElement,
    pub base_skin_num: u32,
    pub mip_ref: u32,
    pub mip_distance: u32,
}

structure_size_tests!(
    EXGeoSpreadSheetHeader = 20,
    EXGeoEntityHeader = 20,
    EXGeoAnimHeader = 32,
    EXGeoAnimModeHeader = 20,
    EXGeoAnimSetHeader = 20
);
