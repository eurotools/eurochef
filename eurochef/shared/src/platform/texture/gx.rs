use enumn::N;
use image::RgbaImage;

use super::TextureDecoder;

pub struct GxTextureDecoder;

impl TextureDecoder for GxTextureDecoder {
    fn get_data_size(
        &self,
        width: u32,
        height: u32,
        depth: u32,
        format: u8,
    ) -> anyhow::Result<usize> {
        let bits = (width as usize * height as usize * depth as usize)
            * InternalFormat::from_exformat(format)?.bpp();

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
        // * The data for GC/Wii textures contains an extra header with the internal GX format
        let input_header = &input[0..64];
        let gxformat = input_header[27];

        let input = &input[64..];

        let fmt = InternalFormat::n(gxformat)
            .ok_or(anyhow::anyhow!("Invalid texture format 0x{gxformat:x}"))?;

        anyhow::ensure!(fmt == InternalFormat::from_exformat(format)?);
        anyhow::ensure!(input.len() >= self.get_data_size(width, height, depth, format)?);
        anyhow::ensure!(output.len() == (width as usize * height as usize * depth as usize) * 4);

        // let (rounded_width, rounded_height) = (
        //     (width - 1).next_power_of_two(),
        //     (height - 1).next_power_of_two(),
        // );

        let (blocks_x, blocks_y) = ((width + 3) / 4, (height + 3) / 4);
        let (rounded_width, rounded_height) = (blocks_x * 4, blocks_y * 4);

        let mut buffer = RgbaImage::new(rounded_width, rounded_height);
        match fmt {
            InternalFormat::I4 => {
                let mut index = 0;
                for y in 0..height {
                    for x in 0..width {
                        let v = input[index / 2];
                        let illuminance =
                            convert_4_to_8(if index % 2 == 0 { v >> 4 } else { v & 0x0f });

                        buffer[(x, y)] = [illuminance, illuminance, illuminance, 0xff].into();

                        index += 1;
                    }
                }
            }
            InternalFormat::I8 => {
                let mut index = 0;
                for y in 0..height {
                    for x in 0..width {
                        let illuminance = input[index];

                        buffer[(x, y)] = [illuminance, illuminance, illuminance, 0xff].into();

                        index += 1;
                    }
                }
            }
            InternalFormat::IA4 => {
                let mut index = 0;
                for y in 0..height {
                    for x in 0..width {
                        let v = input[index];
                        let illuminance = convert_4_to_8(v & 0x0f);
                        let alpha = convert_4_to_8(v >> 4);
                        buffer[(x, y)] = [illuminance, illuminance, illuminance, alpha].into();

                        index += 1;
                    }
                }
            }
            InternalFormat::IA8 => {
                let mut index = 0;
                for y in 0..height {
                    for x in 0..width {
                        let alpha = input[index * 2 + 0];
                        let illuminance = input[index * 2 + 1];
                        buffer[(x, y)] = [illuminance, illuminance, illuminance, alpha].into();

                        index += 1;
                    }
                }
            }
            InternalFormat::RGB5A3 => {
                for (i, bytes) in input.chunks_exact(2).enumerate() {
                    // TODO: Endianness. We're gonna need to move all of this anyways
                    let value = u16::from_be_bytes([bytes[0], bytes[1]]);
                    let (x, y) = (i as u32 % width, i as u32 / width);
                    if x >= width || y >= height {
                        break;
                    }

                    buffer[(x, y)] = if (value & 0x8000) != 0 {
                        [
                            convert_5_to_8(((value >> 10) & 0x1f) as u8),
                            convert_5_to_8(((value >> 5) & 0x1f) as u8),
                            convert_5_to_8(((value) & 0x1f) as u8),
                            0xFF,
                        ]
                    } else {
                        [
                            convert_4_to_8(((value >> 8) & 0xf) as u8),
                            convert_4_to_8(((value >> 4) & 0xf) as u8),
                            convert_4_to_8(((value) & 0xf) as u8),
                            convert_3_to_8(((value >> 12) & 0x7) as u8),
                        ]
                    }
                    .into()
                }
            }
            InternalFormat::RGBA8 => {
                let mut input_offset = 0;
                for y in (0..height).step_by(4) {
                    for x in (0..width).step_by(4) {
                        let src1 = &input[input_offset..input_offset + 32];
                        let src2 = &input[input_offset + 32..input_offset + 64];
                        input_offset += 64;
                        for iy in 0..4 {
                            for ix in 0..4 {
                                let offset2 = (y + iy as u32) * width + x + ix as u32;
                                let (bx, by) = (offset2 % width, offset2 / width);

                                let a = src1[iy * 8 + ix * 2];
                                let r = src1[iy * 8 + ix * 2 + 1];
                                let g = src2[iy * 8 + ix * 2];
                                let b = src2[iy * 8 + ix * 2 + 1];
                                buffer[(bx, by)] = [r, g, b, a].into();
                            }
                        }
                    }
                }

                output.copy_from_slice(&buffer);
            }
            InternalFormat::CMPR => {
                if rounded_width != width || rounded_height != height {
                    anyhow::bail!("Odd resolutions on CMPR are not supported yet!");
                }

                let mut index = 0;
                let mut buffer = vec![0u8; output.len()];
                for y in (0..height as usize).step_by(8) {
                    for x in (0..width as usize).step_by(8) {
                        decode_dxt_block(
                            &mut buffer[(y * width as usize + x) * 4..],
                            &input[index..],
                            width,
                        );
                        index += 8;
                        decode_dxt_block(
                            &mut buffer[(y * width as usize + x + 4) * 4..],
                            &input[index..],
                            width,
                        );
                        index += 8;
                        decode_dxt_block(
                            &mut buffer[((y + 4) * width as usize + x) * 4..],
                            &input[index..],
                            width,
                        );
                        index += 8;
                        decode_dxt_block(
                            &mut buffer[((y + 4) * width as usize + x + 4) * 4..],
                            &input[index..],
                            width,
                        );
                        index += 8;
                    }
                }

                output.copy_from_slice(&buffer);
            }
            _ => {
                anyhow::bail!("Unsupported format {:?}", fmt);
            }
        }

        if fmt != InternalFormat::CMPR && fmt != InternalFormat::RGBA8 {
            let mut src_index = 0;
            let (blockw, blockh) = fmt.block_size();
            for y in (0..height as u32).step_by(blockh) {
                for x in (0..width as u32).step_by(blockw) {
                    for by in 0..blockh as u32 {
                        for bx in 0..blockw as u32 {
                            let (sx, sy) = (src_index % width, src_index / width);
                            let pixel = buffer[(sx, sy)];

                            if (x + bx) < width && (y + by) < height {
                                output[(x + bx, y + by)] =
                                    [pixel[0], pixel[1], pixel[2], pixel[3]].into();
                            }

                            src_index += 1;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, N, PartialEq)]
#[repr(u8)]
enum InternalFormat {
    // TODO: We're just using the internal GX formats for this array, that might change once we've discovered all exformat conversions
    I4 = 0,
    I8 = 1,
    IA4 = 2,
    IA8 = 3,
    RGB565 = 4,
    RGB5A3 = 5,
    RGBA8 = 6,
    C4 = 8,
    C8 = 9,
    C14X2 = 10,
    CMPR = 14,
}

impl InternalFormat {
    pub fn bpp(&self) -> usize {
        match self {
            Self::I4 => 4,
            Self::I8 => 8,
            Self::IA4 => 8,
            Self::IA8 => 16,
            Self::RGB565 => 16,
            Self::RGB5A3 => 16,
            Self::RGBA8 => 32,
            Self::C4 => 4,
            Self::C8 => 8,
            Self::C14X2 => 16,
            Self::CMPR => 4,
        }
    }

    pub fn block_size(&self) -> (usize, usize) {
        match self {
            Self::I4 => (8, 8),
            Self::I8 => (8, 4),
            Self::IA4 => (8, 4),
            Self::IA8 => (4, 4),
            Self::RGB565 => (4, 4),
            Self::RGB5A3 => (4, 4),
            Self::RGBA8 => (4, 4),
            Self::C4 => (8, 8),
            Self::C8 => (8, 4),
            Self::C14X2 => (4, 4),
            Self::CMPR => (8, 8),
        }
    }

    pub fn from_exformat(fmt: u8) -> anyhow::Result<Self> {
        Ok(match fmt {
            0 => Self::CMPR,
            1 => Self::RGBA8,
            3 => Self::RGB5A3,
            4 => Self::I4,
            5 => Self::I8,
            7 => Self::IA4,
            8 => Self::IA8,
            _ => {
                anyhow::bail!("Unknown exformat 0x{fmt:x}");
            }
        })
    }
}

// https://github.com/dolphin-emu/dolphin/blob/5d4e4aa561dc7de12cd54f35a98adcccd85bb5d3/Source/Core/VideoCommon/TextureDecoder_Generic.cpp#L155
fn decode_dxt_block(dst: &mut [u8], src: &[u8], pitch: u32) {
    let c1 = u16::from_be_bytes([src[0], src[1]]);
    let c2 = u16::from_be_bytes([src[2], src[3]]);
    let lines = &src[4..8];

    let blue1 = convert_5_to_8((c1 & 0x1F) as u8) as u32;
    let blue2 = convert_5_to_8((c2 & 0x1F) as u8) as u32;
    let green1 = convert_6_to_8(((c1 >> 5) & 0x3F) as u8) as u32;
    let green2 = convert_6_to_8(((c2 >> 5) & 0x3F) as u8) as u32;
    let red1 = convert_5_to_8(((c1 >> 11) & 0x1F) as u8) as u32;
    let red2 = convert_5_to_8(((c2 >> 11) & 0x1F) as u8) as u32;

    let mut colours = [0u32; 4];
    colours[0] = make_rgba(red1, green1, blue1, 255);
    colours[1] = make_rgba(red2, green2, blue2, 255);
    if c1 > c2 {
        colours[2] = make_rgba(
            blend_dxt(red2, red1),
            blend_dxt(green2, green1),
            blend_dxt(blue2, blue1),
            255,
        );
        colours[3] = make_rgba(
            blend_dxt(red1, red2),
            blend_dxt(green1, green2),
            blend_dxt(blue1, blue2),
            255,
        );
    } else {
        colours[2] = make_rgba(
            (red1 + red2) / 2,
            (green1 + green2) / 2,
            (blue1 + blue2) / 2,
            255,
        );
        colours[3] = make_rgba(
            (red1 + red2) / 2,
            (green1 + green2) / 2,
            (blue1 + blue2) / 2,
            0,
        );
    }

    for y in 0..4 {
        let mut val = lines[y];
        for x in 0..4 {
            let offset = (y as usize * pitch as usize + x) * 4;
            dst[offset..offset + 4]
                .copy_from_slice(&colours[((val >> 6) & 3) as usize].to_le_bytes());
            val <<= 2;
        }
    }
}

fn convert_3_to_8(x: u8) -> u8 {
    (x << 5) | (x << 2) | (x >> 1)
}

fn convert_4_to_8(x: u8) -> u8 {
    (x << 4) | x
}

fn convert_5_to_8(x: u8) -> u8 {
    (x << 3) | (x >> 2)
}

fn convert_6_to_8(x: u8) -> u8 {
    (x << 2) | (x >> 4)
}

fn make_rgba(r: u32, g: u32, b: u32, a: u32) -> u32 {
    (a << 24) | (b << 16) | (g << 8) | r
}

fn blend_dxt(x: u32, y: u32) -> u32 {
    (x * 3 + y * 5) >> 3
}
