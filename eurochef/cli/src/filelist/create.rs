use anyhow::Context;
use eurochef_edb::binrw::{BinReaderExt, BinWriterExt};
use eurochef_edb::versions::{transform_windows_path, Platform};
use eurochef_filelist::{
    path,
    structures::{EXFileListHeader5, FileInfo5, FileLoc5},
};
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
};
use walkdir::WalkDir;

use crate::filelist::TICK_STRINGS;
use crate::PlatformArg;
use core::cmp::max;

pub fn execute_command(
    input_folder: String,
    output_file: String,
    drive_letter: char,
    version: u32,
    platform: PlatformArg,
    split_size: u32,
    scr_file: Option<String>,
) -> anyhow::Result<()> {
    // TODO: Make a trait for filelists bundling both the read and from/into functions so that they can be used genericly
    let platform: Platform = platform.into();
    let endian = platform.endianness();

    if !(5..=7).contains(&version) {
        panic!("Only version 5, 6 and 7 are supported for packing right now")
    }

    println!("Packing files from {input_folder} with drive letter {drive_letter}:");

    let fp_data = format!("{output_file}.000");
    let fp_info = format!("{output_file}.bin");
    let mut f_data = File::create(fp_data).context("Failed to create output file")?;

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
        let scr_files = parse_scr_filelist(scr_file).context("Failed to read SCR file")?;
        let mut file_paths_temp = vec![];
        for s in scr_files {
            if &s[1..=2] != ":\\" {
                panic!("Invalid path in scr file: {s}");
            }

            file_paths_temp.push(s)
        }
        println!("Loaded {} paths from SCR", file_paths_temp.len());
        let pb = ProgressBar::new(0);
        pb.set_style(
            ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap()
                .progress_chars("##-")
                .tick_chars(&TICK_STRINGS),
        );
        pb.set_message("Locating files");

        for p in &file_paths_temp {
            // Join path to root folder and change globbing pattern to be `glob`-compatible
            let path_on_disk =
                Path::new(&input_folder).join(transform_windows_path(&p[3..].replace("#", "?")));

            let mut found_files = false;
            #[allow(for_loops_over_fallibles)]
            for entry in glob::glob(&path_on_disk.to_string_lossy())
                .context("Failed to parse SCR glob pattern, report to cohae")?
            {
                let entry = entry?;
                if std::fs::metadata(&entry)?.is_file() {
                    let fpath = pathdiff::diff_paths(&entry, &input_folder)
                        .unwrap()
                        .to_string_lossy()
                        .replace('/', "\\");

                    file_paths.push((
                        format!("{drive_letter}:\\{fpath}"),
                        entry.to_string_lossy().to_string(),
                    ))
                }

                found_files = true;
            }

            if !found_files {
                // TODO: log crate when?
                println!("Warning: SCR path {p} yielded no results")
            }
        }
    } else {
        let pb = ProgressBar::new(0);
        pb.set_style(
            ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap()
                .progress_chars("##-")
                .tick_chars(&TICK_STRINGS),
        );
        pb.set_message("Locating files");

        for e in WalkDir::new(&input_folder) {
            pb.tick();
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

        pb.finish_and_clear();
        println!("Located {} files", file_paths.len());
    }

    let mut filelist_num = 0;

    let pb =
        ProgressBar::new(file_paths.len() as u64).with_finish(indicatif::ProgressFinish::AndLeave);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {msg} ({pos}/{len})",
        )
        .unwrap()
        .progress_chars("##-")
        .tick_chars(&TICK_STRINGS),
    );
    pb.set_message("Packing files");

    let mut common_garbage_buf = vec![];

    // Virtual path, real path
    for (i, (vpath, rpath)) in file_paths.iter().enumerate().progress_with(pb) {
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

        // swy: write the actual file contents
        f_data.write_all(&filedata)?;


        // Pad next data to 2048 bytes
        let unaligned_pos = f_data.stream_position()?;
        let aligned_pos = (unaligned_pos + 0x7ff) & !0x7ff; /* swy: 2048 - 1 = 0x7ff */
        let difference: usize = (aligned_pos - unaligned_pos) as usize;

        // swy: this funky buffer holds the cumulative overwritten contents of everything that came before;
        //      we need to use this as a sort of emulation layer for matching how the original XUtil memcpy()'ed not only
        //      until the end of the current file, but also extending it to any garbage data that may lay beyond the limit, what's there?
        //      probably the data at that offset of any previous file big enough to reach there, otherwise we'll use zeroes

        //      so here we're always making the buffer big enough to cover the total space that we need, and then pasting the file to cover
        //      from the start, until its maximum size, anything that remains keeps the previous data, because that's what we like ¯\_(ツ)_/¯
        common_garbage_buf.resize(max(common_garbage_buf.len(), filedata.len()), 0);
        common_garbage_buf[0 .. filedata.len()].copy_from_slice(&filedata);

        println!(
            "{} {} remaining space: {:#x} - {:#x} = {:#x}",
            i, vpath, unaligned_pos, aligned_pos, difference
        );

        if difference > 0 {
            // swy: fill out the padding with the correct garbage at that offset, 
            //      this should make the diff engines' life easier. and we should
            //      get a byte-by-byte perfect reconstruction for pristine files,
            //      (as long as they get stored in the same order with the help of a handy .scr spec file)
            let filedata_len_plus_padding = filedata.len() + difference;

            if common_garbage_buf.len() < filedata_len_plus_padding {
                common_garbage_buf.resize(filedata_len_plus_padding, 0);
            }
            f_data.write(&common_garbage_buf[filedata.len() .. filedata_len_plus_padding])?;
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

// TODO: Proper parser for scr files
fn parse_scr_filelist<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<String>> {
    let mut result = vec![];
    let mut filebuf = String::new();

    let mut f = File::open(path).expect("Failed to open SCR file");
    f.read_to_string(&mut filebuf)?;

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

    Ok(result)
}
