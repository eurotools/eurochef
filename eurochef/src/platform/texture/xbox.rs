use enumn::N;

use super::TextureDecoder;

pub struct XboxTextureDecoder;

impl TextureDecoder for XboxTextureDecoder {
    fn get_data_size(&self, width: u16, height: u16, depth: u16, format: u8) -> Option<usize> {
        let bits =
            (width as usize * height as usize * depth as usize) * InternalFormat::n(format)?.bpp();

        Some((bits + 7) / 8)
    }

    fn decode(
        &self,
        input: &[u8],
        output: &mut [u8],
        width: u16,
        height: u16,
        depth: u16,
        format: u8,
    ) -> anyhow::Result<()> {
        let fmt = InternalFormat::n(format)
            .ok_or(anyhow::anyhow!("Invalid texture format 0x{format:x}"))?;

        anyhow::ensure!(output.len() == (width as usize * height as usize * depth as usize) * 4);

        let mut buffer = vec![0u8; output.len()];
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

                bcn.decompress(input, width as usize, height as usize, &mut buffer);
            }
            InternalFormat::ARGB8 | InternalFormat::ARGB8Linear => {
                for (i, bytes) in input.chunks_exact(4).enumerate() {
                    buffer[i * 4 + 0] = bytes[0];
                    buffer[i * 4 + 1] = bytes[1];
                    buffer[i * 4 + 2] = bytes[2];
                    buffer[i * 4 + 3] = bytes[3];
                }
            }
            InternalFormat::ARGB4 => {
                for (i, bytes) in input.chunks_exact(2).enumerate() {
                    let b = bytes[0] & 0x0f;
                    let g = bytes[0] >> 4;
                    let r = bytes[1] & 0x0f;
                    let a = bytes[1] >> 4;
                    buffer[i * 4 + 0] = (b << 4) | b;
                    buffer[i * 4 + 1] = (g << 4) | g;
                    buffer[i * 4 + 2] = (r << 4) | r;
                    buffer[i * 4 + 3] = (a << 4) | a;
                }
            }
            InternalFormat::RGB565 => {
                for (i, bytes) in input.chunks_exact(2).enumerate() {
                    // TODO: Endianness. We're gonna need to move all of this anyways
                    let b = bytes[0] & 0x1f;
                    let g = (bytes[0] >> 5) | ((bytes[1] & 0x07) << 3);
                    let r = bytes[1] >> 3;
                    buffer[i * 4 + 0] = (b << 3) | (b >> 2);
                    buffer[i * 4 + 1] = (g << 2) | (g >> 4);
                    buffer[i * 4 + 2] = (r << 3) | (r >> 2);
                    buffer[i * 4 + 3] = 255;
                }
            }
            InternalFormat::ARGB1555 | InternalFormat::XRGB1555 => {
                for (i, byte) in input.chunks_exact(2).enumerate() {
                    // TODO: Endianness. We're gonna need to move all of this anyways
                    let b = byte[0] & 0x1f;
                    let g = (byte[0] >> 5) | ((byte[1] & 0x03) << 3);
                    let r = (byte[1] & 0x7c) >> 2;
                    let a = byte[1] >> 7;

                    buffer[i * 4 + 0] = r as u8 * 8;
                    buffer[i * 4 + 1] = g as u8 * 8;
                    buffer[i * 4 + 2] = b as u8 * 8;
                    buffer[i * 4 + 3] = a as u8 * 255;
                }
            }
            _ => {
                anyhow::bail!("Unsupported format {:?}", fmt);
            }
        }

        // TODO: Using an intermediate buffer is inefficient, we should just swizzle when decoding.
        if fmt.is_swizzled() {
            for y in 0..height {
                for x in 0..width {
                    let load_offset =
                        deswizzle(x as u32, y as u32, width as u32, height as u32) as usize;
                    let store_offset = y as usize * width as usize + x as usize;

                    output[store_offset * 4..store_offset * 4 + 4]
                        .copy_from_slice(&buffer[load_offset * 4..load_offset * 4 + 4])
                }
            }
        } else {
            output.copy_from_slice(&buffer)
        }

        Ok(())
    }
}

// Implementation based on https://registry.khronos.org/DataFormat/specs/1.3/dataformat.1.3.inline.html#_more_complex_2_d_texel_addressing
fn deswizzle(x: u32, y: u32, width: u32, height: u32) -> u32 {
    let min_dim = if width <= height { width } else { height };
    let mut offset = 0;
    let mut shift = 0;

    let mut mask = 1;
    while mask < min_dim {
        offset |= (((y & mask) << 1) | (x & mask)) << shift;
        shift += 1;
        mask <<= 1;
    }

    // At least one of width and height will  have run out of most-significant bits
    offset |= ((x | y) >> shift) << (shift * 2);
    return offset;
}

#[derive(Debug, N)]
#[repr(u8)]
enum InternalFormat {
    RGB565 = 0,
    XRGB1555 = 1,
    Dxt1 = 2,
    Dxt1Alpha = 3,
    Dxt2 = 4,
    ARGB4 = 5,
    ARGB8 = 6,
    P8 = 7,
    ARGB1555 = 8,
    ARGB8Linear = 9,
    Dxt3 = 10,
    Dxt4 = 11,
    Dxt5 = 12,
}

impl InternalFormat {
    pub fn bpp(&self) -> usize {
        match self {
            Self::RGB565 | Self::ARGB1555 | Self::XRGB1555 => 16,
            Self::ARGB4 => 16,
            Self::ARGB8 | Self::ARGB8Linear => 32,

            Self::Dxt1 | Self::Dxt1Alpha => 4,
            Self::Dxt2 => 8,
            Self::Dxt3 => 8,
            Self::Dxt4 => 8,
            Self::Dxt5 => 8,
            Self::P8 => 8,
        }
    }

    pub fn is_swizzled(&self) -> bool {
        match self {
            Self::RGB565 | Self::ARGB4 | Self::ARGB8 | Self::P8 | Self::ARGB1555 => true,
            Self::XRGB1555 => false,
            Self::ARGB8Linear => false,
            Self::Dxt1 | Self::Dxt1Alpha | Self::Dxt2 | Self::Dxt3 | Self::Dxt4 | Self::Dxt5 => {
                false
            }
        }
    }
}
