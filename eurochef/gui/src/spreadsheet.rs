use egui::FontSelection;
use eurochef_edb::Hashcode;
use eurochef_shared::spreadsheets::{UXGeoSpreadsheet, UXGeoTextItem};

pub struct TextItemList {
    /// Search query for a specific hashcode
    search_hashcode: String,
    /// Search query for text item contents
    search_text: String,

    spreadsheets: Vec<(Hashcode, UXGeoSpreadsheet)>,
    selected_section: usize,
}

impl TextItemList {
    pub fn new(spreadsheets: Vec<(Hashcode, UXGeoSpreadsheet)>) -> Self {
        Self {
            search_hashcode: String::new(),
            search_text: String::new(),
            spreadsheets,
            selected_section: 0,
        }
    }

    // TODO: Display separate spreadsheets
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Search: ");
            ui.text_edit_singleline(&mut self.search_text);
            ui.label("Hashcode: ");
            egui::TextEdit::singleline(&mut self.search_hashcode)
                .font(FontSelection::Style(egui::TextStyle::Monospace))
                .desired_width(76.0)
                .show(ui);
        });

        ui.separator();

        let spreadsheet = self.spreadsheets.iter().find(|(_, v)| match v {
            UXGeoSpreadsheet::Data(_) => false,
            UXGeoSpreadsheet::Text(_) => true,
        });

        if spreadsheet.is_none() {
            ui.heading("No text spreadsheets found");
            return;
        }

        let (_, spreadsheet) = spreadsheet.unwrap();

        let filtered_items: Vec<Vec<&UXGeoTextItem>> = match spreadsheet {
            UXGeoSpreadsheet::Text(v) => v,
            _ => unreachable!(),
        }
        .iter()
        .map(|s| {
            s.entries
                .iter()
                .filter(|v| {
                    if self.search_hashcode.is_empty() {
                        true
                    } else {
                        format!("{:08x}", v.hashcode).contains(&self.search_hashcode.to_lowercase())
                    }
                })
                .filter(|v| {
                    v.text
                        .to_lowercase()
                        .contains(&self.search_text.to_lowercase())
                })
                .collect()
        })
        .collect();

        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_source("section_scroll_area")
                    .show(ui, |ui| {
                        let mut current_set = 0;
                        for (i, s) in match spreadsheet {
                            UXGeoSpreadsheet::Text(v) => v,
                            _ => unreachable!(),
                        }
                        .iter()
                        .enumerate()
                        {
                            if filtered_items[i].is_empty() {
                                continue;
                            }

                            if (s.hashcode & 0xffff0000) != current_set && s.hashcode != u32::MAX {
                                ui.label(format!("Set {:08x}", s.hashcode & 0xffff0000));
                            }

                            if s.hashcode != u32::MAX {
                                current_set = s.hashcode & 0xffff0000;
                            }

                            ui.selectable_value(
                                &mut self.selected_section,
                                i,
                                format!("  Section {:08x}", s.hashcode),
                            );
                        }
                    });
            });

            ui.vertical(|ui| {
                let text_height = egui::TextStyle::Body.resolve(ui.style()).size * 1.25;
                let table = egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .column(egui_extras::Column::exact(76.0))
                    .column(egui_extras::Column::exact(76.0))
                    .column(egui_extras::Column::remainder().resizable(true).clip(true));

                table
                    .header(20., |mut header| {
                        header.col(|ui| {
                            ui.strong("Hashcode");
                        });
                        header.col(|ui| {
                            ui.strong("Sound");
                        });
                        header.col(|ui| {
                            ui.strong("Text");
                        });
                    })
                    .body(|body| {
                        body.rows(
                            text_height,
                            filtered_items[self.selected_section].len(),
                            |row_index, mut row| {
                                let item = filtered_items[self.selected_section][row_index];
                                let context_menu = |ui: &mut egui::Ui| {
                                    if ui.button("Copy hashcode").clicked() {
                                        ui.output_mut(|o| {
                                            o.copied_text = format!("{:08x}", item.hashcode)
                                        });
                                        ui.close_menu()
                                    }
                                    if ui.button("Copy text").clicked() {
                                        ui.output_mut(|o| o.copied_text = item.text.clone());
                                        ui.close_menu()
                                    }
                                };

                                row.col(|ui| {
                                    ui.label(format!("0x{:x}", item.hashcode));
                                })
                                .1
                                .context_menu(context_menu);

                                row.col(|ui| {
                                    if item.sound_hashcode == u32::MAX {
                                        ui.label("none");
                                    } else {
                                        ui.label(format!("0x{:x}", item.sound_hashcode));
                                    }
                                })
                                .1
                                .context_menu(context_menu);

                                row.col(|ui| {
                                    ui.style_mut().wrap = Some(false);
                                    ui.label(&item.text.replace('\n', "\\n"));
                                })
                                .1
                                .context_menu(context_menu);
                            },
                        )
                    });
            });
        });
    }
}
