use std::path::Path;

use binrw::Endian;

pub const EDB_VERSION_SPYRO_DEMO: u32 = 213;
pub const EDB_VERSION_SPYRO: u32 = 240;
pub const EDB_VERSION_PREDATOR: u32 = 250;
pub const EDB_VERSION_GFORCE: u32 = 259;
pub const EDB_VERSION_ICEAGE3: u32 = 260;
pub const EDB_VERSION_BOND: u32 = 263;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Pc,
    Xbox,
    Xbox360,
    GameCube,
    Wii,
    WiiU,
    Ps2,
    Ps3,
    ThreeDS,
}

impl Platform {
    pub fn from_path<P>(path: P) -> Option<Self>
    where
        P: AsRef<Path>,
    {
        let path_bin = path
            .as_ref()
            .iter()
            .rfind(|p| p.to_string_lossy().to_lowercase().starts_with("_bin_"))?
            .to_string_lossy()
            .to_lowercase()
            .to_owned();

        Some(match path_bin.get(5..)? {
            "gc" => Platform::GameCube,
            "pc" => Platform::Pc,
            "ps2" => Platform::Ps2,
            "xb" => Platform::Xbox,
            "xe" => Platform::Xbox360,
            _ => {
                println!("Platform path prefix found, but can't match it to any known platform! ({path_bin})");
                return None;
            }
        })
    }

    pub fn endianness(&self) -> Endian {
        match *self {
            Platform::Pc => Endian::Little,
            Platform::Xbox => Endian::Little,
            Platform::Xbox360 => Endian::Big,
            Platform::GameCube => Endian::Big,
            Platform::Wii => Endian::Big,
            Platform::WiiU => Endian::Big,
            Platform::Ps2 => Endian::Little,
            Platform::Ps3 => Endian::Big,
            Platform::ThreeDS => Endian::Little,
        }
    }
}
