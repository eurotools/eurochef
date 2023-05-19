in vec2 f_uv;
in vec4 f_color;

uniform sampler2D u_texture;
uniform float u_cutoutThreshold;

out vec4 o_color;
void main() {
    vec4 texel = texture(u_texture, f_uv);
    if(texel.a <= u_cutoutThreshold) discard;

    o_color = vec4(texel.xyz * f_color.xyz, f_color.a * texel.a);
}