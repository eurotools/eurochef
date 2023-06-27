use glow::Program;

use super::gl_helper;

pub struct Shaders {
    pub entity_simple: Program,
    pub entity_simple_unlit: Program,

    pub grid: Program,
    pub pickbuffer: Program,
    pub sprite3d: Program,

    pub select_cube: Program,
    pub trigger_link: Program,
}

impl Shaders {
    pub fn load_shaders(ctx: &glow::Context) -> Shaders {
        macro_rules! compile_shader {
            ($name:expr, $defines:expr) => {
                gl_helper::compile_shader(
                    ctx,
                    &[
                        (
                            glow::VERTEX_SHADER,
                            include_str!(concat!("../../assets/shaders/", $name, ".vert")),
                        ),
                        (
                            glow::FRAGMENT_SHADER,
                            include_str!(concat!("../../assets/shaders/", $name, ".frag")),
                        ),
                    ],
                    $defines,
                )
                .expect("Failed to compile shader")
            };
            ($name:expr) => {
                compile_shader!($name, &[])
            };
        }

        Shaders {
            entity_simple: compile_shader!("entity"),
            entity_simple_unlit: compile_shader!("entity", &["#define EC_NO_VERTEX_LIGHTING"]),
            grid: compile_shader!("grid"),
            pickbuffer: compile_shader!("pickbuffer"),
            sprite3d: compile_shader!("sprite3d"),
            select_cube: compile_shader!("select_cube"),
            trigger_link: compile_shader!("trigger_link"),
        }
    }
}
