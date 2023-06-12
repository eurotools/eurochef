precision mediump float;

in vec2 f_uv;

uniform sampler2D u_texture;

out vec4 o_color;
void main() {
    vec4 texel = texture(u_texture, f_uv);
    if(texel.a <= 0.5) discard;
    o_color = texel;
}