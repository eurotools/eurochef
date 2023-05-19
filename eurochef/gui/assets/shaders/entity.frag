in vec2 f_uv;
in vec4 f_color;

uniform sampler2D u_texture;

out vec4 o_color;
void main() {
    o_color = f_color * vec4(texture(u_texture, f_uv).xyz, 1.0);
}