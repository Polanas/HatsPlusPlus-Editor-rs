#version 430 core
out vec4 frag_color;
in vec2 tex_coord;

uniform sampler2D texture;
uniform vec2 frames_amount;
uniform vec2 frame_size;
uniform float current_frame;
uniform float time;

vec2 index_to_position(float index, float width) {
    float x = round(mod(index, width));
    float y = round((index - x) / width);
    return vec2(x,y);
}

const vec3 GRID_COL1 = vec3(192)/255;
const vec3 GRID_COL2 = vec3(128)/255;

//this shader is cursed
void main()
{
    vec2 uv = tex_coord;
    vec2 tex_size = textureSize(texture, 0);
    vec2 pos = index_to_position(current_frame, frames_amount.x);
    vec2 pixel_size = 1.0 / tex_size;
    uv.y = 1 - uv.y;
    uv = floor(uv / pixel_size) * pixel_size;
    vec2 grid_uv = uv;
    grid_uv += .5 * pixel_size;
    grid_uv /= tex_size;
    // adjust uv to be in the center of a pixel 
    uv += .5 * pixel_size;
    // now uv covers one pixel
    uv /= tex_size;
    // move to current frame
    uv += pixel_size * pos;
    // ...and now, one frame
    uv *= frame_size;
    grid_uv *= frame_size;
    grid_uv.x += mod(grid_uv.y, pixel_size.y * 16) > pixel_size.y * 8 ? pixel_size.x * 8 : 0;
    float grid_state = float(mod(grid_uv.x, pixel_size.x * 16.0) > pixel_size.x * 8);
    vec3 grid_col = grid_state > 0 ? GRID_COL1 : GRID_COL2;
    vec4 tex_col = texture2D(texture, uv);
    frag_color = tex_col.a <= 0.0 ? vec4(grid_col, 1.0) : mix(vec4(grid_col,1.0), tex_col, vec4(tex_col.a));
}
