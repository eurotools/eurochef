use std::io::{Read, Seek};

use anyhow::Context;
use eurochef_edb::{
    binrw::BinReaderExt, header::EXGeoHeader, texture::EXGeoTexture, versions::Platform,
};
use image::RgbaImage;
use tracing::{error, warn};

use crate::platform::texture;

#[derive(Clone)]
pub struct UXGeoTexture {
    pub hashcode: u32,

    pub width: u16,
    pub height: u16,
    pub depth: u16,

    /// Platform-specific format index, only for debugging purposes!
    pub format_internal: u8,

    pub flags: u32,
    pub game_flags: u16,

    /// UV scroll rate, in pixels per second
    pub scroll: [i16; 2],

    /// Framerate in frames per second
    pub framerate: u8,
    pub frame_count: u8,

    /// Decoded RGBA frame data
    pub frames: Vec<Vec<u8>>,
}

// TODO(cohae): read() for a single texture (going to be used for reference textures)
impl UXGeoTexture {
    pub fn read_all<R: Read + Seek>(
        header: &EXGeoHeader,
        reader: &mut R,
        platform: Platform, // TODO: Shouldn't need to pass this for every function
    ) -> anyhow::Result<Vec<Self>> {
        let endian = platform.endianness();

        // ? can this be implemented on-trait???
        let texture_decoder = texture::create_for_platform(platform);
        let mut textures = vec![];
        let mut data = vec![];
        for t in header.texture_list.iter() {
            reader.seek(std::io::SeekFrom::Start(t.common.address as u64))?;

            let tex = reader
                .read_type_args::<EXGeoTexture>(endian, (header.version, platform))
                .context("Failed to read texture")?;

            // cohae: This is a bit of a difficult one. We might need an alternative to return these references somehow (enum with variant?), as we cannot open other files from this context.
            if let Some(external_file) = tex.external_file {
                let external_texture = tex.frame_offsets[0].offset_absolute();
                warn!("Texture is an external reference, skipping (texture 0x{external_texture:x} from file 0x{external_file:x})");
                textures.push(Self {
                    hashcode: t.common.hashcode,
                    ..Default::default()
                });
                continue;
            }

            let calculated_size = match texture_decoder.get_data_size(
                tex.width as u32,
                tex.height as u32,
                tex.depth as u32,
                tex.format,
            ) {
                Ok(cs) => cs,
                Err(e) => {
                    error!("Failed to extract texture {:x}: {e}", t.common.hashcode);
                    textures.push(Self {
                        hashcode: t.common.hashcode,
                        ..Default::default()
                    });
                    continue;
                }
            };

            let data_size = tex.data_size.map(|v| v as usize).unwrap_or(calculated_size);

            if data_size == 0 {
                error!(
                    "Texture has no data? (calculated={}, data_size={:?})",
                    calculated_size, tex.data_size
                );
                textures.push(Self {
                    hashcode: t.common.hashcode,
                    ..Default::default()
                });
                continue;
            }

            data.clear();
            data.resize(data_size, 0u8);

            let mut output = RgbaImage::new(tex.width as u32, tex.height as u32);
            let mut texture = UXGeoTexture {
                width: tex.width,
                height: tex.height,
                depth: tex.depth,
                format_internal: tex.format,
                flags: t.flags,
                game_flags: tex.game_flags,
                framerate: tex.frame_rate,
                frame_count: tex.frame_count,
                hashcode: t.common.hashcode,
                scroll: [tex.scroll_u, tex.scroll_v],
                frames: Vec::with_capacity(tex.frame_count as usize),
            };

            for (i, frame_offset) in tex.frame_offsets.iter().enumerate() {
                reader.seek(std::io::SeekFrom::Start(frame_offset.offset_absolute()))?;

                if let Err(e) = reader.read_exact(&mut data) {
                    error!("Failed to read texture {:x}.{i}: {e}", t.common.hashcode);
                    textures.push(Self {
                        hashcode: t.common.hashcode,
                        ..Default::default()
                    });
                    continue;
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
                    textures.push(Self {
                        hashcode: t.common.hashcode,
                        ..Default::default()
                    });
                    continue;
                }

                if output.len() != (t.width as usize * t.height as usize) * 4 {
                    error!(
                        "Texture {:08x}.{i} has mismatching data length (expected {}, got {})",
                        t.common.hashcode,
                        (t.width as usize * t.height as usize) * 4,
                        output.len()
                    );
                    textures.push(Self {
                        hashcode: t.common.hashcode,
                        ..Default::default()
                    });

                    continue;
                }

                texture.frames.push(output.clone().into_vec());
            }

            textures.push(texture);
        }

        Ok(textures)
    }

    pub fn is_valid(&self) -> bool {
        self.flags != u32::MAX && self.game_flags != u16::MAX
    }
}

impl Default for UXGeoTexture {
    fn default() -> Self {
        Self {
            hashcode: u32::MAX,
            width: 2,
            height: 2,
            depth: 1,
            format_internal: 0,
            flags: u32::MAX,
            game_flags: u16::MAX,
            scroll: [0, 0],
            framerate: 0,
            frame_count: 1,
            frames: vec![vec![
                255, 0, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 255, 255,
            ]],
        }
    }
}
