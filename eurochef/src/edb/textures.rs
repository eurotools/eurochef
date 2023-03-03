use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
};

use anyhow::Context;
use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    header::EXGeoHeader,
    texture::EXGeoTexture,
    versions::Platform,
};
use image::RgbaImage;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};

use crate::{edb::TICK_STRINGS, platform::texture, PlatformArg};

pub fn execute_command(
    filename: String,
    platform: Option<PlatformArg>,
    output_folder: Option<String>,
    file_format: String,
) -> anyhow::Result<()> {
    let output_folder = output_folder.unwrap_or(format!(
        "./textures/{}/",
        Path::new(&filename)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    ));
    let output_folder = Path::new(&output_folder);

    let mut file = File::open(&filename)?;
    let endian = if file.read_ne::<u8>().unwrap() == 0x47 {
        Endian::Big
    } else {
        Endian::Little
    };
    file.seek(std::io::SeekFrom::Start(0))?;

    let header = file
        .read_type::<EXGeoHeader>(endian)
        .expect("Failed to read header");

    let platform = platform
        .map(|p| p.into())
        .or(Platform::from_path(&filename))
        .expect("Failed to detect platform");

    println!("Selected platform {platform:?}");

    let pb = ProgressBar::new(header.texture_list.data.len() as u64)
        .with_finish(indicatif::ProgressFinish::AndLeave);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {msg} ({pos}/{len})",
        )
        .unwrap()
        .progress_chars("##-")
        .tick_chars(&TICK_STRINGS),
    );
    pb.set_message("Extracting textures");

    let mut data = vec![];
    let texture_decoder = texture::create_for_platform(platform);
    for t in header.texture_list.data.iter().progress_with(pb) {
        file.seek(std::io::SeekFrom::Start(t.common.address as u64))?;

        let tex = file
            .read_type_args::<EXGeoTexture>(endian, (header.version, platform))
            .context("Failed to read basetexture")?;

        // println!("{:x} {:?}", t.common.hashcode, tex);

        let calculated_size = texture_decoder.get_data_size(
            tex.width as u32,
            tex.height as u32,
            tex.depth as u32,
            tex.format,
        );

        if let Err(e) = calculated_size {
            println!("Failed to extract texture {:x}: {}", t.common.hashcode, e);
            continue;
        }

        data.clear();
        data.resize(
            tex.data_size
                .map(|v| v as usize)
                .unwrap_or(calculated_size.unwrap()),
            0u8,
        );

        std::fs::create_dir_all(output_folder)?;

        let mut output = RgbaImage::new(tex.width as u32, tex.height as u32);
        for (i, frame_offset) in tex.frame_offsets.iter().enumerate() {
            file.seek(std::io::SeekFrom::Start(frame_offset.offset_absolute()))?;

            if let Err(e) = file.read_exact(&mut data) {
                println!("Failed to read texture {:x}: {}", t.common.hashcode, e);
            }

            if let Err(e) = texture_decoder.decode(
                &data,
                &mut output,
                tex.width as u32,
                tex.height as u32,
                tex.depth as u32,
                tex.format,
            ) {
                println!("Texture {:08x} failed to decode: {}", t.common.hashcode, e);
                continue;
            }

            let filename = output_folder.join(format!(
                "{:08x}_frame{}.{}",
                t.common.hashcode, i, file_format
            ));
            match file_format.as_str() {
                "qoi" => {
                    let filedata =
                        qoi::encode_to_vec(output.as_raw(), tex.width as u32, tex.height as u32)?;
                    let mut imgfile =
                        File::create(filename).context("Failed to create output image")?;
                    imgfile.write_all(&filedata)?;
                }
                _ => {
                    output.save(filename)?;
                }
            }
        }
    }

    Ok(())
}
