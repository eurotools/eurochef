use std::{
    env::args,
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
};

use eurochef_edb::{
    binrw::{BinReaderExt, Endian, NullWideString},
    header::EXGeoHeader,
    text::{EXGeoSpreadSheet, EXGeoTextItem},
    texture::{EXGeoTexture, EXTexFmt},
    versions::Platform,
};
use eurochef_elx::ELXML;

fn main() -> std::io::Result<()> {
    match args().nth(1) {
        Some(path) => match File::open(&path) {
            Ok(mut file) => {
                if path.ends_with(".elx") {
                    let mut content = String::new();
                    file.read_to_string(&mut content)
                        .expect("Failed to read content");
                    let xml: ELXML = eurochef_elx::quick_xml::de::from_str(&content)
                        .expect("Failed to read XML");

                    println!("{xml:#?}");
                } else {
                    let endian = if file.read_ne::<u8>().unwrap() == 0x47 {
                        Endian::Big
                    } else {
                        Endian::Little
                    };
                    file.seek(std::io::SeekFrom::Start(0))?;

                    let header = file
                        .read_type::<EXGeoHeader>(endian)
                        .expect("Failed to read header");
                    // println!("Read header: {header:#?}");

                    println!(
                        "Guessed platform: {:?}",
                        Platform::from_flags(header.flags, endian)
                    );

                    for s in &header.spreadsheet_list {
                        if s.m_type != 1 {
                            continue;
                        }

                        file.seek(std::io::SeekFrom::Start(s.common.address as u64))?;
                        let spreadsheet = file
                            .read_type::<EXGeoSpreadSheet>(endian)
                            .expect("Failed to read spreadsheet");

                        for s in spreadsheet.sections {
                            let refpointer =
                                &header.refpointer_list.data[s.refpointer_index as usize];

                            file.seek(std::io::SeekFrom::Start(refpointer.address as u64))?;

                            // Header format is slightly larger for Spyro
                            let text_count = if [213, 236, 221, 240].contains(&header.version) {
                                file.seek(std::io::SeekFrom::Current(20))?;
                                file.read_type::<u32>(endian).unwrap()
                            } else {
                                file.seek(std::io::SeekFrom::Current(4))?; // Skip commonobject
                                file.read_type::<u32>(endian).unwrap()
                            };
                            println!("{} strings @ 0x{:x}", text_count, refpointer.address);
                            for i in 0..text_count {
                                let item = file
                                    .read_type::<EXGeoTextItem>(endian)
                                    .expect("Failed to read textitem");

                                let pos_saved = file.stream_position()?;
                                file.seek(std::io::SeekFrom::Start(item.string.offset_absolute()))?;

                                let string = file
                                    .read_type::<NullWideString>(endian)
                                    .expect("Failed to read text string");

                                print!("{:08x} - {}", item.hashcode, string);
                                if item.sound_hashcode != 0xffffffff {
                                    print!(" (voice hash {:08x})", item.sound_hashcode);
                                }

                                println!();

                                file.seek(std::io::SeekFrom::Start(pos_saved))?;
                            }
                        }
                    }

                    for t in &header.texture_list {
                        file.seek(std::io::SeekFrom::Start(t.common.address as u64))?;
                        let tex = file
                            .read_type::<EXGeoTexture>(endian)
                            .expect("Failed to read basetexture");
                        println!(
                            "0x{:08x} {:?} {}x{}x{}",
                            t.common.hashcode, tex.format, tex.width, tex.height, tex.depth
                        );

                        for (i, frame_offset) in tex.frame_offsets.iter().enumerate() {
                            file.seek(std::io::SeekFrom::Start(frame_offset.offset_absolute()))?;
                            let mut data = vec![
                                0u8;
                                tex.format.calculate_image_size(
                                    tex.width, tex.height, tex.depth, 0
                                )
                            ];

                            file.read(&mut data)?;

                            // I love rust paths /s
                            std::fs::create_dir_all(format!(
                                "extract/{}/",
                                Path::new(&path)
                                    .file_name()
                                    .unwrap()
                                    .to_string_lossy()
                                    .to_string()
                            ))?;

                            let filename = format!(
                                "extract/{}/{:08x}_frame{}.png",
                                Path::new(&path).file_name().unwrap().to_string_lossy(),
                                t.common.hashcode,
                                i
                            );

                            match tex.format {
                                EXTexFmt::Dxt1
                                | EXTexFmt::Dxt1Alpha
                                | EXTexFmt::Dxt2
                                | EXTexFmt::Dxt3
                                | EXTexFmt::Dxt4
                                | EXTexFmt::Dxt5 => {
                                    let bcn = match tex.format {
                                        EXTexFmt::Dxt1 | EXTexFmt::Dxt1Alpha => squish::Format::Bc1,
                                        EXTexFmt::Dxt2 => squish::Format::Bc2,
                                        EXTexFmt::Dxt3 => squish::Format::Bc2,
                                        EXTexFmt::Dxt4 => squish::Format::Bc3,
                                        EXTexFmt::Dxt5 => squish::Format::Bc3,
                                        _ => panic!("Invalid DXT format"),
                                    };

                                    let mut output =
                                        vec![0u8; tex.width as usize * tex.height as usize * 4];
                                    bcn.decompress(
                                        &data,
                                        tex.width as usize,
                                        tex.height as usize,
                                        &mut output,
                                    );

                                    let img = image::RgbaImage::from_raw(
                                        tex.width as u32,
                                        tex.height as u32,
                                        output,
                                    )
                                    .expect("Failed to load decompressed texture data");

                                    img.save(filename).expect("Failed to write image file");
                                }
                                EXTexFmt::A8R8G8B8 => {
                                    let mut output =
                                        vec![0u8; tex.width as usize * tex.height as usize * 4];

                                    // ? Does the `image` crate support RGB565?
                                    for (i, bytes) in data.chunks_exact(4).enumerate() {
                                        output[i * 4] = bytes[3];
                                        output[i * 4 + 1] = bytes[2];
                                        output[i * 4 + 2] = bytes[1];
                                        output[i * 4 + 3] = bytes[0];
                                    }

                                    let img = image::RgbaImage::from_raw(
                                        tex.width as u32,
                                        tex.height as u32,
                                        output,
                                    )
                                    .expect("Failed to load decompressed texture data");

                                    img.save(filename).expect("Failed to write image file");
                                }
                                EXTexFmt::R5G6B5 => {
                                    let mut output =
                                        vec![0u8; tex.width as usize * tex.height as usize * 3];

                                    // ? Does the `image` crate support RGB565?
                                    for (i, byte) in data.chunks_exact(2).enumerate() {
                                        // TODO: Endianness. We're gonna need to move all of this anyways
                                        let value = u16::from_le_bytes([byte[0], byte[1]]);
                                        // RRRRRGGGGGGBBBBB
                                        output[i * 3 + 0] = (value >> 11) as u8 & 0b11111;
                                        output[i * 3 + 1] = (value >> 5) as u8 & 0b111111;
                                        output[i * 3 + 2] = value as u8 & 0b11111;
                                    }

                                    let img = image::RgbImage::from_raw(
                                        tex.width as u32,
                                        tex.height as u32,
                                        output,
                                    )
                                    .expect("Failed to load decompressed texture data");

                                    img.save(filename).expect("Failed to write image file");
                                }
                                _ => {
                                    println!("Cant handle format {:?}", tex.format);
                                }
                            }

                            File::create(format!(
                                "extract/{:08x}_frame{}.bin",
                                t.common.hashcode, i
                            ))?
                            .write_all(&data)?;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to open file: {e}");
            }
        },
        None => {
            println!(
                "No file specified. Usage: {} <file>",
                args().nth(0).unwrap_or("eurochef".to_string())
            );

            return Ok(());
        }
    }

    Ok(())
}
