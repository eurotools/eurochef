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
