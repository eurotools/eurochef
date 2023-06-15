#ifdef GL_ES
    precision mediump float;
#endif

uniform vec4 u_color;
out vec4 out_color;
void main() {
    out_color = u_color;
}