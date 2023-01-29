use binrw::{binrw, BinRead, BinWrite};

use crate::array::EXGeoCommonArrayElement;

// TODO: Remove debug or write a custom impl
#[derive(Debug)]
pub struct EXRelPtr {
    pub offset: i32,
    pub offset_absolute: u64,
}

impl EXRelPtr {
    /// Returns the offset relative to the start of the file
    pub fn offset_absolute(&self) -> u64 {
        self.offset_absolute
    }

    /// Returns the offset to the data relative to the start of the pointer
    pub fn offset_relative(&self) -> i32 {
        self.offset
    }
}

impl BinRead for EXRelPtr {
    type Args = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        options: &binrw::ReadOptions,
        args: Self::Args,
    ) -> binrw::BinResult<Self> {
        let offset = i32::read_options(reader, options, args)?;
        binrw::BinResult::Ok(Self {
            offset,
            offset_absolute: (reader.stream_position()? as i64 + offset as i64) as u64 - 4,
        })
    }
}

impl BinWrite for EXRelPtr {
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
pub struct EXGeoAnimSetHeader {
    pub common: EXGeoCommonArrayElement,
    pub num_anim_sets: u32,
}

#[test]
pub fn assert_struct_size() {
    assert!(std::mem::size_of::<EXGeoCommonArray<0>>() == 8)
}
