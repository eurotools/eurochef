use egui::{Color32, Widget};
use eurochef_shared::{textures::UXGeoTexture, IdentifiableResult};
use fnv::FnvHashMap;
use instant::Instant;

use crate::strip_ansi_codes;

pub struct TextureList {
    textures: Vec<IdentifiableResult<UXGeoTexture>>,

    // Each texture is a collection of frame textures
    egui_textures: FnvHashMap<u32, Vec<egui::TextureHandle>>,

    start_time: Instant,

    // Options/filters
    zoom: f32,
    filter_animated: bool,

    enlarged_texture: Option<(usize, u32)>,
    enlarged_zoom: f32,

    fallback_texture: egui::TextureHandle,
}

impl TextureList {
    const ENLARGED_ZOOM_DEFAULT: f32 = 2.5;

    pub fn new(ctx: &egui::Context, textures: Vec<IdentifiableResult<UXGeoTexture>>) -> Self {
        let mut s = Self {
            textures,
            egui_textures: FnvHashMap::default(),
            start_time: Instant::now(),

            zoom: 1.0,
            filter_animated: false,

            enlarged_texture: None,
            enlarged_zoom: Self::ENLARGED_ZOOM_DEFAULT,

            fallback_texture: ctx.load_texture(
                "fallback",
                egui::ColorImage::from_rgba_unmultiplied([1, 1], &[0, 0, 0, 0]),
                egui::TextureOptions::default(),
            ),
        };

        s.load_textures(ctx);

        s
    }

    pub fn load_textures(&mut self, ctx: &egui::Context) {
        for it in &self.textures {
            if let Ok(t) = &it.data {
                let frames: Vec<egui::TextureHandle> = t
                    .frames
                    .iter()
                    .map(|f| {
                        ctx.load_texture(
                            format!("{:08x}", it.hashcode),
                            egui::ColorImage::from_rgba_unmultiplied(
                                [t.width as usize, t.height as usize],
                                &f,
                            ),
                            egui::TextureOptions::default(),
                        )
                    })
                    .collect();

                self.egui_textures.insert(it.hashcode, frames);
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Zoom: ");
            egui::Slider::new(&mut self.zoom, 0.25..=3.0)
                .show_value(true)
                .ui(ui);

            egui::Checkbox::new(&mut self.filter_animated, "Animated only").ui(ui);
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .id_source("section_scroll_area")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = [4. * self.zoom; 2].into();
                    for (i, it) in self.textures.iter().enumerate() {
                        // Skip null texture
                        if it.hashcode == 0x06000000 {
                            continue;
                        }

                        match (&it.data, it.data.as_ref().map(|d| d.external_texture).ok().flatten()) {
                            (Ok(t), None) => {
                                if self.filter_animated && t.frame_count <= 1 {
                                    continue;
                                }

                                let time = self.start_time.elapsed().as_secs_f32();
                                let frametime_scale = t.frame_count as f32 / t.frames.len() as f32;
                                let frame_time = (1. / t.framerate as f32) * frametime_scale;

                                let frames = &self.egui_textures[&it.hashcode];
                                let current = if frames.len() == 0 {
                                    &self.fallback_texture
                                } else {
                                    if frames.len() > 1 {
                                        &frames[(time / frame_time) as usize % frames.len()]
                                    } else {
                                        &frames[0]
                                    }
                                };

                                let diagnostics = t.diagnostics.to_strings();

                                let response = egui::Image::new(current, egui::vec2(128., 128.) * self.zoom).sense(egui::Sense::click()).ui(ui)
                                .on_hover_ui(|ui| {
                                    ui.label(format!(
                                        "Hashcode: {:08x}\nFormat (internal): 0x{:x}\nDimensions: {}x{}{}\nScroll: {} {}\nFlags: 0x{:x}\nGameflags: 0x{:x}\nIndex: {i}\n",
                                        it.hashcode, t.format_internal, t.width, t.height, if t.depth <= 1 { String::new() } else { format!("x{}", t.depth) }, t.scroll[0], t.scroll[1], t.flags, t.game_flags
                                    ));

                                    if frames.len() > 1 {
                                        ui.label(format!("{} frames ({} fps)\n", frames.len(), t.framerate));
                                    }

                                    for d in &diagnostics {
                                        ui.colored_label(Color32::YELLOW, *d);
                                    }

                                    if !diagnostics.is_empty() {
                                        ui.label("");
                                    }

                                    ui.strong("Click to enlarge");
                                }).on_hover_cursor(egui::CursorIcon::PointingHand);

                                if !diagnostics.is_empty() {
                                    ui.painter().text(
                                        response.rect.left_top() + egui::vec2(24., 24.),
                                        egui::Align2::CENTER_CENTER,
                                        font_awesome::EXCLAMATION_TRIANGLE,
                                        egui::FontId::proportional(24.),
                                        Color32::YELLOW,
                                    );
                                }

                                if response.clicked() {
                                    self.enlarged_texture = Some((i, it.hashcode));
                                }
                            }
                            (_, Some((ext_file, ext_texture))) => {
                                // We don't know anything about linked textures, skip if filtered
                                if self.filter_animated {
                                    continue;
                                }

                                let (rect, response) =
                                ui.allocate_exact_size(egui::vec2(128., 128.) * self.zoom, egui::Sense::click());
    
                                ui.painter().rect_filled( 
                                    rect,
                                    egui::Rounding::none(),
                                    Color32::BLACK,
                                );
        
                                ui.painter().text(
                                    rect.left_top() + egui::vec2(24., 24.),
                                    egui::Align2::CENTER_CENTER,
                                    font_awesome::LINK,
                                    egui::FontId::proportional(24.),
                                    Color32::RED,
                                );
                                response.on_hover_ui(|ui| {
                                    ui.colored_label(Color32::LIGHT_RED, format!(
                                        "Texture {:08x} is a reference to texture {:08x} in file {:08x}",
                                        it.hashcode, ext_texture, ext_file
                                    ));
                                });
                            }
                            (Err(e), _) => {
                                // We don't know anything about failed textures, skip if filtered
                                if self.filter_animated {
                                    continue;
                                }

                                let (rect, response) =
                                ui.allocate_exact_size(egui::vec2(128., 128.) * self.zoom, egui::Sense::click());
    
                                ui.painter().rect_filled( 
                                    rect,
                                    egui::Rounding::none(),
                                    Color32::BLACK,
                                );
        
                                ui.painter().text(
                                    rect.left_top() + egui::vec2(24., 24.),
                                    egui::Align2::CENTER_CENTER,
                                    font_awesome::EXCLAMATION_TRIANGLE,
                                    egui::FontId::proportional(24.),
                                    Color32::RED,
                                );
        
                                    response.on_hover_ui(|ui| {
                                        ui.label(format!(
                                            "Texture {:08x} failed:",
                                            it.hashcode
                                        ));
                                        ui.colored_label(Color32::LIGHT_RED, cutoff_string(strip_ansi_codes(&format!("{e:?}")), 1024));
                                    });
                            },
                        }
                    }
                });
            });
    }

    pub fn show_enlarged_window(&mut self, ctx: &egui::Context) {
        let mut window_open = self.enlarged_texture.is_some();
        if let Some(enlarged_texture) = self.enlarged_texture {
            let (i, _hashcode) = enlarged_texture;
            let it = &self.textures[i];

            if let Ok(t) = &it.data {
                // TODO(cohae): Fix resizing window
                egui::Window::new("Texture Viewer")
                    .open(&mut window_open)
                    .collapsible(false)
                    .default_height(ctx.available_rect().height() * 0.70 as f32)
                    .show(ctx, |ui| {
                        let time = self.start_time.elapsed().as_secs_f32();
                        let frametime_scale = t.frame_count as f32 / t.frames.len() as f32;
                        let frame_time = (1. / t.framerate as f32) * frametime_scale;

                        let frames = &self.egui_textures[&it.hashcode];
                        let current = if frames.len() > 0 {
                            &frames[(time / frame_time) as usize % frames.len()]
                        } else {
                            &frames[0]
                        };

                        if let pos = ctx.input(|i| i.zoom_delta()) {
                            self.enlarged_zoom *= pos;
                        }

                        egui::Image::new(current, current.size_vec2() * self.enlarged_zoom).ui(ui);

                        // TODO(cohae): Animation checkbox, when unticked, show frame slider
                    });
            }
        }

        if !window_open {
            self.enlarged_texture = None;
            self.enlarged_zoom = Self::ENLARGED_ZOOM_DEFAULT; // swy: reset the zoom level each time we close a preview
        }
    }
}

pub fn cutoff_string(string: String, max_len: usize) -> String {
    if string.len() > max_len {
        let new_string = String::from_utf8_lossy(&string.as_bytes()[..max_len]).to_string();
        new_string + "..."
    } else {
        string
    }
}
