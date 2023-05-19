#extension GL_ARB_explicit_attrib_location : enable

layout (location = 0) in vec3 a_pos;
layout (location = 1) in vec3 a_normal;
layout (location = 2) in vec2 a_uv;
layout (location = 3) in vec4 a_col;

out vec2 f_uv;
out vec4 f_color;

// TODO?
// vec4 unpackRGBA(uint packedValue) {
//     float divisor = 1.0 / 255.0;
//     uint r = (packedValue >> 24u) & 0xFFu;
//     uint g = (packedValue >> 16u) & 0xFFu;
//     uint b = (packedValue >> 8u) & 0xFFu;
//     uint a = packedValue & 0xFFu;
//     return vec4(float(r) * divisor, float(g) * divisor, float(b) * divisor, float(a) * divisor);
// }

uniform mat4 u_view;
uniform mat4 u_model;
void main()
{
    f_uv = a_uv;
    f_color = a_col * 2.0;
    vec3 new_pos = (u_model * vec4(a_pos.zxy, 1.0)).xyz;
    new_pos.x = -new_pos.x;

    gl_Position = u_view * vec4(new_pos, 1.0);
}