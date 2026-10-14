use std::io::{Seek, SeekFrom};

use binrw::binrw;

use crate::{
    array::{EXGeoAbsArray, EXGeoCommonArrayElement, EXGeoHashArray, EXRelArray},
    common::{
        EXGeoAnimHeader, EXGeoAnimModeHeader, EXGeoAnimSetHeader, EXGeoAnimSkinHeader,
        EXGeoEntityHeader, EXGeoSpreadSheetHeader,
    },
    texture::EXGeoTextureHeader,
    // versions::{EDB_VERSION_BOND, EDB_VERSION_GFORCE, EDB_VERSION_ICEAGE3},
};

pub type EXGeoMapHeader = EXGeoCommonArrayElement;
pub type EXGeoParticleHeader = EXGeoCommonArrayElement;
pub type EXGeoRefPointerHeader = EXGeoCommonArrayElement;
pub type EXGeoAnimScriptHeader = EXGeoCommonArrayElement;
pub type EXGeoSwooshHeader = EXGeoCommonArrayElement;
pub type EXGeoFontHeader = EXGeoCommonArrayElement;
pub type EXGeoMaterialHeader = EXGeoCommonArrayElement;

#[binrw]
#[brw(magic = 0x47454F4Du32)]
#[derive(Debug, Clone)]
pub struct EXGeoHeader {
    pub hashcode: u32,

    // ! Disney Universe (and presumably later EngineXT titles) is the first
    // ! seen title where this slot isn't a plain u32 - the upper 16 bits are
    // ! a separate field (seen value: 2, meaning unknown) and the actual
    // ! version is only the lower 16 bits. Every older title happens to have
    // ! 0 in the upper half, so reading the whole thing as one u32 silently
    // ! worked for them by coincidence.
    pub version_unk: u16,

    #[brw(assert(version.ge(&182) || version.le(&335), "Unsupported version {version}"))]
    pub version: u16,

    pub flags: u32,
    pub time: u32,
    pub file_size: u32,
    pub base_file_size: u32,

    pub platform_versions: [u32; 6], // 0x1C

    #[brw(seek_before = SeekFrom::Start(if version.lt(&248) { 0x54 } else { 0x40 } ))]
    pub section_list: EXGeoHashArray<()>, // 0x40
    pub refpointer_list: EXGeoHashArray<EXGeoRefPointerHeader>,
    #[br(args(version as u32))]
    pub entity_list: EXGeoHashArray<EXGeoEntityHeader>, // 0x50
    pub anim_list: EXGeoHashArray<EXGeoAnimHeader>,
    #[br(args(version as u32))]
    pub animskin_list: EXGeoHashArray<EXGeoAnimSkinHeader>, // 0x60
    pub animscript_list: EXGeoHashArray<EXGeoAnimScriptHeader>,
    pub map_list: EXGeoHashArray<EXGeoMapHeader>, // 0x70
    pub animmode_list: EXGeoHashArray<EXGeoAnimModeHeader>,
    pub animset_list: EXGeoHashArray<EXGeoAnimSetHeader>, // 0x80
    pub particle_list: EXGeoHashArray<EXGeoParticleHeader>,
    pub swoosh_list: EXGeoHashArray<EXGeoSwooshHeader>, // 0x90
    pub spreadsheet_list: EXGeoHashArray<EXGeoSpreadSheetHeader>,
    pub font_list: EXGeoHashArray<EXGeoFontHeader>, // 0xa0

    #[brw(if(version.ge(&248)))]
    pub forcefeedback_list: EXGeoHashArray<()>,
    #[brw(if(version.ge(&248)))]
    pub material_list: EXGeoHashArray<EXGeoMaterialHeader>, // 0xb0

    // ! Spyro Hack
    #[brw(if(version.eq(&240)))]
    _spyrohack: u64,

    pub texture_list: EXGeoHashArray<EXGeoTextureHeader>,

    pub unk_c0: EXRelArray<()>,
}

// structure_size_tests!(EXGeoHeader = 936);

/// EngineXT (Disney Universe and later - confirmed shipped version 244, and
/// version 300 seen in an Apr 2010 prototype build) GEOM chunk header. This is
/// a different, later redesign from EXGeoHeader above - not just a few extra
/// fields, a genuinely different list set and a different (and simpler) list
/// descriptor shape (plain count+absolute-offset, no self-relative pointer
/// encoding, no hash_size field).
///
/// Recovered by decompiling CBasePlatform::WriteMainGeoHeader from
/// ResourceBuilderXT.exe (the real Eurocom devkit tool that writes these
/// files) with its matching PDB, and validated byte-for-byte against a real
/// sample .edb file with zero discrepancies (filelen field matched the
/// sample's actual file size exactly).
#[binrw]
#[brw(magic = 0x47454F4Du32)]
#[derive(Debug, Clone)]
pub struct EXGeoHeaderXT {
    pub hashcode: u32,
    /// Same packed-u16 quirk as `EXGeoHeader.version_unk`/`version` above --
    /// the upper 16 bits are a separate field (unknown purpose, seen value
    /// 2), the real version is the lower 16 bits only.
    pub version_unk: u16,
    pub version: u16,
    pub flags: u32,
    pub time: u32,
    /// Real total file size - matches the on-disk file length exactly.
    pub file_size: u32,
    pub base_file_size: u32,
    /// Literal ASCII "NUID" tag, always present as a constant, not per-file data.
    pub nuid_tag: u32,
    pub unk_checksum_related: u32,
    pub checksum: u32,
    /// Absolute offset to the debug section, deferred-patched at build time.
    pub debug_section_offset: u32,
    /// Duplicate of file_size.
    pub file_end: u32,
    /// Always 0 on disk - runtime-only pointer placeholder.
    pub debug_file_ptr: u32,
    /// Always 0 on disk - runtime-only pointer placeholder (the in-memory EXGeoFile object).
    pub geo_file_ptr: u32,

    pub auto_include_list: EXGeoAbsArray<()>,
    pub group_section_list: EXGeoAbsArray<()>,
    pub custom_attribute_template_list: EXGeoAbsArray<()>,
    pub shader_list: EXGeoAbsArray<()>,
    pub texture_list: EXGeoAbsArray<()>,
    pub material_list: EXGeoAbsArray<()>,
    pub entity_list: EXGeoAbsArray<()>,
    pub skeleton_list: EXGeoAbsArray<()>,
    pub anim_sequence_list: EXGeoAbsArray<()>,
    pub rigid_body_list: EXGeoAbsArray<()>,
    pub swoosh_list: EXGeoAbsArray<()>,
    pub particle_system_list: EXGeoAbsArray<()>,
    pub timeline_list: EXGeoAbsArray<()>,
    pub zone_list: EXGeoAbsArray<()>,
    pub map_list: EXGeoAbsArray<()>,
    // Fixed header ends here, 0xb0 (176) bytes from the start of the chunk.
}

impl EXGeoHeaderXT {
    /// Reads an `EXGeoHeaderXT` from a full GEOM chunk buffer, decrypting
    /// the list region (see [`crate::geoxt_crypto`]) first since version
    /// 244 (and presumably later) EngineXT chunks store it encrypted.
    ///
    /// Returns both the parsed header and a decrypted copy of the whole
    /// buffer, so callers can go on to read list contents (textures,
    /// materials, entities, etc.) at the offsets the header reports without
    /// re-decrypting.
    ///
    /// Not every chunk turns out to need this -- some (so far seen only for
    /// `CustomAttributeTemplate`-bearing data/config chunks) are already
    /// plaintext, and applying the cipher would corrupt them. If the
    /// decrypted header's list descriptors don't look at all sane (wildly
    /// implausible counts), try [`Self::read_plain`] instead.
    pub fn read_decrypted(data: &[u8]) -> binrw::BinResult<(Self, Vec<u8>)> {
        // base_file_size lives in the unencrypted part of the header
        // (file offset 0x18, right after file_size at 0x14) -- peek it
        // before decrypting to know exactly where the cipher region ends.
        // This is *not* the same as file_size/filelen: for chunks that have
        // a lot of genuinely-plaintext data appended after the list region
        // (e.g. CustomAttributeTemplate row data), base_file_size covers
        // only the encrypted header+list portion, while file_size covers
        // the whole resource including that trailing plaintext data --
        // decrypting all the way to file_size corrupts it. For chunks
        // without such trailing data the two are equal, so this didn't
        // matter until a real example (a PC LevelCAT-bearing chunk,
        // base_file_size=1488 vs file_size=82228) turned up.
        let base_file_size = if data.len() >= 0x1c {
            Some(u32::from_le_bytes(data[0x18..0x1c].try_into().unwrap()) as usize)
        } else {
            None
        };
        let decrypted = crate::geoxt_crypto::decrypt_geoxt_region_copy(data, base_file_size);
        let header = Self::read_plain(&decrypted)?;
        Ok((header, decrypted))
    }

    /// Reads an `EXGeoHeaderXT` from a buffer assumed to already be
    /// plaintext (no decryption applied). Endianness is auto-detected from
    /// the magic the same way the rest of eurochef does (big-endian console
    /// builds store the magic byte-reversed from PC's little-endian form).
    pub fn read_plain(data: &[u8]) -> binrw::BinResult<Self> {
        use binrw::BinReaderExt;
        let mut cursor = std::io::Cursor::new(data);
        let endian = if data.first() == Some(&0x47) {
            binrw::Endian::Big
        } else {
            binrw::Endian::Little
        };
        cursor.read_type(endian)
    }

    /// Reads `texture_list` and `material_list` contents from their *real*
    /// positions.
    ///
    /// Important quirk: each list descriptor's `offset_absolute` field is
    /// **not** where this data actually lives -- at least for
    /// `texture_list`/`material_list`/`entity_list`, the real data is
    /// packed back-to-back in a fixed order (textures, then materials,
    /// then entities, ...) starting immediately after the fixed 0xB0-byte
    /// header, regardless of what the descriptor offsets claim. Verified
    /// against multiple real files where the declared offset and the
    /// computed sequential offset clearly diverge (e.g. `texture_list`
    /// declared offset `0x64`, real data found at the fixed `0xB0`).
    ///
    /// `texture_list` entries are a flat `u32` hashcode each (4 bytes).
    /// `material_list` entries are 20 bytes: `material_uid: u32`,
    /// `texture_uid: u32` (the texture this material references),
    /// `_reserved: u32` (always 0 in samples seen), `index: u32` (a
    /// 0-based position counter, purpose beyond ordering unconfirmed),
    /// `packed: u32` (low byte always 1 in samples seen, meaning
    /// unconfirmed -- possibly a flags/size field).
    ///
    /// See [`EXGeoHeaderXT::read_entity_list`] for `entity_list`, which
    /// shares this 20-byte record shape but needs a different (scan-based)
    /// way to find where its data actually starts.
    pub fn read_texture_and_material_lists(&self, decrypted: &[u8], endian: binrw::Endian) -> binrw::BinResult<(Vec<u32>, Vec<EXGeoMaterialEntryXT>)> {
        use binrw::BinReaderExt;
        let mut cursor = std::io::Cursor::new(decrypted);

        let tex_start = 0xB0u64;
        cursor.seek(std::io::SeekFrom::Start(tex_start))?;
        let mut textures = Vec::with_capacity(self.texture_list.count as usize);
        for _ in 0..self.texture_list.count {
            textures.push(cursor.read_type::<u32>(endian)?);
        }

        let mat_start = tex_start + (self.texture_list.count as u64) * 4;
        cursor.seek(std::io::SeekFrom::Start(mat_start))?;
        let mut materials = Vec::with_capacity(self.material_list.count as usize);
        for _ in 0..self.material_list.count {
            materials.push(cursor.read_type::<EXGeoMaterialEntryXT>(endian)?);
        }

        Ok((textures, materials))
    }

    /// Reads `entity_list` contents.
    ///
    /// `entity_list` entries are the same 20-byte shape as
    /// [`EXGeoMaterialEntryXT`] -- `entity_uid`, `group_ref_uid` (a
    /// hashcode this entity references, category seen so far always one
    /// step "below" the entity's own category), `_reserved: u32` (always
    /// 0 in samples seen), `index: u32`, `packed: u32` -- but unlike
    /// `texture_list`/`material_list`, its real start position is *not*
    /// simply "right after material_list".
    ///
    /// `index` turns out to be a single counter shared across every
    /// 20-byte-record list in the file (texture_list has none, but
    /// material_list and entity_list both draw from it), incrementing by
    /// exactly 1 per record in file order. In every real file checked,
    /// `entity_list`'s first `index` is *not* always `material_list`'s
    /// last `index + 1` -- there can be a gap, almost certainly some
    /// amount of `group_section_list` data (also drawing from the same
    /// counter, and/or a fixed-size reference sub-table) sitting between
    /// them that hasn't been mapped yet.
    ///
    /// Rather than guess that gap's size, this scans forward from right
    /// after `material_list` for the first position where `entity_list`'s
    /// declared `count` worth of 20-byte records all have `_reserved == 0`
    /// and strictly-ascending `index` values. That signature is specific
    /// enough that it has produced exactly one match in every real file
    /// checked. Returns `None` if no such position is found within the
    /// buffer (e.g. `entity_list.count == 0`, or the signature genuinely
    /// isn't present).
    pub fn read_entity_list(&self, decrypted: &[u8], endian: binrw::Endian, materials: &[EXGeoMaterialEntryXT]) -> Option<Vec<EXGeoEntityListEntryXT>> {
        let count = self.entity_list.count as usize;
        if count == 0 {
            return Some(Vec::new());
        }

        let read_u32 = |off: usize| -> Option<u32> {
            let bytes: [u8; 4] = decrypted.get(off..off + 4)?.try_into().ok()?;
            Some(match endian {
                binrw::Endian::Big => u32::from_be_bytes(bytes),
                binrw::Endian::Little => u32::from_le_bytes(bytes),
            })
        };

        // material_list's last index + 1 is a lower bound, not an exact
        // prediction -- there can be a gap of unmapped data (see doc
        // comment) before entity_list's data actually starts, so its first
        // index can be higher than this. The scan below only requires
        // internal consistency (ascending by exactly 1, resv == 0)
        // plus that first index being at or past this bound.
        let min_first_index = materials.last().map(|m| m.index.wrapping_add(1)).unwrap_or(0);
        let search_start = 0xB0 + self.texture_list.count as usize * 4 + self.material_list.count as usize * 20;
        let search_end = decrypted.len().checked_sub(count * 20)?;

        let mut pos = search_start;
        while pos <= search_end {
            let first_index = read_u32(pos + 12);
            let matches = first_index.is_some_and(|fi| fi >= min_first_index)
                && (0..count).all(|i| {
                    let off = pos + i * 20;
                    read_u32(off + 8) == Some(0) && read_u32(off + 12) == Some(first_index.unwrap().wrapping_add(i as u32))
                });
            if matches {
                use binrw::BinReaderExt;
                let mut cursor = std::io::Cursor::new(decrypted);
                cursor.seek(SeekFrom::Start(pos as u64)).ok()?;
                let mut out = Vec::with_capacity(count);
                for _ in 0..count {
                    out.push(cursor.read_type::<EXGeoEntityListEntryXT>(endian).ok()?);
                }
                return Some(out);
            }
            pos += 4;
        }
        None
    }
}

/// A single `entity_list` entry. See
/// [`EXGeoHeaderXT::read_entity_list`] for how this was found.
#[binrw]
#[derive(Debug, Clone, Copy)]
pub struct EXGeoEntityListEntryXT {
    pub entity_uid: u32,
    pub group_ref_uid: u32,
    pub _reserved: u32,
    pub index: u32,
    pub packed: u32,
}

/// A single `material_list` entry for EngineXT (version 244+) GEOM chunks.
/// See [`EXGeoHeaderXT::read_texture_and_material_lists`] for how this was
/// found and what isn't confirmed yet.
#[binrw]
#[derive(Debug, Clone, Copy)]
pub struct EXGeoMaterialEntryXT {
    pub material_uid: u32,
    pub texture_uid: u32,
    pub _reserved: u32,
    pub index: u32,
    pub packed: u32,
}

#[cfg(test)]
mod xt_tests {
    use super::*;

    /// Test fixtures are real Disney Universe files that cannot be redistributed, so they are not in the repo.
    /// Point `EUROCHEF_TEST_FIXTURES` at a directory containing them (e.g. `nightmare_named/io_nbc_glo_gravestone.edb`)
    /// to run these tests; without it each test is skipped.
    fn fixture(relative: &str) -> Option<Vec<u8>> {
        let dir = std::env::var_os("EUROCHEF_TEST_FIXTURES")?;
        let path = std::path::Path::new(&dir).join(relative);
        match std::fs::read(&path) {
            Ok(data) => Some(data),
            Err(_) => {
                eprintln!("skipping: fixture {} not found", path.display());
                None
            }
        }
    }

    /// A static prop model with known texture/material/entity counts,
    /// independently verified with a separate Python implementation against
    /// the same real Xbox 360 Nightmare Before Christmas DLC file.
    #[test]
    fn gravestone_decrypts_to_known_counts() {
        let Some(data) = fixture("nightmare_named/io_nbc_glo_gravestone.edb") else { return; };
        let (header, decrypted) = EXGeoHeaderXT::read_decrypted(&data).expect("parse failed");

        assert_eq!(header.hashcode, 0x40100eda);
        assert_eq!(header.version, 244);

        assert_eq!(header.group_section_list.count, 1);
        assert_eq!(header.custom_attribute_template_list.count, 1);
        assert_eq!(header.shader_list.count, 0);
        assert_eq!(header.texture_list.count, 7);
        assert_eq!(header.material_list.count, 7);
        assert_eq!(header.entity_list.count, 4);
        assert_eq!(header.skeleton_list.count, 0);
        assert_eq!(header.timeline_list.count, 1);

        // texture_list/material_list's declared offset_absolute fields are
        // misleading (see read_texture_and_material_lists docs) -- the real
        // data is packed sequentially starting right after the fixed
        // header, found by working through the bytes directly rather than
        // trusting the descriptor offsets.
        let (textures, materials) = header
            .read_texture_and_material_lists(&decrypted, binrw::Endian::Big)
            .expect("list read failed");

        assert_eq!(
            textures,
            vec![
                0x4020483e, 0x4020483f, 0x40204840, 0x40204841, 0x40204842, 0x40204843,
                0x40204898
            ]
        );

        assert_eq!(materials.len(), 7);
        for (i, m) in materials.iter().enumerate() {
            assert_eq!(m.index, i as u32, "material[{i}].index");
            assert_eq!(m._reserved, 0, "material[{i}]._reserved");
        }
        // First three materials reference the first three textures in order.
        assert_eq!(materials[0].material_uid, 0x40280000);
        assert_eq!(materials[0].texture_uid, textures[0]);
        assert_eq!(materials[1].material_uid, 0x40280001);
        assert_eq!(materials[1].texture_uid, textures[1]);

        let entities = header
            .read_entity_list(&decrypted, binrw::Endian::Big, &materials)
            .expect("entity_list signature not found");
        assert_eq!(entities.len(), 4);
        for (i, e) in entities.iter().enumerate() {
            assert_eq!(e._reserved, 0, "entity[{i}]._reserved");
            assert_eq!(e.index, materials.last().unwrap().index + 1 + i as u32, "entity[{i}].index");
        }
    }

    #[test]
    fn scythestatue_entity_list_signature_found() {
        // A rigged/articulated prop (unlike the static gravestone/candycane)
        // -- has extra data between material_list and entity_list that
        // isn't mapped yet (likely group_section_list's real payload), so
        // entity_list's first index doesn't immediately follow
        // material_list's last index here. read_entity_list's scan handles
        // this; this test exists specifically to keep covering that case.
        let Some(data) = fixture("nightmare_named/io_nbc_l01_a01_scythestatue.edb") else { return; };
        let (header, decrypted) = EXGeoHeaderXT::read_decrypted(&data).expect("parse failed");
        assert_eq!(header.material_list.count, 4);
        assert_eq!(header.entity_list.count, 6);

        let (_, materials) = header
            .read_texture_and_material_lists(&decrypted, binrw::Endian::Big)
            .expect("list read failed");
        let entities = header
            .read_entity_list(&decrypted, binrw::Endian::Big, &materials)
            .expect("entity_list signature not found");
        assert_eq!(entities.len(), 6);
        for (i, e) in entities.iter().enumerate() {
            assert_eq!(e._reserved, 0, "entity[{i}]._reserved");
            if i > 0 {
                assert_eq!(e.index, entities[i - 1].index + 1, "entity[{i}].index");
            }
        }
    }

    #[test]
    fn candycane_textures_and_materials() {
        let Some(data) = fixture("nightmare_named/io_nbc_glo_candycane.edb") else { return; };
        let (header, decrypted) = EXGeoHeaderXT::read_decrypted(&data).expect("parse failed");
        assert_eq!(header.hashcode, 0x40100ee4);
        assert_eq!(header.texture_list.count, 5);
        assert_eq!(header.material_list.count, 2);

        let (textures, materials) = header
            .read_texture_and_material_lists(&decrypted, binrw::Endian::Big)
            .expect("list read failed");
        assert_eq!(
            textures,
            vec![0x402046ff, 0x40204d96, 0x40205705, 0x40205706, 0x40205707]
        );
        assert_eq!(materials.len(), 2);
        assert_eq!(materials[0].material_uid, 0x40280000);
        assert_eq!(materials[0].texture_uid, textures[0]);
        assert_eq!(materials[0].index, 0);
        assert_eq!(materials[1].material_uid, 0x40280001);
        assert_eq!(materials[1].texture_uid, textures[1]);
        assert_eq!(materials[1].index, 1);

        let entities = header
            .read_entity_list(&decrypted, binrw::Endian::Big, &materials)
            .expect("entity_list signature not found");
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].index, 2);
        assert_eq!(entities[1].index, 3);
        assert_eq!(entities[0]._reserved, 0);
        assert_eq!(entities[1]._reserved, 0);
    }

    #[test]
    fn content_basepack_decrypts_to_known_counts() {
        // A pure data/manifest file with no visual assets -- everything
        // except group_section_list and custom_attribute_template_list
        // should be empty.
        let Some(data) = fixture("nightmare_named/nbc_content_basepack.edb") else { return; };
        let (header, _) = EXGeoHeaderXT::read_decrypted(&data).expect("parse failed");

        assert_eq!(header.group_section_list.count, 1);
        assert_eq!(header.custom_attribute_template_list.count, 17);
        assert_eq!(header.texture_list.count, 0);
        assert_eq!(header.material_list.count, 0);
        assert_eq!(header.entity_list.count, 0);
    }

    /// A real PC chunk (little-endian, magic stored as "MOEG") whose
    /// base_file_size (1488) is drastically smaller than its file_size
    /// (82228) -- the bulk of the file is genuinely-plaintext
    /// CustomAttributeTemplate row/schema data appended after the encrypted
    /// header+list region. This is the regression test for the
    /// base_file_size-vs-file_size decrypt-boundary bug: decrypting all the
    /// way to file_size corrupts that trailing data (confirmed manually --
    /// readable column-name strings like "Scheme"/"HUBIconR" turn to
    /// garbage), while base_file_size leaves it untouched.
    #[test]
    fn pc_levelcat_file_uses_base_file_size_not_file_size() {
        let Some(data) = fixture("pc_levelcat_0x40100d13.edb") else { return; };
        assert_eq!(data[0..4], *b"MOEG");

        let (header, decrypted) = EXGeoHeaderXT::read_decrypted(&data).expect("parse failed");
        assert_eq!(header.hashcode, 0x40100d13);
        assert_eq!(header.file_size, 82228);
        assert_eq!(header.base_file_size, 1488);
        assert_eq!(header.group_section_list.count, 1);
        assert_eq!(header.custom_attribute_template_list.count, 25);

        // The genuinely-plaintext column-schema region starting at this
        // relative offset must survive untouched -- it would decrypt to
        // garbage if the cipher were (wrongly) applied all the way to
        // file_size instead of stopping at base_file_size.
        let schema_region = &decrypted[0x3080..0x3080 + 16];
        assert!(schema_region.windows(6).any(|w| w == b"Scheme"));
    }
}
