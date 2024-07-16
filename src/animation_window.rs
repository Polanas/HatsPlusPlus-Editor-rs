use std::cell::Ref;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use bevy_math::IVec2;
use bevy_math::Vec2;
use eframe::egui::{Button, CollapsingHeader, Id, Pos2, Rect, Ui, Window};
use eframe::glow::Context;
use eframe::glow::{self, HasContext};
use once_cell::sync::Lazy;

use crate::animations::AnimType;
use crate::{animations::Animation, shader::Shader, texture::Texture, VERTEX_ARRAY};
use crate::{egui_utils, AnimationWindowAction};

const DUCK_GAME_HERTZ: f32 = 60.0;
const MAX_SYMBOL_WIDTH: i32 = 16;
pub const TEXTURES_SCALE_FACTOR: f32 = 5.0;

#[derive(Debug)]
pub struct AnimationWindow {
    pub current_anim_index: usize,
    pub current_frame_index: usize,
    frame_timer: f32,
    paused: bool,
    default_anim: AnimationCell,
}

#[derive(Debug, Clone, Copy)]
struct Uniforms {
    offset: Vec2,
    time: f32,
    frames_amount: Vec2,
    frame_size: Vec2,
    current_frame: f32,
}

fn draw_texture(gl: &Context, texture: crate::texture::Inner, shader: Shader, uniforms: Uniforms) {
    let vertex_array = VERTEX_ARRAY.read().unwrap().unwrap();
    unsafe {
        shader.activate(gl);
        shader.set_i32(gl, "background_type", 0);
        shader.set_f32(gl, "current_frame", uniforms.current_frame);
        shader.set_f32(gl, "time", uniforms.time);
        shader.set_vec2(gl, "frame_size", uniforms.frame_size);
        shader.set_vec2(gl, "offset", uniforms.offset);
        shader.set_vec2(gl, "frames_amount", uniforms.frames_amount);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture.native));
        gl.bind_vertex_array(Some(vertex_array));
        gl.draw_arrays(glow::TRIANGLES, 0, 6);
    }
}

type AnimationCell = Rc<RefCell<Animation>>;

pub struct AnimationWindowFrameData<'a> {
    pub time: f32,
    pub ui: &'a Ui,
    pub shader: Shader,
    pub hertz: f32,
    pub animations: Option<Vec<AnimationCell>>,
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
            default_anim: RefCell::new(Animation::new(
                AnimType::OnDefault,
                1,
                false,
                vec![0.into()],
            ))
            .into(),
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
        let animation = {
            if let Some(ref anims) = data.animations {
                anims.get(self.current_anim_index).cloned()
            } else {
                Some(self.default_anim.clone())
            }
        };
        if let Some(animation) = &animation {
            if !self.paused {
                self.update(animation.borrow(), data.hertz);
            }
        };
        let frame_screen_width = data.frame_size.x as f32 * TEXTURES_SCALE_FACTOR;
        let hat_name =
            if (data.hat_name.len() as i32 * MAX_SYMBOL_WIDTH) as f32 > frame_screen_width {
                format!(
                    "{}..",
                    data.hat_name
                        .chars()
                        .take(((frame_screen_width) / MAX_SYMBOL_WIDTH as f32) as usize - 1)
                        .collect::<String>()
                )
            } else {
                data.hat_name
            };
        let window_id = Id::new(data.texture.path().unwrap());
        let offset = data
            .ui
            .ctx()
            .memory_mut(|m| m.area_rect(window_id))
            .unwrap_or(Rect::from_min_max(Pos2::ZERO, Pos2::ZERO));
        Window::new(hat_name)
            .id(window_id)
            .resizable(false)
            .max_width(data.frame_size.x as f32 * TEXTURES_SCALE_FACTOR)
            .show(data.ui.ctx(), |ui| {
                if data.animations.as_ref().map(|a| a.len()).unwrap_or(0) > 0 {
                    CollapsingHeader::new("Animations").show(ui, |ui| {
                        for (i, anim) in data
                            .animations
                            .as_ref()
                            .map(|a| a.iter())
                            .unwrap_or_default()
                            .enumerate()
                        {
                            let anim_name = anim.borrow().anim_type.to_string();
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
                }
                let (rect, _) = ui.allocate_exact_size(
                    eframe::egui::Vec2::new(
                        data.frame_size.x as f32 * TEXTURES_SCALE_FACTOR,
                        data.frame_size.y as f32 * TEXTURES_SCALE_FACTOR,
                    ),
                    eframe::egui::Sense {
                        click: false,
                        drag: false,
                        focusable: false,
                    },
                );
                let current_frame = animation
                    .as_ref()
                    .map(|anim| {
                        anim.borrow()
                            .frames
                            .get(self.current_frame_index)
                            .map(|f| f.value)
                            .unwrap_or(0)
                    })
                    .unwrap_or_default() as f32;
                let uniforms = Uniforms {
                    offset: (offset.max.x, offset.min.y).into(),
                    current_frame,
                    frames_amount: Vec2::new(
                        (data.texture.width() / data.frame_size.x) as f32,
                        (data.texture.height() / data.frame_size.y) as f32,
                    ),
                    frame_size: Vec2::new(data.frame_size.x as f32, data.frame_size.y as f32),
                    time: data.time,
                };
                let inner = data.texture.clone().inner();
                let callback = eframe::egui::PaintCallback {
                    rect,
                    callback: Arc::new(egui_glow::CallbackFn::new(move |_, painter| {
                        draw_texture(painter.gl(), inner, data.shader.clone(), uniforms)
                    })),
                };
                ui.painter().add(callback);
                let Some(animation) = animation.as_ref().map(|a| a.borrow()) else {
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
                        self.current_frame_index + 1,
                        animation.frames.len()
                    ));
                });
                self.current_frame_index %= animation.frames.len();
            });
    }
    fn update(&mut self, animation: Ref<Animation>, hertz: f32) {
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
