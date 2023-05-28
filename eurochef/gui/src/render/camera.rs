use glam::{Mat4, Vec2, Vec3};

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
            orientation: Vec2::new(2.5, 0.5),
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
                * glam::Quat::from_rotation_y(self.orientation.x),
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

#[derive(Clone)]
pub struct FpsCamera {
    orientation: Vec2,
    pub front: Vec3,
    pub right: Vec3,
    pub position: Vec3,
    pub speed_mul: f32,
}

impl Default for FpsCamera {
    fn default() -> Self {
        Self {
            front: Vec3::Y,
            right: Vec3::Z,
            position: Vec3::ZERO,
            orientation: Vec2::ZERO,
            speed_mul: 1.0,
        }
    }
}

impl FpsCamera {
    fn update_vectors(&mut self) {
        let mut front = Vec3::ZERO;
        front.x = -self.orientation.x.to_radians().cos() * self.orientation.y.to_radians().sin();
        front.y = -self.orientation.x.to_radians().sin();
        front.z = self.orientation.x.to_radians().cos() * self.orientation.y.to_radians().cos();

        self.front = front.normalize();
        self.right = self.front.cross(Vec3::Y).normalize();
    }
}

impl Camera3D for FpsCamera {
    fn update(&mut self, ui: &egui::Ui, response: Option<egui::Response>, delta: f32) {
        let scroll = ui.input(|i| i.scroll_delta);
        self.speed_mul = (self.speed_mul + scroll.y * 0.005).clamp(0.0, 5.0);

        if let Some(response) = response {
            let mouse_delta = response.drag_delta();
            self.orientation += Vec2::new(mouse_delta.y as f32 * 0.8, mouse_delta.x as f32) * 0.15;
        }

        let mut speed = delta * zoom_factor(self.speed_mul) * 10.0;
        if ui.input(|i| i.modifiers.shift) {
            speed *= 2.0;
        }
        if ui.input(|i| i.modifiers.ctrl) {
            speed /= 2.0;
        }

        let mut direction = Vec3::ZERO;
        if ui.input(|i| i.key_down(egui::Key::W)) {
            direction += self.front;
        }
        if ui.input(|i| i.key_down(egui::Key::S)) {
            direction -= self.front;
        }

        if ui.input(|i| i.key_down(egui::Key::D)) {
            direction += self.right;
        }
        if ui.input(|i| i.key_down(egui::Key::A)) {
            direction -= self.right;
        }

        if ui.input(|i| i.key_down(egui::Key::Q)) {
            direction -= Vec3::Y * 0.5;
        }
        if ui.input(|i| i.key_down(egui::Key::E)) {
            direction += Vec3::Y * 0.5;
        }

        self.position += direction * speed;

        self.orientation.x = self.orientation.x.clamp(-89.9, 89.9);

        self.update_vectors();
    }

    fn calculate_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.front, Vec3::Y)
    }

    fn zoom(&self) -> f32 {
        1.
    }
}
