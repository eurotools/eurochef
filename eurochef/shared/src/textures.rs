use std::io::{Read, Seek};

use anyhow::Context;
use eurochef_edb::{
    binrw::BinReaderExt, header::EXGeoHeader, texture::EXGeoTexture, versions::Platform,
};
use image::RgbaImage;
use tracing::error;

use crate::platform::texture;

#[derive(Clone)]
pub struct UXGeoTexture {
    pub hashcode: u32,

    pub width: u16,
    pub height: u16,
    pub depth: u16,

    /// Platform-specific format index, only for debugging purposes!
    pub format_internal: u8,
    pub flags: u16,

    /// UV scroll rate, in pixels per second
    pub scroll: [i16; 2],

    /// Framerate in frames per second
    pub framerate: u8,
    pub frame_count: u8,

    /// Decoded RGBA frame data
    pub frames: Vec<Vec<u8>>,
}

impl UXGeoTexture {
    pub fn read_all<R: Read + Seek>(
        header: EXGeoHeader,
        reader: &mut R,
        platform: Platform, // TODO: Shouldn't need to pass this for every function
    ) -> anyhow::Result<Vec<Self>> {
        let endian = platform.endianness();

        // ? can this be implemented on-trait???
        let texture_decoder = texture::create_for_platform(platform);
        let mut textures = vec![];
        let mut data = vec![];
        for t in header.texture_list.data.iter() {
            reader.seek(std::io::SeekFrom::Start(t.common.address as u64))?;

            let tex = reader
                .read_type_args::<EXGeoTexture>(endian, (header.version, platform))
                .context("Failed to read basetexture")?;

            let calculated_size = texture_decoder.get_data_size(
                tex.width as u32,
                tex.height as u32,
                tex.depth as u32,
                tex.format,
            );

            if let Err(e) = calculated_size {
                error!("Failed to extract texture {:x}: {:?}", t.common.hashcode, e);
                continue;
            }

            data.clear();
            data.resize(
                tex.data_size
                    .map(|v| v as usize)
                    .unwrap_or(calculated_size.unwrap()),
                0u8,
            );

            let mut output = RgbaImage::new(tex.width as u32, tex.height as u32);
            let mut texture = UXGeoTexture {
                width: tex.width,
                height: tex.height,
                depth: tex.depth,
                format_internal: tex.format,
                flags: tex.game_flags,
                framerate: tex.frame_rate,
                frame_count: tex.frame_count,
                hashcode: t.common.hashcode,
                scroll: [tex.scroll_u, tex.scroll_v],
                frames: Vec::with_capacity(tex.frame_count as usize),
            };

            for frame_offset in tex.frame_offsets.iter() {
                reader.seek(std::io::SeekFrom::Start(frame_offset.offset_absolute()))?;

                if let Err(e) = reader.read_exact(&mut data) {
                    error!("Failed to read texture {:x}: {}", t.common.hashcode, e);
                }

                if let Err(e) = texture_decoder.decode(
                    &data,
                    &mut output,
                    tex.width as u32,
                    tex.height as u32,
                    tex.depth as u32,
                    tex.format,
                ) {
                    error!("Texture {:08x} failed to decode: {}", t.common.hashcode, e);
                    continue;
                }

                texture.frames.push(output.clone().into_vec());
            }

            textures.push(texture);
        }

        Ok(textures)
    }
}
