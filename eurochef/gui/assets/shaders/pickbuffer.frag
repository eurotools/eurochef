#ifdef GL_ES
    precision mediump float;
#endif

flat in uint o_value;
out vec4 out_color;
void main() {
    float r = float(o_value & uint(0xff)) / 255.0;
    float g = float((o_value & uint(0xff00)) >> 8) / 255.0;
    float b = float((o_value & uint(0xff0000)) >> 16) / 255.0;
    out_color = vec4(r, g, b, 1.0);
}