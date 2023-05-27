use glam::{Mat4, Vec2};

pub trait Camera3D: Sync + Send {
    fn update(&mut self, ui: &egui::Ui, response: Option<egui::Response>, delta: f32);

    /// Calculate the view matrix
    fn calculate_matrix(&self) -> Mat4;

    fn zoom(&self) -> f32;
}

fn zoom_factor(zoom_level: f32) -> f32 {
    2.0f32.powf(zoom_level * std::f32::consts::LN_2) - 0.9
}

#[derive(Clone)]
pub struct ArcBallCamera {
    orientation: Vec2,
    zoom: f32,
    log_zoom: bool,
}

impl ArcBallCamera {
    pub fn new(orientation: Vec2, zoom: f32, log_zoom: bool) -> Self {
        ArcBallCamera {
            orientation,
            zoom,
            log_zoom,
        }
    }
}

impl Default for ArcBallCamera {
    fn default() -> Self {
        ArcBallCamera {
            orientation: Vec2::new(-2., -1.),
            zoom: 5.0,
            log_zoom: true,
        }
    }
}

impl Camera3D for ArcBallCamera {
    fn update(&mut self, ui: &egui::Ui, response: Option<egui::Response>, _delta: f32) {
        if let Some(multi_touch) = ui.ctx().multi_touch() {
            self.zoom += -(multi_touch.zoom_delta - 1.0);
        } else {
            if let Some(response) = response {
                self.orientation +=
                    Vec2::new(response.drag_delta().x, response.drag_delta().y) * 0.005;
            }
            self.zoom += -ui.input(|i| i.scroll_delta).y * 0.005;
        }

        self.zoom = self.zoom.clamp(0.00, 250.0);
    }

    fn calculate_matrix(&self) -> Mat4 {
        glam::Mat4::from_rotation_translation(
            glam::Quat::from_rotation_x(self.orientation.y)
                * glam::Quat::from_rotation_z(self.orientation.x),
            glam::vec3(
                0.0,
                0.0,
                if self.log_zoom {
                    -zoom_factor(self.zoom)
                } else {
                    -self.zoom
                },
            ),
        )
    }

    fn zoom(&self) -> f32 {
        if self.log_zoom {
            zoom_factor(self.zoom)
        } else {
            self.zoom
        }
    }
}
