use crate::{binrw::BinReaderExt, header::EXGeoHeader};
use anyhow::Context;
use binrw::Endian;
use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, Seek},
    path::Path,
};
use tracing::error;

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
        /* swy: feel free to refactor and tidy up; quirk and dirty proof-of-concept
        without reshuffling things */

        let trop: String = path.as_ref().display().to_string();
        let crap: File = File::open(trop).ok()?;
        let mut reader = BufReader::new(&crap);

        let endian = if reader.read_ne::<u8>().ok()? == 0x47 {
            Endian::Big
        } else {
            Endian::Little
        };
        reader.rewind();
        let header = reader.read_type::<EXGeoHeader>(endian).unwrap();

        // swy: see here: https://sphinxandthecursedmummy.fandom.com/wiki/Filelist#File_format_data_structures
        //          here: https://sphinxandthecursedmummy.fandom.com/wiki/EDB#File_format_data_structures
        //      and here: https://sphinxandthecursedmummy.fandom.com/wiki/EngineX#Games

        // swy: ignore 64-bit version Sphinx PC files; they have a very different format
        if header.version == 182 && header.flags & (1 << 31) != 0 {
            error!("64-bit EDB files are unsupported for the time being.");
            return None;
        }

        if header.flags & (1 << 28) != 0 {
            // swy: marked as PS2; no problems so far
            return Some(Self::Ps2);
        }

        if header.flags & (1 << 29) != 0 {
            // swy: marked as PC; GC and XB are unfortunately also flagged as this, so we need to be sneaky
            if endian == Endian::Big {
                // swy: the only big-endian CPU using these files in this EDB version is on GameCube/Wii
                return Some(Self::GameCube);
            }

            if header.platform_versions[0] > 0 {
                // swy: on Xbox the first one is set to 1, unlike for any other platform, which is normally set to zero
                return Some(Self::Xbox);
            }

            return Some(Self::Pc);
        }

        /* -- */

        let path_bin = path
            .as_ref()
            .iter()
            .rfind(|p| p.to_string_lossy().to_lowercase().starts_with("_bin_"))?
            .to_string_lossy()
            .to_lowercase()
            .to_owned();

        Self::from_shorthand(path_bin.get(5..)?)
    }

    pub fn from_shorthand(code: &str) -> Option<Self> {
        Some(match code {
            "gc" => Self::GameCube,
            "pc" => Self::Pc,
            "ps2" => Self::Ps2,
            "xb" => Self::Xbox,
            "xe" => Self::Xbox360,
            "wii" => Self::Wii,
            _ => {
                error!("Can't match shorthand ID to any known platform! ({code})");
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

    pub fn is_gx(&self) -> bool {
        match *self {
            Platform::GameCube | Platform::Wii => true,
            _ => false,
        }
    }
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Pc => f.write_str("PC"),
            Platform::Xbox => f.write_str("Xbox"),
            Platform::Xbox360 => f.write_str("Xbox 360"),
            Platform::GameCube => f.write_str("GameCube"),
            Platform::Wii => f.write_str("Wii"),
            Platform::WiiU => f.write_str("Wii U"),
            Platform::Ps2 => f.write_str("PlayStation 2"),
            Platform::Ps3 => f.write_str("PlayStation 3"),
            Platform::ThreeDS => f.write_str("3DS"),
        }
    }
}
