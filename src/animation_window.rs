use std::sync::Arc;

use bevy_math::{vec2, IVec2, Vec2};
use eframe::egui::{DragValue, Id, Ui, Window};
use eframe::glow::Context;
use eframe::glow::{self, HasContext};

use crate::{animation::Animation, shader::Shader, texture::Texture, VERTEX_ARRAY};

const DUCK_GAME_HERTZ: f32 = 60.0;

#[derive(Debug)]
pub struct AnimationWindow {
    pub current_anim_index: usize,
    pub current_frame_index: usize,
    frame_timer: f32,
}

#[derive(Debug, Clone, Copy)]
struct Uniforms {
    frames_amount: Vec2,
    frame_size: Vec2,
    current_frame: f32,
}

fn draw_texture(gl: &Context, texture: crate::texture::Inner, shader: Shader, uniforms: Uniforms) {
    let vertex_array = VERTEX_ARRAY.read().unwrap().unwrap();
    unsafe {
        shader.activate(gl);
        shader.set_f32(gl, "current_frame", uniforms.current_frame);
        shader.set_vec2(gl, "frame_size", uniforms.frame_size);
        shader.set_vec2(gl, "frames_amount", uniforms.frames_amount);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture.native));
        gl.bind_vertex_array(Some(vertex_array));
        gl.draw_arrays(glow::TRIANGLES, 0, 6);
    }
}

pub struct AnimationWindowFrameData<'a> {
    pub ui: &'a Ui,
    pub shader: Shader,
    pub hertz: f32,
    pub animations: Vec<Animation>,
    pub texture: Texture,
    pub frame_size: IVec2,
    pub hat_name: String,
}

impl AnimationWindow {
    pub fn new() -> Self {
        Self {
            current_anim_index: 0,
            current_frame_index: 0,
            frame_timer: 0.0,
        }
    }
    pub fn draw(&mut self, data: AnimationWindowFrameData) {
        let Some(animation) = data.animations.get(self.current_anim_index) else {
            return;
        };
        self.update(animation, data.hertz);
        Window::new(data.hat_name)
            .id(Id::new(data.texture.path().unwrap()))
            .resizable(false)
            .show(data.ui.ctx(), |ui| {
                ui.add(
                    DragValue::new(&mut self.current_anim_index)
                        .clamp_range(0..=(data.animations.len() - 1)),
                );
                let (rect, _) = ui.allocate_exact_size(
                    eframe::egui::Vec2::new(
                        data.frame_size.x as f32 * 5.0,
                        data.frame_size.y as f32 * 5.0,
                    ),
                    eframe::egui::Sense {
                        click: false,
                        drag: false,
                        focusable: false,
                    },
                );
                let uniforms = Uniforms {
                    current_frame: animation.frames[self.current_frame_index] as f32,
                    frames_amount: Vec2::new(
                        (data.texture.width() / data.frame_size.x) as f32,
                        (data.texture.height() / data.frame_size.y) as f32,
                    ),
                    frame_size: Vec2::new(data.frame_size.x as f32, data.frame_size.y as f32),
                };
                let inner = data.texture.clone().inner();
                let callback = eframe::egui::PaintCallback {
                    rect,
                    callback: Arc::new(egui_glow::CallbackFn::new(move |_, painter| {
                        draw_texture(painter.gl(), inner, data.shader.clone(), uniforms)
                    })),
                };
                ui.painter().add(callback);
            });
    }

    fn update(&mut self, animation: &Animation, hertz: f32) {
        self.frame_timer += hertz / DUCK_GAME_HERTZ;
        if self.frame_timer > animation.delay as f32 {
            self.frame_timer = 0.0;
            self.current_frame_index += 1;
            self.current_frame_index %= animation.frames.len();
        }
    }
}
