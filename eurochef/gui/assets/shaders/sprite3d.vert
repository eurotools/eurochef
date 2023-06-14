#extension GL_ARB_explicit_attrib_location : enable
precision mediump float;

layout (location = 0) in vec3 a_pos;
layout (location = 1) in vec2 a_uv;

out vec2 f_uv;

uniform mat4 u_view;
uniform mat4 u_model;
void main()
{
    f_uv = a_uv;
    gl_Position = u_view * u_model * vec4(a_pos, 1.0);
}