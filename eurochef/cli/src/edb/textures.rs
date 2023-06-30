use std::{
    fs::File,
    io::{BufReader, Write},
    path::Path,
};

use anyhow::Context;
use eurochef_edb::{edb::EdbFile, versions::Platform};
use eurochef_shared::textures::UXGeoTexture;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};

use crate::{edb::TICK_STRINGS, PlatformArg};

pub fn execute_command(
    filename: String,
    platform: Option<PlatformArg>,
    output_folder: Option<String>,
    file_format: String,
    no_apngs: bool,
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
    std::fs::create_dir_all(output_folder)?;

    let platform = platform
        .map(|p| p.into())
        .or(Platform::from_path(&filename))
        .expect("Failed to detect platform");

    let mut file = File::open(&filename)?;
    let mut reader = BufReader::new(&mut file);
    let mut edb = EdbFile::new(&mut reader, platform)?;
    let header = edb.header.clone();

    let pb = ProgressBar::new(header.texture_list.len() as u64)
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

    let textures = UXGeoTexture::read_all(&mut edb);
    for it in textures.into_iter().progress_with(pb) {
        let hash_str = format!("0x{:x}", it.hashcode);
        let _span = error_span!("texture", hash = %hash_str);
        let _span_enter = _span.enter();

        match it.data {
            Ok(t) => {
                if t.depth > 1 {
                    warn!("Texture is 3D, skipping");
                    continue;
                }

                if file_format == "png" && !no_apngs {
                    let filename =
                        output_folder.join(format!("{:08x}.{}", it.hashcode, file_format));
                    if t.frames.len() > 1 {
                        let png_frames: Vec<apng::PNGImage> = t
                            .frames
                            .into_iter()
                            .map(|data| {
                                apng::load_dynamic_image(
                                    image::RgbaImage::from_vec(
                                        t.width as u32,
                                        t.height as u32,
                                        data,
                                    )
                                    .unwrap()
                                    .into(),
                                )
                                .unwrap()
                            })
                            .collect();

                        let apng_config = apng::create_config(&png_frames, Some(0))?;
                        let mut imgfile =
                            File::create(filename).context("Failed to create output image")?;
                        let mut encoder = apng::Encoder::new(&mut imgfile, apng_config)?;
                        encoder.encode_all(
                            png_frames,
                            Some(&apng::Frame {
                                delay_den: Some(1000),
                                delay_num: Some((1000.0 / t.framerate as f32) as u16),
                                ..Default::default()
                            }),
                        )?;
                        encoder.finish_encode()?;
                    } else {
                        if let Some(f) = t.frames.into_iter().nth(0) {
                            let image =
                                image::RgbaImage::from_vec(t.width as u32, t.height as u32, f)
                                    .unwrap();
                            image.save(filename)?;
                        }
                    }
                } else {
                    for (i, f) in t.frames.into_iter().enumerate() {
                        let filename = output_folder
                            .join(format!("{:08x}_frame{}.{}", it.hashcode, i, file_format));
                        match file_format.as_str() {
                            "qoi" => {
                                let filedata =
                                    qoi::encode_to_vec(f, t.width as u32, t.height as u32)?;
                                let mut imgfile = File::create(filename)
                                    .context("Failed to create output image")?;
                                imgfile.write_all(&filedata)?;
                            }
                            _ => {
                                let image =
                                    image::RgbaImage::from_vec(t.width as u32, t.height as u32, f)
                                        .unwrap();
                                image.save(filename)?;
                            }
                        }
                    }
                }
            }
            Err(e) => error!("{e:?}"),
        }
    }

    info!("Successfully extracted textures!");

    Ok(())
}
