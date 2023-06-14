use std::fmt::Display;

use glam::Vec3;
use instant::Instant;

use super::{
    camera::{ArcBallCamera, Camera3D, FpsCamera},
    grid::GridRenderer,
    RenderUniforms,
};

#[derive(PartialEq, Eq)]
pub enum CameraType {
    Orbit,
    Fly,
}

impl Display for CameraType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CameraType::Orbit => f.write_str("Orbit"),
            CameraType::Fly => f.write_str("Fly"),
        }
    }
}

pub struct BaseViewer {
    pub show_grid: bool,
    pub orthographic: bool,
    pub camera_orbit: ArcBallCamera,
    pub camera_fly: FpsCamera,
    pub selected_camera: CameraType,
    pub grid: GridRenderer,
    pub uniforms: RenderUniforms,

    last_frame: Instant,
}

impl BaseViewer {
    pub fn new(gl: &glow::Context) -> Self {
        Self {
            camera_orbit: ArcBallCamera::default(),
            camera_fly: FpsCamera::default(),
            selected_camera: CameraType::Orbit,
            show_grid: true,
            orthographic: false,
            grid: GridRenderer::new(gl, 30),
            uniforms: RenderUniforms::default(),
            last_frame: Instant::now(),
        }
    }

    pub fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        if self.selected_camera == CameraType::Orbit {
            ui.checkbox(&mut self.orthographic, "Orthographic");
        }
        ui.checkbox(&mut self.show_grid, "Show grid");

        egui::ComboBox::from_label("Camera")
            .selected_text(self.selected_camera.to_string())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.selected_camera, CameraType::Orbit, "Orbit");
                ui.selectable_value(&mut self.selected_camera, CameraType::Fly, "Fly");
            });
    }

    pub fn show_statusbar(&mut self, ui: &mut egui::Ui) {
        let camera: &mut dyn Camera3D = match self.selected_camera {
            CameraType::Fly => &mut self.camera_fly,
            CameraType::Orbit => &mut self.camera_orbit,
        };

        if self.selected_camera == CameraType::Fly {
            ui.strong("Speed:");
        } else {
            ui.strong("Zoom:");
        }
        ui.label(format!("{:.2}", camera.zoom()));
    }

    pub fn update(&mut self, ui: &mut egui::Ui, response: &egui::Response) {
        if ui.input(|i| i.key_pressed(egui::Key::F)) {
            self.selected_camera = match self.selected_camera {
                CameraType::Orbit => CameraType::Fly,
                CameraType::Fly => CameraType::Orbit,
            };
        }

        if ui.input(|i| i.key_pressed(egui::Key::G)) {
            self.show_grid = !self.show_grid;
        }

        if ui.input(|i| i.key_pressed(egui::Key::O) || i.key_pressed(egui::Key::Num5)) {
            self.orthographic = !self.orthographic;
        }

        let camera: &mut dyn Camera3D = match self.selected_camera {
            CameraType::Fly => &mut self.camera_fly,
            CameraType::Orbit => &mut self.camera_orbit,
        };

        camera.update(
            ui,
            Some(response),
            (Instant::now() - self.last_frame).as_secs_f32(),
        );
        self.last_frame = Instant::now();
    }

    pub fn start_render(&mut self, gl: &glow::Context, aspect_ratio: f32, time: f32) {
        unsafe {
            super::start_render(gl);
        }

        let camera: &mut dyn Camera3D = match self.selected_camera {
            CameraType::Fly => &mut self.camera_fly,
            CameraType::Orbit => &mut self.camera_orbit,
        };
        self.uniforms.update(
            if self.selected_camera == CameraType::Orbit {
                self.orthographic
            } else {
                false
            },
            camera,
            aspect_ratio,
            time,
        );

        if self.show_grid {
            unsafe { self.grid.draw(&self.uniforms, gl) }
        }
    }

    pub fn focus_on_point(&mut self, point: Vec3, dist_scale: f32) {
        let camera: &mut dyn Camera3D = match self.selected_camera {
            CameraType::Fly => &mut self.camera_fly,
            CameraType::Orbit => &mut self.camera_orbit,
        };

        camera.focus_on_point(point, dist_scale);
    }
}
