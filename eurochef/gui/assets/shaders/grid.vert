precision mediump float;

const vec2 verts[3] = vec2[3](
    vec2(0.0, 1.0),
    vec2(-1.0, -1.0),
    vec2(1.0, -1.0)
);

uniform int u_size;
uniform mat4 u_view;
void main() {
    bool is_horizontal = (gl_VertexID % 4) < 2;
    bool is_second_vertex = (gl_VertexID & 1) != 0;
    int index = gl_VertexID / 4;
    
    vec3 vert = vec3(0.0, index, 0.0);
    if(is_second_vertex) {
        vert.x += float(u_size);
    }

    vert -= vec3(vec2(float(u_size+1) / 2.0), 0.0);
    vert *= 2.0;

    if(is_horizontal) {
        gl_Position = u_view * vec4(vert.xyz, 1.0);
    } else {
        gl_Position = u_view * vec4(vert.yxz, 1.0);
    }
}