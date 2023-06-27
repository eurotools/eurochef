use font_awesome as fa;

use eurochef_edb::header::EXGeoHeader;

pub struct FileInfoPanel {
    pub header: EXGeoHeader,
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

        ui.label(egui::RichText::new(format!("{} EDB File Info", fa::INFO_CIRCLE)).heading());
        quick_info!("Version", self.header.version.to_string());
        ui.horizontal(|ui| {
            ui.strong("Hashcode:");
            ui.label(format!("{:x}", self.header.hashcode));
            if ui.button(font_awesome::CLIPBOARD.to_string()).clicked() {
                ui.output_mut(|o| o.copied_text = format!("{:x}", self.header.hashcode));
            }
        });

        quick_info!("Flags", format!("0x{:08x}", self.header.flags));

        quick_info!(
            "Build timestamp",
            format!(
                "{}",
                chrono::NaiveDateTime::from_timestamp_opt(self.header.time as i64, 0).unwrap()
            )
        );
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
        quick_array!("Animation scripts", animscript_list);
        quick_array!("Maps", map_list);
        quick_array!("Animation modes", animmode_list);
        quick_array!("Animation sets", animset_list);
        quick_array!("Particles", particle_list);
        quick_array!("Swooshes", swoosh_list);
        quick_array!("Spreadsheets", spreadsheet_list);
        quick_array!("Fonts", font_list);
        quick_array!("Force feedback", forcefeedback_list);
        quick_array!("Materials", material_list);
        quick_array!("Textures", texture_list);
        quick_array!("unk_c0", unk_c0);
    }
}
