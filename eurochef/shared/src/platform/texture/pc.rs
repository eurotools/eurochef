use anyhow::Context;
use enumn::N;
use image::RgbaImage;

use super::TextureDecoder;

pub struct PcTextureDecoder;

impl TextureDecoder for PcTextureDecoder {
    fn get_data_size(
        &self,
        width: u32,
        height: u32,
        depth: u32,
        format: u8,
    ) -> anyhow::Result<usize> {
        let bits = (width as usize * height as usize * depth as usize)
            * InternalFormat::n(format)
                .context(format!("Invalid format 0x{format:x}"))?
                .bpp();

        Ok((bits + 7) / 8)
    }

    fn decode(
        &self,
        input: &[u8],
        output: &mut RgbaImage,
        width: u32,
        height: u32,
        depth: u32,
        format: u8,
    ) -> anyhow::Result<()> {
        let fmt = InternalFormat::n(format)
            .ok_or(anyhow::anyhow!("Invalid texture format 0x{format:x}"))?;

        anyhow::ensure!(output.len() == (width as usize * height as usize * depth as usize) * 4);

        match fmt {
            InternalFormat::Dxt1
            | InternalFormat::Dxt1Alpha
            | InternalFormat::Dxt2
            | InternalFormat::Dxt3
            | InternalFormat::Dxt4
            | InternalFormat::Dxt5 => {
                let bcn = match fmt {
                    InternalFormat::Dxt1 | InternalFormat::Dxt1Alpha => squish::Format::Bc1,
                    InternalFormat::Dxt2 => squish::Format::Bc2,
                    InternalFormat::Dxt3 => squish::Format::Bc2,
                    InternalFormat::Dxt4 => squish::Format::Bc3,
                    InternalFormat::Dxt5 => squish::Format::Bc3,
                    _ => panic!("Invalid DXT format"),
                };

                bcn.decompress(input, width as usize, height as usize, output);
            }
            InternalFormat::ARGB8 => {
                for (i, bytes) in input.chunks_exact(4).enumerate() {
                    let (x, y) = (i as u32 % width, i as u32 / width);
                    output[(x, y)] = [bytes[2], bytes[1], bytes[0], bytes[3]].into();
                }
            }
            InternalFormat::RGB565 => {
                for (i, byte) in input.chunks_exact(2).enumerate() {
                    // TODO: Endianness. We're gonna need to move all of this anyways
                    let value = u16::from_le_bytes([byte[0], byte[1]]);
                    let (x, y) = (i as u32 % width, i as u32 / width);
                    // RRRRRGGGGGGBBBBB
                    output[(x, y)] = [
                        ((value >> 11) as u8 & 0b11111) * 8,
                        ((value >> 5) as u8 & 0b111111) * 4,
                        (value as u8 & 0b11111) * 8,
                        0xff,
                    ]
                    .into();
                }
            }
            InternalFormat::ARGB1555 => {
                for (i, byte) in input.chunks_exact(2).enumerate() {
                    // TODO: Endianness. We're gonna need to move all of this anyways
                    let (x, y) = (i as u32 % width, i as u32 / width);
                    let b = byte[0] & 0x1f;
                    let g = (byte[0] >> 5) | ((byte[1] & 0x03) << 3);
                    let r = (byte[1] & 0x7c) >> 2;
                    let a = byte[1] >> 7;

                    output[(x, y)] = [r as u8 * 8, g as u8 * 8, b as u8 * 8, a as u8 * 255].into();
                }
            }
            _ => {
                anyhow::bail!("Unsupported format {:?}", fmt);
            }
        }

        Ok(())
    }
}

#[derive(Debug, N)]
#[repr(u8)]
enum InternalFormat {
    RGB565 = 0,
    ARGB1555 = 1,
    Dxt1 = 2,
    Dxt1Alpha = 3,
    Dxt2 = 4,
    ARGB4 = 5,
    ARGB8 = 6,
    Dxt3 = 7,
    Dxt4 = 8,
    Dxt5 = 9,
}

impl InternalFormat {
    pub fn bpp(&self) -> usize {
        match self {
            Self::RGB565 | Self::ARGB1555 => 16,
            Self::ARGB4 => 16,
            Self::ARGB8 => 32,

            Self::Dxt1 | Self::Dxt1Alpha => 4,
            Self::Dxt2 => 8,
            Self::Dxt3 => 8,
            Self::Dxt4 => 8,
            Self::Dxt5 => 8,
        }
    }
}
