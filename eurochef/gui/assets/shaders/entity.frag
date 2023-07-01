precision mediump float;

in vec2 f_uv;
in vec2 f_normalUv;
in vec4 f_color;
in vec3 f_eye;

uniform sampler2D u_texture;
uniform float u_cutoutThreshold;

vec2 matcap(vec3 eye, vec3 normal) {
  vec3 reflected = reflect(eye, normal);
  float m = 2.8284271247461903 * sqrt( reflected.z+1.0 );
  return reflected.xy / m + 0.5;
}

out vec4 o_color;
void main() {
#ifdef EC_MATCAP
    o_color = texture2D(u_texture, f_normalUv) * f_color;
    return;
#endif

    vec4 texel = texture(u_texture, f_uv);
    if(texel.a <= u_cutoutThreshold) discard;

#ifdef EC_NO_VERTEX_LIGHTING
    o_color = texel;
    o_color.a = texel.a * f_color.a;
#else
    o_color = texel * f_color;
#endif

#ifdef EC_NO_TRANSPARENCY
    o_color.a = 1.0;
#endif
}