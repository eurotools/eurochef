use std::io::{Read, Seek};

use anyhow::Context;
use bitflags::bitflags;
use eurochef_edb::{binrw::BinReaderExt, edb::EdbFile, texture::EXGeoTexture, Hashcode};
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

    pub color: [u8; 4],

    pub external_texture: Option<(Hashcode, Hashcode)>,

    pub diagnostics: UXTextureDiagnostics,
}

impl UXGeoTexture {
    pub fn read_all(edb: &mut EdbFile) -> Vec<(usize, IdentifiableResult<Self>)> {
        // ? can this be implemented on-trait???
        let texture_decoder = texture::create_for_platform(edb.platform);
        let mut textures = vec![];
        for (i, t) in edb.header.texture_list.clone().iter().enumerate() {
            textures.push((
                i,
                IdentifiableResult::new(
                    t.common.hashcode,
                    Self::read(t.common.address, edb, &texture_decoder, t.flags),
                ),
            ))
        }

        textures
    }

    /// Read specific hashcodes
    /// Returns an index to enable fast indexing
    pub fn read_hashcodes(
        edb: &mut EdbFile,
        hashcodes: &[Hashcode],
    ) -> Vec<(usize, IdentifiableResult<Self>)> {
        let texture_decoder = texture::create_for_platform(edb.platform);
        let mut textures = vec![];
        for (i, t) in edb
            .header
            .texture_list
            .clone()
            .iter()
            .enumerate()
            .filter(|(_, c)| hashcodes.contains(&c.common.hashcode))
        {
            textures.push((
                i,
                IdentifiableResult::new(
                    t.common.hashcode,
                    Self::read(t.common.address, edb, &texture_decoder, t.flags),
                ),
            ))
        }

        textures
    }

    pub fn read(
        address: u32,
        edb: &mut EdbFile,
        texture_decoder: &Box<dyn TextureDecoder>,
        flags: u32,
    ) -> anyhow::Result<Self> {
        edb.seek(std::io::SeekFrom::Start(address as u64))?;
        let tex = edb
            .read_type_args::<EXGeoTexture>(edb.endian, (edb.header.version, edb.platform))
            .context("Failed to read texture")?;

        if let Some(external_file) = tex.external_file {
            let external_texture = tex.frame_offsets[0].offset_relative() as u32;
            edb.add_reference(external_file, external_texture);
            return Ok(UXGeoTexture {
                width: tex.width,
                height: tex.height,
                depth: tex.depth,
                format_internal: tex.format,
                flags,
                game_flags: tex.game_flags,
                framerate: tex.frame_rate,
                frame_count: 0,
                scroll: [tex.scroll_u, tex.scroll_v],
                frames: vec![],
                color: tex.color,
                diagnostics: Default::default(),
                external_texture: Some((external_file, external_texture)),
            });
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
            color: tex.color,
            diagnostics: Default::default(),
            external_texture: None,
        };

        let mut clut = vec![];
        if let Some(clut_offset) = &tex.clut_offset {
            let clut_size = texture_decoder.get_clut_size(tex.format)?;
            clut.resize(clut_size, 0);

            edb.seek(std::io::SeekFrom::Start(clut_offset.offset_absolute()))?;
            edb.read_exact(&mut clut)?;
        }

        for (i, frame_offset) in tex.frame_offsets.iter().enumerate() {
            edb.seek(std::io::SeekFrom::Start(frame_offset.offset_absolute()))?;
            edb.read_exact(&mut data)
                .context(format!("Failed to read frame {i}"))?;

            if edb.header.version == 156 && clut.len() == 0 {
                let clut_size = texture_decoder.get_clut_size(tex.format)?;
                clut.resize(clut_size, 0);
                edb.read_exact(&mut clut)
                    .context(format!("Failed to read clut for frame {i}"))?;
            }

            texture_decoder
                .decode(
                    &data,
                    if clut.len() > 0 { Some(&clut) } else { None },
                    &mut output,
                    tex.width as u32,
                    tex.height as u32,
                    tex.depth as u32,
                    tex.format,
                    edb.header.version,
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

        texture.calculate_diagnostics();
        Ok(texture)
    }

    pub fn is_valid(&self) -> bool {
        self.flags != u32::MAX && self.game_flags != u16::MAX
    }

    pub fn calculate_diagnostics(&mut self) {
        self.diagnostics = UXTextureDiagnostics::empty();

        // Calculate the average of the first frame
        // let mut avg = [0u64; 4];
        // for v in self.frames[0].chunks_exact(4) {
        //     avg[0] += u64::from(v[0]);
        //     avg[1] += u64::from(v[1]);
        //     avg[2] += u64::from(v[2]);
        //     avg[3] += u64::from(v[3]);
        // }

        // let avg = [
        //     (avg[0] / (self.width as u64 * self.height as u64)) as u8,
        //     (avg[1] / (self.width as u64 * self.height as u64)) as u8,
        //     (avg[2] / (self.width as u64 * self.height as u64)) as u8,
        //     (avg[3] / (self.width as u64 * self.height as u64)) as u8,
        // ];

        // self.diagnostics |= UXTextureDiagnostics::MISMATCHING_AVERAGE;

        if self.frames.is_empty() {
            self.diagnostics |= UXTextureDiagnostics::NO_FRAMES;
        }
    }
}

bitflags! {
    #[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
    pub struct UXTextureDiagnostics: u32 {
        const MISMATCHING_AVERAGE = (1 << 0);
        const NO_FRAMES = (1 << 1);
    }
}

impl UXTextureDiagnostics {
    pub fn to_strings(&self) -> Vec<&'static str> {
        let mut strings = vec![];
        for flag in self.iter() {
            let s = match flag {
                UXTextureDiagnostics::MISMATCHING_AVERAGE => {
                    "Texture data does not match color average"
                }
                UXTextureDiagnostics::NO_FRAMES => "Texture has no frames",
                _ => unreachable!(),
            };

            strings.push(s);
        }

        strings
    }
}
