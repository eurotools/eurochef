use std::{
    fs::{self, File, FileType},
    io::{Read, Seek, Write},
    path::Path,
};

use clap::{Parser, Subcommand};
use clap_num::maybe_hex;
use eurochef_edb::{
    binrw::{BinReaderExt, BinWriterExt},
    versions::Platform,
};
use eurochef_filelist::{
    path,
    structures::{EXFileListHeader5, FileInfo5, FileLoc5},
    UXFileList,
};
use walkdir::WalkDir;

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

#[derive(Subcommand, Debug, Clone)]
enum FilelistCommand {
    /// Extract a filelist
    Extract {
        /// .bin file to use
        filename: String,

        /// The folder to extract to (will be created if it doesnt exist)
        #[arg(default_value = "./")]
        output_folder: String,
    },
    /// Create a new filelist from a folder
    Create {
        /// Folder to read files from
        input_folder: String,

        /// The folder to put the generated files in
        #[arg(default_value = "./")]
        output_folder: String,

        /// The output filename
        #[arg(default_value = "Filelist")]
        file_name: String,

        #[arg(long, short = 'l', default_value_t = 'x')]
        drive_letter: char,

        /// Supported versions: 5, 6, 7
        #[arg(long, short, default_value_t = 7)]
        version: u32,

        #[arg(value_enum, short, long)]
        platform: PlatformArg,

        /// Maximum size per data file
        #[arg(long, short, default_value_t = 0x80000000, value_parser = maybe_hex::<u32>)]
        split_size: u32,
    },
}

pub fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match &args.cmd {
        Command::Filelist { subcommand } => handle_filelist(subcommand.clone(), args),
    }
}

// TODO: Split commands into separate files
fn handle_filelist(cmd: FilelistCommand, args: Args) -> anyhow::Result<()> {
    match cmd {
        FilelistCommand::Extract {
            filename,
            output_folder,
        } => {
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

                let fpath_noprefix = Path::new(&output_folder).join(&fpath.to_str().unwrap()[3..]);
                std::fs::create_dir_all(fpath_noprefix.parent().unwrap())?;
                File::create(fpath_noprefix)?.write(&data)?;
            }

            Ok(())
        }
        FilelistCommand::Create {
            input_folder,
            output_folder,
            file_name,
            drive_letter,
            version,
            platform,
            split_size,
        } => {
            // TODO: Make a trait for filelists bundling both the read and from/into functions so that they can be used genericly
            let platform: Platform = platform.into();
            let endian = platform.endianness();

            if !(5..=7).contains(&version) {
                panic!("Only version 5, 6 and 7 are supported for packing right now")
            }

            println!("Packing files from {input_folder} with drive letter {drive_letter}:");

            let fp_data = Path::new(&output_folder).join(file_name.clone() + ".000");
            let fp_info = Path::new(&output_folder).join(file_name.clone() + ".bin");
            let mut f_data = File::create(fp_data)?;

            let mut files: Vec<(String, FileInfo5)> = vec![];

            // TODO: Handle absolute paths
            {
                let ifpath = Path::new(&input_folder);
                if ifpath.is_absolute() {
                    panic!("Absolute paths are not supported (yet)");
                }
            }

            let mut filelist_num = 0;
            let mut filelist_size = 0;
            for e in WalkDir::new(input_folder) {
                let e = e?;
                if e.file_type().is_file() {
                    let fpath = e.path().to_string_lossy().replace('/', "\\");
                    println!("{drive_letter}:\\{fpath}");
                    let mut filedata = vec![];
                    let mut infile = File::open(e.path())?;
                    infile.read_to_end(&mut filedata)?;

                    infile.seek(std::io::SeekFrom::Start(4))?;
                    let hashcode = infile.read_type(endian)?;
                    let version = infile.read_type(endian)?;

                    if filelist_size + filedata.len() > split_size as usize {
                        filelist_size = 0;
                        filelist_num += 1;

                        let fp_data = Path::new(&output_folder)
                            .join(file_name.clone() + &format!(".{:03}", filelist_num));
                        f_data = File::create(fp_data)?;
                    }

                    filelist_size += filedata.len();

                    files.push((
                        format!("{drive_letter}:\\{fpath}"),
                        FileInfo5 {
                            version,
                            flags: 536870921, // ! Unhandled
                            length: filedata.len() as u32,
                            hashcode,
                            fileloc: vec![FileLoc5 {
                                addr: f_data.stream_position()? as u32,
                                filelist_num,
                            }],
                        },
                    ));

                    f_data.write_all(&filedata)?;
                }
            }

            let filelist = EXFileListHeader5 {
                version,
                filesize: 0,
                build_type: 1,
                num_filelists: filelist_num as u16,
                filename_list_offset: 0,
                fileinfo: files.iter().map(|(_, v)| v.clone()).collect(),
            };

            let mut f_info = File::create(fp_info)?;
            f_info.write_type(&filelist, endian)?;
            let filename_offset = f_info.stream_position()?;

            let filename_table_size = files.len() * 4;
            let filename_data_offset = filename_offset + filename_table_size as u64;
            let mut offset = filename_data_offset;
            for (i, (v, _)) in files.iter().enumerate() {
                let ptr_offset = filename_offset + i as u64 * 4;
                f_info.write_type(&((offset - ptr_offset) as u32), endian)?;
                offset += v.len() as u64 + 1;
            }

            f_info.seek(std::io::SeekFrom::Start(filename_data_offset))?;

            for (i, (v, _)) in files.iter().enumerate() {
                let mut path_buf = v.as_bytes().to_vec();
                path_buf.push(0);

                if version >= 7 {
                    path::scramble_filename_v7(i as u32, &mut path_buf);
                }

                f_info.write_all(&path_buf)?;
            }

            let file_size = f_info.stream_position()?;

            f_info.seek(std::io::SeekFrom::Start(4))?;
            f_info.write_type(&(file_size as u32), endian)?;

            f_info.seek(std::io::SeekFrom::Start(0x10))?;
            f_info.write_type(&(filename_offset as u32 - 0x10), endian)?;

            println!(
                "Successfully packed {} files into {} data files",
                files.len(),
                filelist_num + 1
            );

            Ok(())
        }
    }
}
