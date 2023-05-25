use glow::HasContext;

pub unsafe fn compile_shader(
    gl: &glow::Context,
    shader_sources: &[(u32, &str)],
) -> Result<glow::Program, String> {
    let shader_version = egui_glow::ShaderVersion::get(gl);
    let program = gl.create_program().expect("Cannot create program");
    let _shaders: Vec<_> = shader_sources
        .iter()
        .map(|(shader_type, shader_source)| {
            let shader = gl
                .create_shader(*shader_type)
                .expect("Cannot create shader");
            gl.shader_source(
                shader,
                &format!(
                    "{}\n{}",
                    shader_version.version_declaration(),
                    shader_source
                ),
            );
            gl.compile_shader(shader);
            assert!(
                gl.get_shader_compile_status(shader),
                "Failed to compile custom_3d_glow {shader_type}: {}",
                gl.get_shader_info_log(shader)
            );

            gl.attach_shader(program, shader);
            shader
        })
        .collect();

    gl.link_program(program);

    Ok(program)
}

pub unsafe fn load_texture(
    gl: &glow::Context,
    width: i32,
    height: i32,
    data: &[u8],
    format: u32,
    flags: u32,
) -> glow::Texture {
    let texture = gl.create_texture().unwrap();
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));

    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGBA as i32,
        width,
        height,
        0,
        format,
        glow::UNSIGNED_BYTE,
        Some(data),
    );

    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);

    // TODO: Bitflags
    if (flags & 0x100000) != 0 {
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
    }
    if (flags & 0x200000) != 0 {
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
    }

    gl.generate_mipmap(glow::TEXTURE_2D);

    texture
}
