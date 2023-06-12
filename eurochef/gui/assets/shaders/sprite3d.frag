precision mediump float;

in vec2 f_uv;

uniform sampler2D u_texture;

out vec4 o_color;
void main() {
    o_color = texture(u_texture, f_uv);
}