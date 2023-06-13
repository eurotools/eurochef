#extension GL_ARB_explicit_attrib_location : enable
precision mediump float;

layout (location = 0) in vec3 a_pos;

uniform uint u_id;
uniform uint u_type;

flat out uint o_value;

uniform mat4 u_view;
uniform mat4 u_model;
void main() {
    vec4 new_pos = (u_model * vec4(a_pos, 1.0));
    new_pos.x = - new_pos.x;

    o_value = u_id | (u_type << 20);

    gl_Position = u_view * new_pos;
}