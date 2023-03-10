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

pub fn transform_windows_path<P: AsRef<str>>(path: P) -> String {
    path.as_ref().replace("\\", "/")
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
            "gc" => Self::GameCube,
            "pc" => Self::Pc,
            "ps2" => Self::Ps2,
            "xb" => Self::Xbox,
            "xe" => Self::Xbox360,
            "wii" => Self::Wii,
            _ => {
                println!("Platform path prefix found, but can't match it to any known platform! ({path_bin})");
                return None;
            }
        })
    }

    pub fn shorthand(&self) -> &'static str {
        match self {
            Self::Pc => "pc",
            Self::Xbox => "xb",
            Self::Xbox360 => "xe",
            Self::GameCube => "gc",
            Self::Wii => "wii",
            Self::WiiU => "wiiu", // TODO: check
            Self::Ps2 => "ps2",
            Self::Ps3 => "ps3", // TODO: check?
            Self::ThreeDS => "3ds",
        }
    }

    pub fn endianness(&self) -> Endian {
        match *self {
            Self::Pc => Endian::Little,
            Self::Xbox => Endian::Little,
            Self::Xbox360 => Endian::Big,
            Self::GameCube => Endian::Big,
            Self::Wii => Endian::Big,
            Self::WiiU => Endian::Big,
            Self::Ps2 => Endian::Little,
            Self::Ps3 => Endian::Big,
            Self::ThreeDS => Endian::Little,
        }
    }
}
