#version 430 core
layout(location = 0) in vec2 aPos;

const vec2 verts[6] = vec2[6](
    vec2(0.0, 0.0),
    vec2(0.0, 1.0),
    vec2(1.0, 0.0),
    vec2(1.0, 0.0),
    vec2(0.0, 1.0),
    vec2(1.0, 1.0)
    );

out vec2 tex_coord;
void main() {
    tex_coord = verts[gl_VertexID];
    gl_Position = vec4(aPos, 0.0, 1.0);
}
