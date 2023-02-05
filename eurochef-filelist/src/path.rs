pub fn unscramble_filename_v7(file_index: u32, bytes: &mut [u8]) {
    for i in 0..bytes.len() {
        bytes[i] = (bytes[i] as u32)
            .wrapping_add(0x16)
            .wrapping_sub(file_index)
            .wrapping_sub(i as u32) as u8;

        if bytes[i] == 0 {
            break;
        }
    }
}

pub fn scramble_filename_v7(file_index: u32, bytes: &mut [u8]) {
    for i in 0..bytes.len() {
        bytes[i] = (bytes[i] as u32)
            .wrapping_sub(0x16)
            .wrapping_add(file_index)
            .wrapping_add(i as u32) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unscramble_path() {
        let mut input: [u8; 39] = [
            0xC3, 0x86, 0xA9, 0xB5, 0xB5, 0xBF, 0xC3, 0xB5, 0xB8, 0xB0, 0xB7, 0xBF, 0xC5, 0xB9,
            0xCB, 0xD3, 0xB7, 0xBB, 0xBF, 0xC7, 0xCD, 0xBF, 0xD1, 0xC5, 0xBF, 0xD1, 0xDC, 0xC5,
            0xD0, 0xD6, 0xDD, 0xD9, 0xCD, 0xD6, 0x9B, 0xD3, 0xD3, 0xD2, 0x71,
        ];

        unscramble_filename_v7(353, &mut input);

        assert_eq!(&input, b"x:\\gforce\\binary\\_bin_pc\\mw_intobj.edb\0")
    }

    #[test]
    fn scramble_path() {
        let mut input = b"x:\\gforce\\binary\\_bin_pc\\as_fplay.edb\0".to_vec();
        let output: [u8; 38] = [
            0x62, 0x25, 0x48, 0x54, 0x54, 0x5E, 0x62, 0x54, 0x57, 0x4F, 0x56, 0x5E, 0x64, 0x58,
            0x6A, 0x72, 0x56, 0x5A, 0x5E, 0x66, 0x6C, 0x5E, 0x70, 0x64, 0x5E, 0x64, 0x77, 0x64,
            0x6C, 0x77, 0x74, 0x6A, 0x83, 0x39, 0x71, 0x71, 0x70, 0x0F,
        ];

        scramble_filename_v7(0, &mut input);

        assert_eq!(&input, &output)
    }
}
