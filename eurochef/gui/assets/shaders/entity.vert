#extension GL_ARB_explicit_attrib_location : enable
precision mediump float;

layout (location = 0) in vec3 a_pos;
layout (location = 1) in vec3 a_normal;
layout (location = 2) in vec2 a_uv;
layout (location = 3) in vec4 a_col;

out vec2 f_uv;
out vec2 f_normalUv;
out vec4 f_color;
out vec3 f_eye;

// TODO?
// vec4 unpackRGBA(uint packedValue) {
//     float divisor = 1.0 / 255.0;
//     uint r = (packedValue >> 24u) & 0xFFu;
//     uint g = (packedValue >> 16u) & 0xFFu;
//     uint b = (packedValue >> 8u) & 0xFFu;
//     uint a = packedValue & 0xFFu;
//     return vec4(float(r) * divisor, float(g) * divisor, float(b) * divisor, float(a) * divisor);
// }

uniform vec2 u_scroll;
uniform mat4 u_view;
uniform mat4 u_model;
uniform mat4 u_normal;
void main()
{
    f_uv = a_uv + u_scroll;
    f_color = vec4(a_col.xyz * 2.0, a_col.a);

#ifdef EC_MATCAP
    vec4 p = vec4(a_pos, 1.0);
    vec3 e = normalize( vec3( (u_view * u_model) * p ) );
    vec3 n = normalize( u_normal * vec4(a_normal, 1.0) ).xyz;

    vec3 r = reflect( e, n );
    float m = 2. * sqrt(
        pow( r.x, 2. ) +
        pow( r.y, 2. ) +
        pow( r.z + 1., 2. )
    );
    f_normalUv = (r.xy / m + .5) * 2.0;
#endif

    // vec4 mv_pos = (u_view * u_model) * vec4(a_pos, 1.0 );
    // f_eye = normalize(mv_pos.xyz);

    gl_Position = u_view * u_model * vec4(a_pos, 1.0);
}