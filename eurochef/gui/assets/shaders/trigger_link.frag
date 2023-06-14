#ifdef GL_ES
    precision mediump float;
#endif

#define SCROLL_SPEED 0.50f
#define LINE_LENGTH 0.30f
#define LINE_LENGTH_HALF (LINE_LENGTH / 2.0f)

in float o_pos; // Scalar 0-1
in float o_length;

out vec4 o_color;

uniform float u_time;
uniform float u_scale;
uniform vec3 u_color;
void main() {
    float pos_scaled = (1.0f - o_pos) * o_length;
    pos_scaled += u_scale * SCROLL_SPEED * u_time;

    if(mod(pos_scaled, (u_scale * LINE_LENGTH)) < (u_scale * LINE_LENGTH_HALF))
        o_color = vec4(u_color, 1.0);
    else
        discard;
}