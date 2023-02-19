use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
};

use eurochef_edb::binrw::BinReaderExt;
use eurochef_filelist::UXFileList;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};

use crate::filelist::TICK_STRINGS;

pub fn execute_command(
    filename: String,
    output_folder: String,
    create_scr: bool,
) -> anyhow::Result<()> {
    println!("Extracting {filename} to {output_folder}");
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

    let pb = ProgressBar::new(filelist.files.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {msg} ({pos}/{len})",
        )
        .unwrap()
        .progress_chars("##-")
        .tick_chars(&TICK_STRINGS),
    );
    pb.set_message("Extracting files");

    for (i, (filename, info)) in filelist.files.iter().enumerate().progress_with(pb) {
        let filename_fixed = filename.replace('\\', "/");
        let fpath = Path::new(&filename_fixed);

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
        df.read_exact(&mut data)?;

        let fpath_noprefix = Path::new(&output_folder).join(&fpath.to_str().unwrap()[3..]);
        std::fs::create_dir_all(fpath_noprefix.parent().unwrap())?;
        File::create(fpath_noprefix)?.write(&data)?;
    }

    println!("Successfully extracted {} files", filelist.files.len());

    Ok(())
}
