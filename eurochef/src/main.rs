use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
};

use clap::{Parser, Subcommand};
use eurochef_edb::{binrw::BinReaderExt, versions::Platform};
use eurochef_filelist::UXFileList;

#[derive(clap::ValueEnum, PartialEq, Debug, Clone)]
enum PlatformArg {
    Pc,
    Xbox,
    Xbox360,
    Ps2,
    Ps3,
    Gamecube,
    Wii,
    WiiU,
}

impl Into<Platform> for PlatformArg {
    fn into(self) -> Platform {
        match self {
            PlatformArg::Pc => Platform::Pc,
            PlatformArg::Xbox => Platform::Xbox,
            PlatformArg::Xbox360 => Platform::Xbox360,
            PlatformArg::Ps2 => Platform::Ps2,
            PlatformArg::Ps3 => Platform::Ps3,
            PlatformArg::Gamecube => Platform::GameCube,
            PlatformArg::Wii => Platform::Wii,
            PlatformArg::WiiU => Platform::WiiU,
        }
    }
}

#[derive(Parser, Debug)]
struct Args {
    /// Override for platform detection
    #[arg(value_enum, short, long)]
    platform: Option<PlatformArg>,

    /// Decryption key used to decrypt content and filenames
    #[arg(short, long)]
    decryption_key: Option<String>,

    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Commands for working with filelists
    Filelist {
        #[command(subcommand)]
        subcommand: FilelistCommand,
    },
}

#[derive(Subcommand, Debug)]
enum FilelistCommand {
    /// Extract the given filelist
    Extract {
        /// .bin file to use
        filename: String,
    },
}

pub fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("{args:?}");

    match args.cmd {
        Command::Filelist { subcommand } => handle_filelist(subcommand),
    }
}

fn handle_filelist(cmd: FilelistCommand) -> anyhow::Result<()> {
    match cmd {
        FilelistCommand::Extract { filename } => {
            println!("Extracting {filename}");
            let mut f = File::open(&filename)?;
            let filelist = UXFileList::read(&mut f)?;

            let file_base = &filename[..filename.len() - 3];
            let mut data_files = vec![];
            if let Some(num_filelists) = filelist.num_filelists {
                for i in 0..(num_filelists + 1) {
                    data_files.push(File::open(format!("{}{:03}", file_base, i))?);
                }
            } else {
                data_files.push(File::open(format!("{}DAT", file_base))?);
            }

            for (filename, info) in filelist.files.iter() {
                let filename_fixed = filename.replace('\\', "/");
                let fpath = Path::new(&filename_fixed);
                println!("{:?} ({} bytes) ", fpath, info.length);

                let df = &mut data_files[info.filelist_num.unwrap_or(0) as usize];

                df.seek(std::io::SeekFrom::Start(info.addr as u64))?;

                let magic: u32 = df
                    .read_type(filelist.endian)
                    .expect("Failed to read file header");

                let mut filesize = info.length;

                if magic == 0x47454F4D {
                    df.seek(std::io::SeekFrom::Current(0x10))?;
                    filesize = df
                        .read_type(filelist.endian)
                        .expect("Failed to read GeoFile size");
                }

                df.seek(std::io::SeekFrom::Start(info.addr as u64))?;

                let mut data = vec![0u8; filesize as usize];
                df.read(&mut data)?;

                let fpath_noprefix = Path::new(&fpath.to_str().unwrap()[3..]);
                std::fs::create_dir_all(fpath_noprefix.parent().unwrap())?;
                File::create(fpath_noprefix)?.write(&data)?;
            }

            Ok(())
        }
    }
}
