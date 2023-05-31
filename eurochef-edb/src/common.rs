use std::{any::TypeId, fmt::Debug, ops::Deref};

use binrw::{binrw, BinRead, BinReaderExt, BinWrite};
use num::NumCast;
use serde::Serialize;

use crate::array::EXGeoCommonArrayElement;

pub type EXVector3 = [f32; 3]; // TODO: Replace with structs
pub type EXVector = [f32; 4];
pub type EXVector2 = [f32; 2];

// TODO: RelPtr16 generic
#[derive(Clone)]
pub struct EXRelPtr<T: BinRead = (), OT: BinRead + NumCast = i32, const OFFSET: i64 = 0> {
    offset: OT,
    offset_absolute: u64,

    data: T,
}

impl<T: BinRead, OT: BinRead + NumCast, const OFFSET: i64> EXRelPtr<T, OT, OFFSET> {
    /// Returns the offset relative to the start of the file
    pub fn offset_absolute(&self) -> u64 {
        self.offset_absolute
    }

    /// Returns the offset to the data relative to the start of the pointer
    pub fn offset_relative(&self) -> i32 {
        self.offset.to_i32().unwrap()
    }
}

impl<'a, T: BinRead, OT: BinRead + NumCast, const OFFSET: i64> BinRead for EXRelPtr<T, OT, OFFSET>
where
    <OT as BinRead>::Args<'a>: Default + Clone,
    T: 'static,
{
    type Args<'b> = T::Args<'b>;

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let offset: OT = reader.read_type(endian)?;
        let offset_absolute =
            (reader.stream_position()? as i64 + offset.to_i64().unwrap() + OFFSET) as u64 - 4;

        let data = if TypeId::of::<T>() != TypeId::of::<()>() {
            let pos_saved = reader.stream_position()?;
            reader.seek(std::io::SeekFrom::Start(offset_absolute))?;

            let inner = T::read_options(reader, endian, args)?;
            reader.seek(std::io::SeekFrom::Start(pos_saved))?;

            inner
        } else {
            // Hack to return () (no-op)
            T::read_options(reader, endian, args)?
        };

        binrw::BinResult::Ok(Self {
            offset,
            offset_absolute,
            data,
        })
    }
}

impl<T: BinRead, OT: BinRead + NumCast, const OFFSET: i64> BinWrite for EXRelPtr<T, OT, OFFSET> {
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

impl<T: BinRead + Debug, OT: BinRead + NumCast, const OFFSET: i64> Debug
    for EXRelPtr<T, OT, OFFSET>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EXRelPtr(")?;
        self.data.fmt(f)?;
        f.write_str(format!(", addr=0x{:x}", self.offset_absolute).as_str())?;
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

impl<T: BinRead + Serialize, OT: BinRead + NumCast, const OFFSET: i64> Serialize
    for EXRelPtr<T, OT, OFFSET>
where
    T: 'static,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // If using unit type, serialize the absolute address instead.
        if TypeId::of::<T>() == TypeId::of::<()>() {
            let addr_repr = format!("relptr(0x{:x})", self.offset_absolute);
            addr_repr.serialize(serializer)
        } else {
            self.data.serialize(serializer)
        }
    }
}

impl<T: BinRead + Debug> Deref for EXRelPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
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
#[brw(import(version: u32))]
pub struct EXGeoEntityHeader {
    pub common: EXGeoCommonArrayElement,
    pub unk_4: u32,
    #[brw(if(version <= 221))]
    pub ext: Option<EntityHeaderExt>,
}

#[binrw]
#[derive(Debug)]
pub struct EntityHeaderExt {
    pub mip_ref: f32,
    pub mip_distance: [f32; 3],
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
#[brw(import(version: u32))]
pub struct EXGeoAnimSkinHeader {
    pub common: EXGeoCommonArrayElement,
    pub base_skin_num: u32,
    pub mip_ref: u32,
    pub mip_distance: u32,
    #[brw(if(version.eq(&248) || version.eq(&252)))]
    pub unk: [u32; 2],
}
