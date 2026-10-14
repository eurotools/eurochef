//! EngineXT (Disney Universe and later, GEOM version 244+) GEOM chunks have
//! their list-region (everything from byte offset 0x40 onward, i.e.
//! immediately after `auto_include_list` in [`crate::header::EXGeoHeaderXT`])
//! encrypted with a stateful RC4 stream cipher. The fixed header through
//! `auto_include_list` (0x00-0x3F) is plain, unencrypted data.
//!
//! Found by decompiling the `EXGeoFile` load state-machine in `DisneyUPC.exe`
//! (case 3 of the function reached from the string "Unable To Allocate
//! Memory for GeoFile: %s\n") which calls the same RC4 key-scheduling
//! routine used for the FILELIST.BIN name-table cipher, but with a
//! different, literal key embedded directly in the binary.
//!
//! Verified byte-exact against multiple real Xbox 360 Disney Universe DLC
//! `.edb` files: after decrypting, the 14 list descriptors past
//! `auto_include_list` (`group_section_list` through `map_list`) decode to
//! sane `(count, offset)` pairs, and list *data* itself (e.g. `texture_list`
//! entries) decrypts to real, sequential-looking hashcodes.
//!
//! Every v244 GEOM chunk checked so far (Xbox 360 DLC files and a PC
//! `CustomAttributeTemplate`-bearing data/config chunk) does turn out to be
//! encrypted this way -- an earlier note here claiming the PC file was an
//! exception was wrong, caused by decrypting past the real end of the
//! encrypted region (see [`crate::header::EXGeoHeaderXT::read_decrypted`]'s
//! `base_file_size`-vs-`file_size` doc comment for the actual subtlety:
//! some chunks have a large amount of genuinely-plaintext data, like
//! `CustomAttributeTemplate` row data, appended after the encrypted
//! header+list region, and decrypting into that trailing data corrupts it).
//! Still worth a sanity check on any new chunk type -- if the decrypted
//! list descriptors don't look sane (implausible counts/offsets), try
//! [`crate::header::EXGeoHeaderXT::read_plain`] on the raw bytes instead.

/// The literal RC4 key embedded in `DisneyUPC.exe`'s GeoFile loader. 65
/// bytes: 64 visible ASCII characters plus a NUL terminator that's part of
/// the key, not a string terminator to strip (the decompiled call site
/// passes length 0x41 explicitly).
const GEOXT_CIPHER_KEY: &[u8; 65] =
    b"vmfekg53098yt7549gvb14y485gj54gj4590gu65ug4675ghjft2906hj9657h6b\0";

/// Byte offset within a GEOM chunk where the encrypted region begins --
/// immediately after `auto_include_list`'s 8-byte descriptor.
pub const GEOXT_CIPHER_START_OFFSET: usize = 0x40;

struct Rc4State {
    s: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4State {
    fn new(key: &[u8]) -> Self {
        let mut s: [u8; 256] = std::array::from_fn(|i| i as u8);
        let mut j: u8 = 0;
        for i in 0..256u16 {
            let i = i as u8;
            j = j.wrapping_add(s[i as usize]).wrapping_add(key[(i as usize) % key.len()]);
            s.swap(i as usize, j as usize);
        }
        Self { s, i: 0, j: 0 }
    }

    fn crypt_byte(&mut self, in_byte: u8) -> u8 {
        self.i = self.i.wrapping_add(1);
        let si = self.s[self.i as usize];
        self.j = self.j.wrapping_add(si);
        let sj = self.s[self.j as usize];
        self.s.swap(self.i as usize, self.j as usize);
        let k = self.s[si.wrapping_add(sj) as usize];
        in_byte ^ k
    }
}

/// Decrypts `data[GEOXT_CIPHER_START_OFFSET..end]` in place, where `end` is
/// `data.len()` if `end` is `None`, or the given value clamped to
/// `data.len()`. Pass the chunk's own `base_file_size` header field (not
/// `file_size`/`filelen` -- they can differ when a chunk has plaintext data
/// appended after its list region, e.g. `CustomAttributeTemplate` row data)
/// as `end` to match exactly what the game does (decrypt only through the
/// logical end of the encrypted header+list region, not any trailing
/// plaintext data or padding).
///
/// No-op if `data` is shorter than `GEOXT_CIPHER_START_OFFSET`.
pub fn decrypt_geoxt_region(data: &mut [u8], end: Option<usize>) {
    if data.len() <= GEOXT_CIPHER_START_OFFSET {
        return;
    }
    let end = end.unwrap_or(data.len()).min(data.len());
    if end <= GEOXT_CIPHER_START_OFFSET {
        return;
    }
    let mut state = Rc4State::new(GEOXT_CIPHER_KEY);
    for b in &mut data[GEOXT_CIPHER_START_OFFSET..end] {
        *b = state.crypt_byte(*b);
    }
}

/// Returns a decrypted copy of `data`, leaving the original untouched.
pub fn decrypt_geoxt_region_copy(data: &[u8], end: Option<usize>) -> Vec<u8> {
    let mut out = data.to_vec();
    decrypt_geoxt_region(&mut out, end);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rc4_roundtrip() {
        let mut state_enc = Rc4State::new(GEOXT_CIPHER_KEY);
        let plaintext = b"Hello, EngineXT!";
        let ciphertext: Vec<u8> = plaintext.iter().map(|&b| state_enc.crypt_byte(b)).collect();

        let mut state_dec = Rc4State::new(GEOXT_CIPHER_KEY);
        let decrypted: Vec<u8> = ciphertext.iter().map(|&b| state_dec.crypt_byte(b)).collect();

        assert_eq!(decrypted, plaintext);
    }
}
