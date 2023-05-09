use std::io::{Read, Seek};

use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    header::EXGeoHeader,
};

pub struct FileInfoPanel {
    header: EXGeoHeader,
}

impl FileInfoPanel {
    pub fn new(header: EXGeoHeader) -> Self {
        Self { header }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        macro_rules! quick_info {
            ($label:expr, $value:expr) => {
                ui.horizontal(|ui| {
                    ui.strong(format!("{}:", $label));
                    ui.label($value);
                })
            };
        }
        macro_rules! quick_array {
            ($label:expr, $array:ident) => {
                let v = self.header.$array.len();
                quick_info!(
                    $label,
                    if v == 0 {
                        "empty".to_string()
                    } else {
                        if v > 1 {
                            format!("{} entries", v)
                        } else {
                            "1 entry".to_string()
                        }
                    }
                )
            };
        }

        ui.label(egui::RichText::new("EDB File Info").heading());
        quick_info!("Version", self.header.version.to_string());
        quick_info!(
            "Base file size",
            format!("{}KB", self.header.base_file_size / 1024)
        );
        quick_info!("File size", format!("{}KB", self.header.file_size / 1024));

        ui.separator();

        quick_array!("Sections", section_list);
        quick_array!("Refpointers", refpointer_list);
        quick_array!("Entities", entity_list);
        quick_array!("Animations", anim_list);
        quick_array!("Animation skins", animskin_list);
        quick_array!("Scripts", script_list);
        quick_array!("Maps", map_list);
        quick_array!("Animation modes", animmode_list);
        quick_array!("Animation sets", animset_list);
        quick_array!("Particles", particle_list);
        quick_array!("Swooshes", swoosh_list);
        quick_array!("Spreadsheets", spreadsheet_list);
        quick_array!("Fonts", font_list);
        quick_array!("Force feedback", font_list);
        quick_array!("Materials", material_list);
        quick_array!("Textures", texture_list);
        quick_array!("unk_c0", unk_c0);
    }
}

// TODO(cohae): EdbFile struct so we dont have to read endianness separately
pub fn read_from_file<R: Read + Seek>(reader: &mut R) -> EXGeoHeader {
    reader.seek(std::io::SeekFrom::Start(0)).ok();
    let endian = if reader.read_ne::<u8>().unwrap() == 0x47 {
        Endian::Big
    } else {
        Endian::Little
    };
    reader.seek(std::io::SeekFrom::Start(0)).unwrap();

    let header = reader
        .read_type::<EXGeoHeader>(endian)
        .expect("Failed to read header");

    header
}
