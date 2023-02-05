use std::any::TypeId;

use binrw::{binrw, BinRead, BinWrite};

use crate::{array::EXGeoCommonArrayElement, structure_size_tests};

// TODO: RelPtr16 generic
#[derive(Debug)]
pub struct EXRelPtr<T: BinRead = ()> {
    pub offset: i32,
    pub offset_absolute: u64,

    pub data: T,
}

impl<T: BinRead> EXRelPtr<T> {
    /// Returns the offset relative to the start of the file
    pub fn offset_absolute(&self) -> u64 {
        self.offset_absolute
    }

    /// Returns the offset to the data relative to the start of the pointer
    pub fn offset_relative(&self) -> i32 {
        self.offset
    }
}

impl<T: BinRead> BinRead for EXRelPtr<T> {
    type Args = T::Args;

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        options: &binrw::ReadOptions,
        args: Self::Args,
    ) -> binrw::BinResult<Self> {
        let offset = i32::read_options(reader, options, ())?;
        let offset_absolute = (reader.stream_position()? as i64 + offset as i64) as u64 - 4;

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

impl<T: BinRead> BinWrite for EXRelPtr<T> {
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
