use anyhow::Context;
use enumn::N;
use image::RgbaImage;

use super::TextureDecoder;

pub struct XenonTextureDecoder;

impl TextureDecoder for XenonTextureDecoder {
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
        _clut: Option<&[u8]>,
        output: &mut RgbaImage,
        width: u32,
        height: u32,
        depth: u32,
        format: u8,
        _version: u32,
    ) -> anyhow::Result<()> {
        let fmt = InternalFormat::n(format)
            .ok_or(anyhow::anyhow!("Invalid texture format 0x{format:x}"))?;

        anyhow::ensure!(output.len() == (width as usize * height as usize * depth as usize) * 4);

        let mut buffer = vec![0u8; output.len()];
        match fmt {
            InternalFormat::Dxt1
            | InternalFormat::Dxt2
            | InternalFormat::Dxt3
            | InternalFormat::Dxt5 => {
                let bcn = match fmt {
                    InternalFormat::Dxt1 | InternalFormat::Dxt2 => squish::Format::Bc1,
                    InternalFormat::Dxt3 => squish::Format::Bc2,
                    // InternalFormat::Dxt4 => squish::Format::Bc3,
                    InternalFormat::Dxt5 => squish::Format::Bc3,
                    _ => panic!("Invalid DXT format"),
                };

                let mut swapped = input.to_vec();
                swap_endianness16(&mut swapped);

                bcn.decompress(&swapped, width as usize, height as usize, &mut buffer);
            }
            InternalFormat::ARGB4 => {
                for (i, bytes) in input.chunks_exact(2).enumerate() {
                    let r = bytes[1] & 0x0f;
                    let g = bytes[1] >> 4;
                    let b = bytes[0] & 0x0f;
                    let a = bytes[0] >> 4;
                    buffer[i * 4 + 0] = (b << 4) | b;
                    buffer[i * 4 + 1] = (g << 4) | g;
                    buffer[i * 4 + 2] = (r << 4) | r;
                    buffer[i * 4 + 3] = (a << 4) | a;
                }
            }
            InternalFormat::RGB565 => {
                for (i, bytes) in input.chunks_exact(2).enumerate() {
                    // TODO: Endianness. We're gonna need to move all of this anyways
                    let r = bytes[1] & 0x1f;
                    let g = (bytes[1] >> 5) | ((bytes[0] & 0x07) << 3);
                    let b = bytes[0] >> 3;
                    buffer[i * 4 + 0] = (b << 3) | (b >> 2);
                    buffer[i * 4 + 1] = (g << 2) | (g >> 4);
                    buffer[i * 4 + 2] = (r << 3) | (r >> 2);
                    buffer[i * 4 + 3] = 255;
                }
            }
            InternalFormat::ARGB8 => {
                for (i, bytes) in input.chunks_exact(4).enumerate() {
                    buffer[i * 4 + 0] = bytes[1];
                    buffer[i * 4 + 1] = bytes[2];
                    buffer[i * 4 + 2] = bytes[3];
                    buffer[i * 4 + 3] = bytes[0];
                }
            }
        }

        // TODO(cohae): This line shouldnt have to exist
        output.copy_from_slice(&buffer);

        Ok(())
    }
}

fn swap_endianness16(buffer: &mut [u8]) {
    for i in (0..buffer.len()).step_by(2) {
        let a = buffer[i];
        buffer[i] = buffer[i + 1];
        buffer[i + 1] = a;
    }
}

#[derive(Debug, N)]
#[repr(u8)]
enum InternalFormat {
    Dxt1 = 0,
    Dxt2 = 1,
    Dxt3 = 3,
    Dxt5 = 5,

    RGB565 = 6,
    ARGB4 = 8,
    ARGB8 = 9,
}

impl InternalFormat {
    pub fn bpp(&self) -> usize {
        match self {
            Self::RGB565 | Self::ARGB4 => 16,
            Self::ARGB8 => 32,
            Self::Dxt1 | Self::Dxt2 => 4,
            Self::Dxt3 | Self::Dxt5 => 8,
        }
    }
}
