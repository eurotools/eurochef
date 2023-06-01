use anyhow::Context;
use enumn::N;
use image::{Rgba, RgbaImage};

use super::TextureDecoder;

pub struct Ps2TextureDecoder;

impl TextureDecoder for Ps2TextureDecoder {
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

        Ok(bits / 8)
        // Ok((bits + 7) / 8)
    }

    fn get_clut_size(&self, format: u8) -> anyhow::Result<usize> {
        Ok(InternalFormat::n(format)
            .context(format!("Invalid format 0x{format:x}"))?
            .clut_size())
    }

    fn decode(
        &self,
        input: &[u8],
        clut: Option<&[u8]>,
        output: &mut RgbaImage,
        width: u32,
        height: u32,
        depth: u32,
        format: u8,
        version: u32,
    ) -> anyhow::Result<()> {
        let fmt = InternalFormat::n(format)
            .ok_or(anyhow::anyhow!("Invalid texture format 0x{format:x}"))?;

        anyhow::ensure!(input.len() >= self.get_data_size(width, height, depth, format)?);
        anyhow::ensure!(output.len() == (width as usize * height as usize * depth as usize) * 4);
        anyhow::ensure!(clut.is_some() == (fmt.clut_size() != 0));

        let input = &input[0..self.get_data_size(width, height, depth, format)?];

        match fmt {
            InternalFormat::P256x32 => {
                let clut_swizzled: &[[u8; 4]] = bytemuck::cast_slice(clut.unwrap());
                let mut clut = clut_swizzled.to_vec();
                for i in 0..8 {
                    let offset = i * 32;
                    clut[offset + 8..offset + 16]
                        .copy_from_slice(&clut_swizzled[offset + 16..offset + 24]);
                    clut[offset + 16..offset + 24]
                        .copy_from_slice(&clut_swizzled[offset + 8..offset + 16])
                }

                let input_deswiz = swizzle8_to_32(input, width, height);
                for y in 0..height {
                    for x in 0..width {
                        let byte = input_deswiz[(y * width + x) as usize];
                        let pixel = clut[byte as usize];
                        output[(x, y)] =
                            Rgba([pixel[0], pixel[1], pixel[2], (pixel[3] & 0x7f) * 2]);
                    }
                }
            }
            InternalFormat::P16x32 => {
                let clut: &[[u8; 4]] = bytemuck::cast_slice(clut.unwrap());
                let input_deswiz = swizzle4_to_32(input, width, height, version);
                for y in 0..height {
                    for x in 0..width {
                        let byte = input_deswiz[(y * width + x) as usize];
                        let pixel = clut[byte as usize];
                        output[(x, y)] =
                            Rgba([pixel[0], pixel[1], pixel[2], (pixel[3] & 0x7f) * 2]);
                    }
                }
            }
            InternalFormat::_32BIT => {
                output.copy_from_slice(input);
            }
            _ => {
                anyhow::bail!("Unsupported format {:?}", fmt);
            }
        }

        Ok(())
    }
}

#[derive(Debug, N, PartialEq)]
#[repr(u8)]
enum InternalFormat {
    P16x16 = 0,  // (PSMT4) 16x16-bit palette values
    P16x32 = 1,  // (PSMT4) 16x32-bit palette values
    P256x16 = 2, // (PSMT8) 256x16-bit palette values
    P256x32 = 3, // (PSMT8) 256x32-bit palette values
    _16BIT = 4,  // PSMCT16S
    _32BIT = 5,  // PSMCT32
}

impl InternalFormat {
    pub fn bpp(&self) -> usize {
        match self {
            InternalFormat::P16x16 => 4,
            InternalFormat::P16x32 => 4,
            InternalFormat::P256x16 => 8,
            InternalFormat::P256x32 => 8,
            InternalFormat::_16BIT => 16,
            InternalFormat::_32BIT => 32,
        }
    }

    pub fn clut_size(&self) -> usize {
        match self {
            InternalFormat::P16x16 => 16 * 2,
            InternalFormat::P16x32 => 16 * 4,
            InternalFormat::P256x16 => 256 * 2,
            InternalFormat::P256x32 => 256 * 4,
            InternalFormat::_16BIT => 0,
            InternalFormat::_32BIT => 0,
        }
    }
}

fn swizzle4_to_32(input: &[u8], width: u32, height: u32, version: u32) -> Vec<u8> {
    const INTERLACE_MATRIX: [u8; 8] = [0x00, 0x10, 0x02, 0x12, 0x11, 0x01, 0x13, 0x03];

    const MATRIX: [i32; 4] = [0, 1, -1, 0];
    const TILE_MATRIX: [i32; 2] = [4, -4];

    let mut pixels = vec![0u8; (width * height) as usize];
    let mut output = vec![0u8; (width * height) as usize];

    let mut d = 0usize;
    let mut s = 0usize;

    for _ in 0..height {
        for _ in 0..(width >> 1) {
            {
                let p = input[s];
                s += 1;

                pixels[d] = p & 0xF;
                d += 1;
                pixels[d] = p >> 4;
                d += 1;
            }
        }
    }

    if version == 163 {
        return pixels.to_vec();
    }

    for y in 0..height {
        for x in 0..width {
            let odd_row = (y & 1) != 0;

            let num1 = (y / 4) & 1;
            let num2 = (x / 4) & 1;
            let num3 = y % 4;

            let mut num4 = (x / 4) % 4;

            if odd_row {
                num4 += 4;
            }

            let num5 = (x * 4) % 16;
            let num6 = (x / 16) * 32;

            let num7 = if odd_row { (y - 1) * width } else { y * width };

            let xx = x as i32 + num1 as i32 * TILE_MATRIX[num2 as usize];
            let yy = y as i32 + MATRIX[num3 as usize];

            let i = INTERLACE_MATRIX[num4 as usize] as u32 + num5 + num6 + num7;
            let j = yy as usize * width as usize + xx as usize;

            output[j] = if i < pixels.len() as u32 {
                pixels[i as usize]
            } else {
                pixels[pixels.len() - 1]
            };
        }
    }

    output
}

fn swizzle8_to_32(input: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut output = vec![0u8; input.len()];

    for y in 0..height {
        for x in 0..width {
            let block_location = (y & (!0xF)) * width + (x & (!0xF)) * 2;
            let swap_selector = (((y + 2) >> 2) & 0x1) * 4;
            let pos_y = (((y & (!3)) >> 1) + (y & 1)) & 0x7;
            let column_location = pos_y * width * 2 + ((x + swap_selector) & 0x7) * 4;

            let byte_num = ((y >> 1) & 1) + ((x >> 2) & 2); // 0, 1, 2, 3

            let byte = if (block_location + column_location + byte_num) as usize >= input.len() {
                input[input.len() - 1]
            } else {
                input[(block_location + column_location + byte_num) as usize]
            };

            output[(y * width + x) as usize] = byte;
        }
    }

    output
}
