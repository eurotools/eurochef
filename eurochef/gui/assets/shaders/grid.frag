#ifdef GL_ES
    precision mediump float;
#endif

in vec4 line_color;
out vec4 out_color;
void main() {
    out_color = line_color;
}