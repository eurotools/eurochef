use std::io::{Read, Seek};

use anyhow::Context;
use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    header::EXGeoHeader,
    texture::EXGeoTexture,
    versions::Platform,
};
use image::RgbaImage;

use crate::{
    platform::texture::{self, TextureDecoder},
    IdentifiableResult,
};

#[derive(Clone)]
pub struct UXGeoTexture {
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

impl UXGeoTexture {
    pub fn read_all<R: Read + Seek>(
        header: &EXGeoHeader,
        reader: &mut R,
        platform: Platform, // TODO: Shouldn't need to pass this for every function
    ) -> Vec<IdentifiableResult<Self>> {
        let endian = platform.endianness();

        // ? can this be implemented on-trait???
        let texture_decoder = texture::create_for_platform(platform);
        let mut textures = vec![];
        for t in header.texture_list.iter() {
            textures.push(IdentifiableResult::new(
                t.common.hashcode,
                Self::read(
                    t.common.address,
                    header,
                    endian,
                    platform,
                    reader,
                    &texture_decoder,
                    t.flags,
                ),
            ))
        }

        textures
    }

    pub fn read<R: Read + Seek>(
        address: u32,
        header: &EXGeoHeader,
        endian: Endian,
        platform: Platform,
        reader: &mut R,
        texture_decoder: &Box<dyn TextureDecoder>,
        flags: u32,
    ) -> anyhow::Result<Self> {
        reader.seek(std::io::SeekFrom::Start(address as u64))?;
        let tex = reader
            .read_type_args::<EXGeoTexture>(endian, (header.version, platform))
            .context("Failed to read texture")?;

        // cohae: This is a bit of a difficult one. We might need an alternative to return these references somehow (enum with variant?), as we cannot open other files from this context.
        if let Some(external_file) = tex.external_file {
            let external_texture = tex.frame_offsets[0].offset_absolute();
            return Err(anyhow::anyhow!(
                "Texture is an external reference, skipping (texture 0x{external_texture:x} from file 0x{external_file:x})",
            ));
        }

        let calculated_size = texture_decoder
            .get_data_size(
                tex.width as u32,
                tex.height as u32,
                tex.depth as u32,
                tex.format,
            )
            .context("Failed to get data size")?;

        let data_size = tex.data_size.map(|v| v as usize).unwrap_or(calculated_size);

        if data_size == 0 {
            return Err(anyhow::anyhow!(
                "Texture has no data? (calculated={}, data_size={:?})",
                calculated_size,
                tex.data_size
            ));
        }

        let mut data = vec![0u8; data_size];
        let mut output = RgbaImage::new(tex.width as u32, tex.height as u32);
        let mut texture = UXGeoTexture {
            width: tex.width,
            height: tex.height,
            depth: tex.depth,
            format_internal: tex.format,
            flags,
            game_flags: tex.game_flags,
            framerate: tex.frame_rate,
            frame_count: tex.frame_count,
            scroll: [tex.scroll_u, tex.scroll_v],
            frames: Vec::with_capacity(tex.frame_count as usize),
        };

        let mut clut = vec![];
        if let Some(clut_offset) = &tex.clut_offset {
            let clut_size = texture_decoder.get_clut_size(tex.format)?;
            clut.resize(clut_size, 0);

            reader.seek(std::io::SeekFrom::Start(clut_offset.offset_absolute()))?;
            reader.read_exact(&mut clut)?;
        }

        for (i, frame_offset) in tex.frame_offsets.iter().enumerate() {
            reader.seek(std::io::SeekFrom::Start(frame_offset.offset_absolute()))?;
            reader
                .read_exact(&mut data)
                .context(format!("Failed to read frame {i}"))?;

            texture_decoder
                .decode(
                    &data,
                    if clut.len() > 0 { Some(&clut) } else { None },
                    &mut output,
                    tex.width as u32,
                    tex.height as u32,
                    tex.depth as u32,
                    tex.format,
                    header.version,
                )
                .context("Failed to decode texture")?;

            if output.len() != (tex.width as usize * tex.height as usize) * 4 {
                return Err(anyhow::anyhow!(
                    "Texture has mismatching data length (expected {}, got {})",
                    (tex.width as usize * tex.height as usize) * 4,
                    output.len()
                ));
            }

            texture.frames.push(output.clone().into_vec());
        }

        Ok(texture)
    }

    pub fn is_valid(&self) -> bool {
        self.flags != u32::MAX && self.game_flags != u16::MAX
    }
}
