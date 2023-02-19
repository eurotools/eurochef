use std::{
    fs::File,
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
    Xb,
    Xbox,
    Xbox360,
    Ps2,
    Ps3,
    Gc,
    Gamecube,
    Wii,
    WiiU,
}

impl Into<Platform> for PlatformArg {
    fn into(self) -> Platform {
        match self {
            PlatformArg::Pc => Platform::Pc,
            PlatformArg::Xbox | PlatformArg::Xb => Platform::Xbox,
            PlatformArg::Xbox360 => Platform::Xbox360,
            PlatformArg::Ps2 => Platform::Ps2,
            PlatformArg::Ps3 => Platform::Ps3,
            PlatformArg::Gamecube | PlatformArg::Gc => Platform::GameCube,
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

        /// Create a .scr file in the output folder
        #[arg(short = 's', long)]
        create_scr: bool,
    },
    /// Create a new filelist from a folder
    Create {
        /// Folder to read files from
        input_folder: String,

        /// Where to place the generated filelist
        #[arg(default_value = "./Filelist")]
        output_file: String,

        #[arg(long, short = 'l', default_value_t = 'x')]
        drive_letter: char,

        /// Supported versions: 5, 6, 7
        #[arg(long, short, default_value_t = 7)]
        version: u32,

        #[arg(value_enum, short, long, ignore_case = true)]
        platform: PlatformArg,

        /// Maximum size per data file, might be overridden by a .scr file
        #[arg(long, short = 'z', default_value_t = 0x80000000, value_parser = maybe_hex::<u32>)]
        split_size: u32,

        /// .scr file to read options from (currently doesnt support wildcards)
        #[arg(long, short)]
        scr_file: Option<String>,
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
            create_scr,
        } => {
            println!("Extracting {filename}");
            let mut f = File::open(&filename)?;
            let filelist = UXFileList::read(&mut f)?;

            std::fs::create_dir_all(&output_folder)?;

            let mut scr_file = create_scr.then_some(
                File::create(Path::new(&output_folder).join("FileList.scr"))
                    .expect("Failed to create .scr file"),
            );

            scr_file.as_mut().map(|f| {
                writeln!(
                    f,
                    "[FileInfomation]

[FileList]\n"
                )
                .expect("Failed to write scr file header");
            });

            let file_base = &filename[..filename.len() - 3];
            let mut data_files = vec![];
            if let Some(num_filelists) = filelist.num_filelists {
                for i in 0..(num_filelists + 1) {
                    data_files.push(File::open(format!("{}{:03}", file_base, i))?);
                }
            } else {
                data_files.push(File::open(format!("{}DAT", file_base))?);
            }

            for (i, (filename, info)) in filelist.files.iter().enumerate() {
                let filename_fixed = filename.replace('\\', "/");
                let fpath = Path::new(&filename_fixed);
                println!(
                    "{} {:?} ({} bytes, hashcode {:08x}, version {}, flags 0x{:x}) ",
                    i, fpath, info.length, info.hashcode, info.version, info.flags
                );

                scr_file.as_mut().map(|f| {
                    writeln!(f, "{}", filename).expect("Failed to write file name to .scr");
                });

                if fpath.to_string_lossy().is_empty() {
                    println!(
                        "Skipping file {} with empty path (hashcode {:08x})",
                        i, info.hashcode
                    );
                    continue;
                }

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
            output_file,
            drive_letter,
            version,
            platform,
            split_size,
            scr_file,
        } => {
            // TODO: Make a trait for filelists bundling both the read and from/into functions so that they can be used genericly
            let platform: Platform = platform.into();
            let endian = platform.endianness();

            if !(5..=7).contains(&version) {
                panic!("Only version 5, 6 and 7 are supported for packing right now")
            }

            println!("Packing files from {input_folder} with drive letter {drive_letter}:");

            let fp_data = format!("{output_file}.000");
            let fp_info = format!("{output_file}.bin");
            let mut f_data = File::create(fp_data)?;

            let mut files: Vec<(String, FileInfo5)> = vec![];

            // TODO: Handle absolute paths on unix
            #[cfg(not(target_os = "windows"))]
            {
                let ifpath = Path::new(&input_folder);
                if ifpath.is_absolute() {
                    panic!("Absolute paths are not supported (yet)");
                }
            }

            let mut file_paths = vec![];

            if let Some(scr_file) = scr_file {
                println!("Reading files in SCR order");
                let scr_files = parse_scr_filelist(scr_file);
                for s in scr_files {
                    if &s[1..=2] != ":\\" {
                        panic!("Invalid path in scr file: {s}");
                    }

                    let path_on_disk = Path::new(&input_folder).join(&s[3..]);
                    file_paths.push((
                        s,
                        path_on_disk
                            .to_string_lossy()
                            .to_string()
                            .replace('\\', "/"),
                    ))
                }
            } else {
                println!("Reading files recursively");
                for e in WalkDir::new(&input_folder) {
                    let e = e?;
                    if e.file_type().is_file() {
                        let fpath = pathdiff::diff_paths(e.path(), &input_folder)
                            .unwrap()
                            .to_string_lossy()
                            .replace('/', "\\");

                        file_paths.push((
                            format!("{drive_letter}:\\{fpath}"),
                            e.path().to_string_lossy().to_string(),
                        ))
                    }
                }
            }

            let mut filelist_num = 0;

            // Virtual path, real path
            for (i, (vpath, rpath)) in file_paths.iter().enumerate() {
                println!("Packing file {vpath}");
                let mut filedata = vec![];
                let mut infile = File::open(rpath)?;
                infile.read_to_end(&mut filedata)?;

                let mut length = filedata.len() as u32;

                let (hashcode, version, flags) = if vpath.to_ascii_lowercase().ends_with(".edb") {
                    // Use base filesize instead of full filesize
                    infile.seek(std::io::SeekFrom::Start(0x18))?;
                    length = infile.read_type(endian)?;

                    // TODO: Kind of inefficient to read from the file again instead of using the buffer
                    infile.seek(std::io::SeekFrom::Start(4))?;
                    (
                        infile.read_type(endian)?,
                        infile.read_type(endian)?,
                        infile.read_type(endian)?,
                    )
                } else if vpath.to_ascii_lowercase().ends_with(".sfx") {
                    infile.seek(std::io::SeekFrom::Start(4))?;
                    (
                        infile.read_le::<u32>()? | 0x21000000,
                        infile.read_type::<u8>(endian)? as u32,
                        0,
                    )
                } else {
                    (0x81000000 | i as u32, 0, 0)
                };

                if f_data.stream_position()? as usize + filedata.len() > split_size as usize {
                    filelist_num += 1;

                    let fp_data = format!("{}.{:03}", output_file, filelist_num);
                    f_data = File::create(&fp_data)?;
                }

                files.push((
                    vpath.clone(),
                    FileInfo5 {
                        version,
                        flags,
                        length,
                        hashcode,
                        fileloc: vec![FileLoc5 {
                            addr: f_data.stream_position()? as u32,
                            filelist_num,
                        }],
                    },
                ));

                f_data.write_all(&filedata)?;

                // Pad next data to 2048 bytes
                let unaligned_pos = f_data.stream_position()?;
                if unaligned_pos & 0x7ff != 0 {
                    let remainder = unaligned_pos % 2048;
                    let aligned_pos = unaligned_pos + (2048 - remainder);
                    f_data.seek(std::io::SeekFrom::Start(aligned_pos))?;
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
                let mut path_buf = v.to_lowercase().as_bytes().to_vec();
                path_buf.push(0);

                if version >= 7 {
                    path::scramble_filename_v7(i as u32, &mut path_buf);
                }

                f_info.write_all(&path_buf)?;
            }

            // Pad the file to 32 bytes
            f_info.seek(std::io::SeekFrom::End(0))?;
            let unaligned_size = f_info.stream_position()?;
            if unaligned_size & 0x1f != 0 {
                let remainder = 32 - (unaligned_size % 32);
                f_info.write_all(&vec![0u8; remainder as usize])?;
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

// TODO: Proper parser for scr files
fn parse_scr_filelist<P: AsRef<Path>>(path: P) -> Vec<String> {
    let mut result = vec![];
    let mut filebuf = String::new();

    let mut f = File::open(path).expect("Failed to open SCR file");
    f.read_to_string(&mut filebuf).unwrap();

    let mut in_filesection = false;
    for l in filebuf.lines() {
        let line = l.trim();

        if in_filesection && line.len() > 3 {
            result.push(line.to_owned())
        }

        if line == "[FileList]" {
            in_filesection = true;
        } else if line.starts_with("[") {
            in_filesection = false;
        }
    }

    result
}
