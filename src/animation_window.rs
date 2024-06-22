use std::sync::Arc;

use bevy_math::{vec2, IVec2, Vec2};
use eframe::egui::{Button, CollapsingHeader, DragValue, Id, ImageButton, Ui, Window};
use eframe::glow::Context;
use eframe::glow::{self, HasContext};
use num_traits::CheckedSub;
use once_cell::sync::Lazy;

use crate::prelude::AnimationType;
use crate::{animations, egui_utils, AnimationWindowAction};
use crate::{animations::Animation, shader::Shader, texture::Texture, VERTEX_ARRAY};

const DUCK_GAME_HERTZ: f32 = 60.0;
const DEFAULT_ANIMATION: Lazy<Animation> =
    Lazy::new(|| Animation::new(AnimationType::OnDefault, 1, false, vec![0.into()]));

#[derive(Debug)]
pub struct AnimationWindow {
    pub current_anim_index: usize,
    pub current_frame_index: usize,
    frame_timer: f32,
    paused: bool,
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
    pub animations: Option<Vec<Animation>>,
    pub texture: Texture,
    pub frame_size: IVec2,
    pub hat_name: String,
    pub anim_window_action: AnimationWindowAction,
}

impl AnimationWindow {
    pub fn new() -> Self {
        Self {
            paused: false,
            current_anim_index: 0,
            current_frame_index: 0,
            frame_timer: 0.0,
        }
    }
    pub fn draw(&mut self, data: AnimationWindowFrameData) {
        self.current_anim_index = usize::min(
            self.current_anim_index,
            data.animations
                .as_ref()
                .map(|a| a.len())
                .unwrap_or(0)
                .saturating_sub(1),
        );
        #[allow(clippy::borrow_interior_mutable_const)]
        let default_animation = &*DEFAULT_ANIMATION;
        let animation = {
            if let Some(anims) = &data.animations {
                anims.get(self.current_anim_index)
            } else {
                Some(default_animation)
            }
        };
        if let Some(animation) = animation {
            if !self.paused {
                self.update(animation, data.hertz);
            }
        };
        Window::new(data.hat_name)
            .id(Id::new(data.texture.path().unwrap()))
            .resizable(false)
            .max_width(data.frame_size.x as f32 * 5.0)
            .show(data.ui.ctx(), |ui| {
                CollapsingHeader::new("Animations").show(ui, |ui| {
                    for (i, anim) in data
                        .animations
                        .as_ref()
                        .map(|a| a.iter())
                        .unwrap_or_default()
                        .enumerate()
                    {
                        let anim_name = anim.anim_type.to_string();
                        ui.scope(|ui| {
                            if i == self.current_anim_index {
                                let widgets = &mut ui.style_mut().visuals.widgets;
                                widgets.inactive = widgets.active;
                            }
                            if ui.button(anim_name).clicked() {
                                self.current_anim_index = i;
                            }
                        });
                    }
                });
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
                let current_frame = animation
                    .map(|anim| {
                        anim.frames
                            .get(self.current_frame_index)
                            .map(|f| f.value)
                            .unwrap_or(0)
                    })
                    .unwrap_or_default() as f32;
                let uniforms = Uniforms {
                    current_frame,
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
                let Some(animation) = animation else {
                    return;
                };
                if animation.frames.len() < 2 {
                    return;
                }
                egui_utils::centered(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    if ui
                        .add(Button::new("⬅").min_size(eframe::egui::Vec2::splat(22.0)))
                        .clicked()
                        || matches!(
                            data.anim_window_action,
                            AnimationWindowAction::DecreaseFrame
                        )
                    {
                        self.paused = true;
                        self.current_frame_index = self
                            .current_frame_index
                            .checked_sub(1)
                            .unwrap_or(animation.frames.len() - 1);
                    }
                    let pause_icon = match self.paused {
                        true => "▶",
                        false => "󰏤",
                    };
                    if ui
                        .add(Button::new(pause_icon).min_size(eframe::egui::Vec2::splat(22.0)))
                        .clicked()
                        || matches!(data.anim_window_action, AnimationWindowAction::Pause)
                    {
                        self.paused = !self.paused;
                    }
                    if ui
                        .add(Button::new("➡").min_size(eframe::egui::Vec2::splat(22.0)))
                        .clicked()
                        || matches!(
                            data.anim_window_action,
                            AnimationWindowAction::IncreaseFrame
                        )
                    {
                        self.paused = true;
                        self.current_frame_index += 1;
                    }
                });
                ui.vertical_centered(|ui| {
                    ui.label(format!(
                        "Frame {0} / {1}",
                        self.current_frame_index,
                        animation.frames.len()
                    ));
                });
                self.current_frame_index %= animation.frames.len();
            });
    }
    fn update(&mut self, animation: &Animation, hertz: f32) {
        if animation.frames.is_empty() {
            self.current_frame_index = 0;
            return;
        }
        self.current_frame_index = usize::min(self.current_frame_index, animation.frames.len() - 1);
        self.frame_timer += hertz / DUCK_GAME_HERTZ;
        if self.frame_timer > animation.delay as f32 {
            self.frame_timer = 0.0;
            self.current_frame_index += 1;
            self.current_frame_index %= animation.frames.len();
        }
    }
}
