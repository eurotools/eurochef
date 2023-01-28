use std::{
    env::args,
    fs::File,
    io::{Read, Seek, Write},
};

use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    header::EXGeoHeader,
    texture::{EXGeoTexture, EXTexFmt},
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
                    println!("Read header: {header:#?}");

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

                            let filename =
                                format!("extract/{:08x}_frame{}.png", t.common.hashcode, i);

                            match tex.format {
                                EXTexFmt::Dxt1
                                | EXTexFmt::Dxt1Alt
                                | EXTexFmt::Dxt2
                                | EXTexFmt::Dxt3
                                | EXTexFmt::Dxt4
                                | EXTexFmt::Dxt5 => {
                                    let bcn = match tex.format {
                                        EXTexFmt::Dxt1 | EXTexFmt::Dxt1Alt => squish::Format::Bc1,
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
                                        let r = (value & 0b11111000) >> 3;
                                        let g =
                                            (value & 0b00000111) << 3 | (value & 0b11100000) >> 5;
                                        let b = value & 0b00011111;

                                        output[i * 2] = ((r << 3) | (r >> 2)) as u8;
                                        output[i * 2 + 1] = ((g << 2) | (g >> 4)) as u8;
                                        output[i * 2 + 2] = ((b << 3) | (b >> 2)) as u8;
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
