precision mediump float;

out float o_pos;
out float o_length;

uniform vec3 u_start;
uniform vec3 u_end;
uniform mat4 u_view;

void main()
{
    vec3 start = u_start;
    start.x = -start.x;
    vec3 end = u_end;
    end.x = -end.x;

    o_pos = float(gl_VertexID);
    o_length = distance(start, end);
    
    if(gl_VertexID == 0)
        gl_Position = u_view * vec4(start, 1.0);
    else
        gl_Position = u_view * vec4(end, 1.0);
}