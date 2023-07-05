use eurochef_edb::{Hashcode, HashcodeUtils, HC_BASE_ENTITY, HC_BASE_SCRIPT, HC_BASE_TEXTURE};
use eurochef_shared::script::UXGeoScript;
use glam::{Mat4, Quat};
use glow::HasContext;
use nohash_hasher::IntMap;

use crate::entity_frame::RenderableTexture;

use self::{camera::Camera3D, entity::EntityRenderer};

pub mod billboard;
pub mod blend;
pub mod camera;
pub mod entity;
pub mod gl_helper;
pub mod grid;
pub mod pickbuffer;
pub mod script;
pub mod shaders;
pub mod trigger;
pub mod tweeny;
pub mod viewer;

#[derive(Default)]
pub struct RenderUniforms {
    pub view: Mat4,
    pub camera_rotation: Quat,
    pub time: f32,
}

impl RenderUniforms {
    pub fn update<C: Camera3D + ?Sized>(
        &mut self,
        orthographic: bool,
        camera: &mut C,
        aspect_ratio: f32,
        time: f32,
    ) {
        let mut projection = if orthographic {
            glam::Mat4::orthographic_rh_gl(
                (-aspect_ratio * -camera.zoom()) * 2.0,
                (aspect_ratio * -camera.zoom()) * 2.0,
                (1.0 * -camera.zoom()) * 2.0,
                (-1.0 * -camera.zoom()) * 2.0,
                -2500.0,
                2500.0,
            )
        } else {
            glam::Mat4::perspective_rh(90.0_f32.to_radians(), aspect_ratio, 0.02, 2000.0)
        };

        if !orthographic {
            projection.x_axis = -projection.x_axis;
        }

        self.view = projection * camera.calculate_matrix();
        self.camera_rotation = camera.rotation();
        self.time = time;
    }
}

pub unsafe fn start_render(gl: &glow::Context) {
    gl.depth_mask(true);
    gl.clear_depth_f32(1.0);
    gl.clear(glow::DEPTH_BUFFER_BIT);
    gl.cull_face(glow::FRONT);
    gl.enable(glow::DEPTH_TEST);
    gl.depth_func(glow::LEQUAL);
}

pub struct RenderStore {
    files: IntMap<
        Hashcode,
        (
            IntMap<Hashcode, (usize, EntityRenderer)>,
            IntMap<Hashcode, (usize, RenderableTexture)>,
            Vec<UXGeoScript>,
            Vec<Hashcode>, // All loaded hashcodes, used for analysis
        ),
    >,
}

impl RenderStore {
    pub fn new() -> Self {
        Self {
            files: Default::default(),
        }
    }

    pub fn purge(&mut self, purge_memory: bool) {
        self.files.clear();
        if purge_memory {
            self.files.shrink_to_fit()
        }
    }

    pub fn purge_file(&mut self, file: Hashcode) {
        self.files.remove(&file);
    }

    pub fn get_entity(&self, file: Hashcode, entity_hashcode: Hashcode) -> Option<&EntityRenderer> {
        self.files.get(&file).and_then(|v| {
            if entity_hashcode.is_local() {
                v.0.iter()
                    .find(|(_, (v, _))| *v == entity_hashcode.index() as usize)
                    .map(|(_, (_, v))| v)
            } else {
                v.0.get(&entity_hashcode).map(|(_, v)| v)
            }
        })
    }

    pub fn get_script(&self, file: Hashcode, script_hashcode: Hashcode) -> Option<&UXGeoScript> {
        self.files.get(&file).and_then(|v| {
            if script_hashcode.is_local() {
                v.2.get(script_hashcode.index() as usize)
            } else {
                v.2.iter().find(|v| v.hashcode == script_hashcode)
            }
        })
    }

    // pub fn iter_entities(&self, file: Hashcode) -> Option<Iter<u32, EntityRenderer>> {
    //     self.files.get(&file).map(|v| v.0.iter())
    // }

    pub fn get_texture(
        &self,
        file: Hashcode,
        texture_hashcode: Hashcode,
    ) -> Option<&RenderableTexture> {
        self.files
            .get(&file)
            .and_then(|v| v.1.get(&texture_hashcode).map(|(_, v)| v))
    }

    pub fn get_texture_by_index(
        &self,
        file: Hashcode,
        index: usize,
    ) -> Option<(u32, &RenderableTexture)> {
        self.files.get(&file).and_then(|v| {
            v.1.iter()
                .find(|(_, (v, _))| *v == index)
                .map(|(hc, (_, v))| (*hc, v))
        })
    }

    fn insert_hashcode(&mut self, file: Hashcode, hashcode: Hashcode) {
        if let Some(v) = self.files.get_mut(&file) {
            v.3.push(hashcode);
        }
    }

    pub fn is_file_loaded(&self, file: Hashcode) -> bool {
        self.files.contains_key(&file)
    }

    pub fn is_object_loaded(&self, file: Hashcode, hashcode: Hashcode) -> bool {
        match hashcode.base() {
            HC_BASE_ENTITY | HC_BASE_SCRIPT | HC_BASE_TEXTURE => {}
            v => {
                debug!("Checked load for unknown object type 0x{v:x} (hc {hashcode:08x})");
                return true;
            }
        }

        self.files
            .get(&file)
            .map(|f| f.3.contains(&hashcode))
            .unwrap_or(false)
    }

    pub fn insert_entity(
        &mut self,
        file: Hashcode,
        entity_hashcode: Hashcode,
        index: usize,
        entity: EntityRenderer,
    ) {
        let file_entry = match self.files.entry(file) {
            std::collections::hash_map::Entry::Occupied(o) => &mut o.into_mut().0,
            std::collections::hash_map::Entry::Vacant(v) => &mut v.insert(Default::default()).0,
        };

        file_entry.insert(entity_hashcode, (index, entity));
        self.insert_hashcode(file, entity_hashcode);
    }

    pub fn insert_texture(
        &mut self,
        file: Hashcode,
        texture_hashcode: Hashcode,
        index: usize,
        texture: RenderableTexture,
    ) {
        let file_entry = match self.files.entry(file) {
            std::collections::hash_map::Entry::Occupied(o) => &mut o.into_mut().1,
            std::collections::hash_map::Entry::Vacant(v) => &mut v.insert(Default::default()).1,
        };

        file_entry.insert(texture_hashcode, (index, texture));
        self.insert_hashcode(file, texture_hashcode);
    }

    pub fn insert_script(&mut self, file: Hashcode, script: UXGeoScript) {
        let file_entry = match self.files.entry(file) {
            std::collections::hash_map::Entry::Occupied(o) => &mut o.into_mut().2,
            std::collections::hash_map::Entry::Vacant(v) => &mut v.insert(Default::default()).2,
        };

        let script_hashcode = script.hashcode;
        file_entry.push(script);
        self.insert_hashcode(file, script_hashcode);
    }
}
