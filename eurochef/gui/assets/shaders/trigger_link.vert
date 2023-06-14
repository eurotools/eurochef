precision mediump float;

out float o_pos;
out float o_length;

uniform vec3 u_start;
uniform vec3 u_end;
uniform mat4 u_view;

void main()
{
    o_pos = float(gl_VertexID);
    o_length = distance(u_start, u_end);
    
    if(gl_VertexID == 0)
        gl_Position = u_view * vec4(u_start, 1.0);
    else
        gl_Position = u_view * vec4(u_end, 1.0);
}