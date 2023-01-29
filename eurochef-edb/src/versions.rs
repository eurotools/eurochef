use binrw::Endian;

pub const EDB_VERSION_SPYRO_DEMO: u32 = 213;
pub const EDB_VERSION_SPYRO: u32 = 240;
pub const EDB_VERSION_GFORCE: u32 = 259;
pub const EDB_VERSION_ICEAGE3: u32 = 260;
pub const EDB_VERSION_BOND: u32 = 263;

#[derive(Debug, Clone, Copy)]
pub enum Platform {
    Pc,
    Xbox,
    Xbox360,
    GameCube,
    Wii,
    Ps2,
    Ps3,
    ThreeDS,
}

impl Platform {
    // ? This is more an educated "guess" on what the platform is
    pub fn from_flags(flags: u32, endianness: Endian) -> Self {
        match endianness {
            Endian::Little => match flags & 0xff000000 {
                0x20000000 => Platform::Pc, // ! This matches on both PC and XBOX
                0x10000000 => Platform::Ps2,
                _ => panic!(
                    "Couldn't find platform for endianness/flags pair {endianness}/0x{flags:x}"
                ),
            },
            Endian::Big => match flags & 0xff000000 {
                0x20000000 => Platform::GameCube, // ! Matched by X360 as well
                _ => panic!(
                    "Couldn't find platform for endianness/flags pair {endianness}/0x{flags:x}"
                ),
            },
        }
    }
}
