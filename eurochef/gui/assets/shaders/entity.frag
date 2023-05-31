precision mediump float;

in vec2 f_uv;
in vec4 f_color;

uniform sampler2D u_texture;
uniform float u_cutoutThreshold;

out vec4 o_color;
void main() {
    vec4 texel = texture(u_texture, f_uv);
    if(texel.a <= u_cutoutThreshold) discard;

#ifdef EC_NO_VERTEX_LIGHTING
    o_color = texel;
#else
    o_color = texel * f_color;
#endif

#ifdef EC_NO_TRANSPARENCY
    o_color.a = 1.0;
#endif
}