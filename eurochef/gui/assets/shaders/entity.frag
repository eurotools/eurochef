precision mediump float;

in vec2 f_uv;
in vec4 f_color;

uniform sampler2D u_texture;
uniform float u_cutoutThreshold;

out vec4 o_color;
void main() {
    vec4 texel = texture(u_texture, f_uv);
    if(texel.a <= u_cutoutThreshold) discard;

    vec4 vertexColor = vec4(f_color * 255.0);
    vec4 textureColor = vec4(texel * 255.0);
    vec3 _314 = clamp(
        (
            (
                (
                    (
                        (textureColor.xyz * (
                                vertexColor.xyz + (vertexColor.xyz / 128.0)
                            )
                        ) * 2.0
                    )
                ) + 128.0
            ) / 256.0
        ), 
        0.0, 
        255.0
    );

    o_color.xyz = _314 / 255.0;
    o_color.a = f_color.a * texel.a;
}