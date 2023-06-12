use glow::HasContext;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum BlendMode {
    None,
    Cutout,
    Blend,
    Additive,
    ReverseSubtract,
}

pub fn set_blending_mode(gl: &glow::Context, blend: BlendMode) {
    unsafe {
        match blend {
            BlendMode::None | BlendMode::Cutout => {
                gl.disable(glow::BLEND);
            }
            _ => gl.enable(glow::BLEND),
        }

        match blend {
            BlendMode::Cutout => {
                gl.enable(glow::SAMPLE_ALPHA_TO_COVERAGE);
            }
            _ => gl.disable(glow::SAMPLE_ALPHA_TO_COVERAGE),
        }

        match blend {
            BlendMode::Blend | BlendMode::Additive | BlendMode::ReverseSubtract => {
                let blend_src = match blend {
                    BlendMode::Blend => glow::SRC_ALPHA,
                    BlendMode::Additive => glow::SRC_ALPHA,
                    BlendMode::ReverseSubtract => glow::SRC_ALPHA,
                    _ => unreachable!(),
                };

                let blend_dst = match blend {
                    BlendMode::Blend => glow::ONE_MINUS_SRC_ALPHA,
                    BlendMode::Additive => glow::ONE,
                    BlendMode::ReverseSubtract => glow::ONE,
                    _ => unreachable!(),
                };

                let blend_func = match blend {
                    BlendMode::Blend => glow::FUNC_ADD,
                    BlendMode::Additive => glow::FUNC_ADD,
                    BlendMode::ReverseSubtract => glow::FUNC_REVERSE_SUBTRACT,
                    _ => unreachable!(),
                };

                gl.blend_equation(blend_func);
                gl.blend_func(blend_src, blend_dst);
            }
            _ => {}
        }
    }
}
