use std::io::{Seek, Read};

use egui::Widget;
use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    header::EXGeoHeader,
    versions::Platform,
};
use eurochef_shared::textures::UXGeoTexture;
use fnv::FnvHashMap;
use instant::Instant;

pub struct TextureList {
    textures: Vec<UXGeoTexture>,

    // Each texture is a collection of frame textures
    egui_textures: FnvHashMap<u32, Vec<egui::TextureHandle>>,

    start_time: Instant,

    // Options/filters
    zoom: f32,
    filter_animated: bool,

    enlarged_texture: Option<(usize, u32)>,
}

impl TextureList {
    pub fn new(textures: Vec<UXGeoTexture>) -> Self {
        Self {
            textures,
            egui_textures: FnvHashMap::default(),
            start_time: Instant::now(),

            zoom: 1.0,
            filter_animated: false,

            enlarged_texture: None,
        }
    }

    pub fn load_textures(&mut self, ctx: &egui::Context) {
        for t in &self.textures {
            let frames = t
                .frames
                .iter()
                .map(|f| {
                    ctx.load_texture(
                        format!("{:08x}", t.hashcode),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [t.width as usize, t.height as usize],
                            &f,
                        ),
                        egui::TextureOptions::default(),
                    )
                })
                .collect();

            self.egui_textures.insert(t.hashcode, frames);
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
            .always_show_scroll(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = [4.; 2].into();
                    for (i, t) in self.textures.iter().enumerate() {
                        if self.filter_animated && self.textures[i].frame_count <= 1 {
                            continue;
                        }

                        let time = self.start_time.elapsed().as_secs_f32();
                        let frametime_scale = t.frame_count as f32 / t.frames.len() as f32;
                        let frame_time = (1. / t.framerate as f32) * frametime_scale;

                        let frames = &self.egui_textures[&t.hashcode];
                        if frames.len() == 0 {
                            continue;
                        }

                        let current = if frames.len() > 1 {
                            &frames[(time / frame_time) as usize % frames.len()]
                        } else {
                            &frames[0]
                        };

                        // TODO(cohae): figure out scrolling
                        // let offset_per_second = egui::vec2(t.scroll[0] as f32, t.scroll[1] as f32) / 30000.0;
                        // let mut offset = offset_per_second * time;

                        // if t.scroll[0].abs() == 1 {
                        //     offset.x = time * t.scroll[0] as f32;
                        // }
                        // if t.scroll[1].abs() == 1 {
                        //     offset.y = time * t.scroll[1] as f32;
                        // }
                        
                        // offset.x %= 1.0;
                        // offset.y %= 1.0;

                        // let uv = egui::Rect::from_min_max(egui::pos2(0. + offset.x, 0. + offset.y), egui::pos2(1. + offset.x, 1. + offset.y));
                        let response = egui::Image::new(current, [128. * self.zoom, 128. * self.zoom]).sense(egui::Sense::click()).ui(ui)
                            .on_hover_ui(|ui| {
                                ui.label(format!(
                                    "Hashcode: {:08x}\nFormat (internal): 0x{:x}\nDimensions: {}x{}x{}\nScroll: {} {}\nFlags: 0x{:x}\n",
                                    t.hashcode, t.format_internal, t.width, t.height, t.depth, t.scroll[0], t.scroll[1], t.game_flags
                                ));

                                if frames.len() > 1 {
                                    ui.label(format!("{} frames ({} fps)\n", frames.len(), t.framerate));
                                }

                                ui.strong("Click to enlarge");
                            }).on_hover_cursor(egui::CursorIcon::PointingHand);

                        if response.clicked() {
                            self.enlarged_texture = Some((i, t.hashcode));
                        }
                    }
                });
            });
    }

    pub fn show_enlarged_window(&mut self, ctx: &egui::Context) {
        let mut window_open = self.enlarged_texture.is_some();
        if let Some(enlarged_texture) = self.enlarged_texture {
            let (i, _hashcode) = enlarged_texture;
            let t = &self.textures[i];

            // TODO(cohae): Fix resizing window
            egui::Window::new("Texture Viewer")
                .open(&mut window_open)
                .collapsible(false)
                .default_height(ctx.available_rect().height() * 0.70 as f32)
                .show(ctx, |ui| {
                    let time = self.start_time.elapsed().as_secs_f32();
                    let frametime_scale = t.frame_count as f32 / t.frames.len() as f32;
                    let frame_time = (1. / t.framerate as f32) * frametime_scale;

                    let frames = &self.egui_textures[&t.hashcode];
                    let current = if frames.len() > 0 {
                        &frames[(time / frame_time) as usize % frames.len()]
                    } else {
                        &frames[0]
                    };

                    egui::Image::new(current, current.size_vec2() * 2.5)
                        .ui(ui);

                    // TODO(cohae): Animation checkbox, when unticked, show frame slider
                });
        }

        if !window_open {
            self.enlarged_texture = None;
        }
    }
}

// TODO(cohae): EdbFile struct so we dont have to read endianness separately
pub fn read_from_file<R: Read + Seek>(reader: &mut R, platform: Platform) -> Vec<UXGeoTexture> {
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

    UXGeoTexture::read_all(&header, reader, platform).unwrap()
}
