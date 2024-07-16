use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use bevy_math::{IVec2, Vec2};
use eframe::{
    egui::Ui,
    glow::{self, Context, HasContext, BLEND, ONE_MINUS_SRC_ALPHA, SRC_ALPHA},
};

use crate::{
    animations::AnimType, prelude::Animation, shader::Shader, texture::Texture, VERTEX_ARRAY
};
const DUCK_GAME_HERTZ: f32 = 60.0;

#[derive(Debug, Clone, Copy)]
struct Uniforms {
    time: f32,
    frames_amount: Vec2,
    frame_size: Vec2,
    current_frame: f32,
}
#[derive(Debug)]
pub struct SpriteDrawer {
    pub anim_index: usize,
    pub frame_index: usize,
    pub frame_timer: f32,
    pub paused: bool,
    pub texture: Texture,
    pub animations: Option<Vec<Rc<Animation>>>,
    pub shader: Shader,
    pub frame_size: IVec2,
    pub scaling_factor: f32,
    default_animation: Rc<Animation>,
}

fn draw_texture(gl: &Context, texture: crate::texture::Inner, shader: Shader, uniforms: Uniforms) {
    let vertex_array = VERTEX_ARRAY.read().unwrap().unwrap();
    unsafe {
        shader.activate(gl);
        shader.set_f32(gl, "current_frame", uniforms.current_frame);
        shader.set_f32(gl, "time", uniforms.time);
        shader.set_i32(gl, "background_type", 1);
        shader.set_vec2(gl, "frame_size", uniforms.frame_size);
        shader.set_vec2(gl, "frames_amount", uniforms.frames_amount);
        gl.enable(BLEND);
        gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture.native));
        gl.bind_vertex_array(Some(vertex_array));
        gl.draw_arrays(glow::TRIANGLES, 0, 6);
    }
}
#[derive(Debug, Clone, Copy)]
pub enum AnimChangeBehaviour {
    Reset,
    Keep,
    Reverse,
}

impl SpriteDrawer {
    pub fn new(
        gl: &Context,
        texture_path: impl AsRef<Path>,
        scaling_factor: f32,
        shader: Shader,
        frame_size: IVec2,
    ) -> Self {
        Self {
            anim_index: 0,
            frame_index: 0,
            frame_timer: 0.0,
            paused: false,
            frame_size,
            texture: Texture::from_path(gl, texture_path).unwrap(),
            animations: None,
            default_animation: Animation::new(AnimType::OnDefault, 1, false, vec![0.into()])
                .into(),
            scaling_factor,
            shader,
        }
    }

    pub fn add_animation(&mut self, anim: Animation) {
        let animations = match &mut self.animations {
            Some(a) => a,
            None => {
                self.animations = Some(vec![]);
                &mut self.animations.as_mut().unwrap()
            }
        };
        animations.push(anim.into());
    }

    pub fn set_anim(&mut self, anim_type: AnimType, behaviour: AnimChangeBehaviour) -> Option<()> {
        let anims = self.animations.as_ref()?;
        let (index, anim) = anims
            .iter()
            .enumerate()
            .find(|(_, a)| a.anim_type == anim_type)?;
        match behaviour {
            AnimChangeBehaviour::Reset => {
                self.frame_index = 0;
            },
            AnimChangeBehaviour::Keep => (),
            AnimChangeBehaviour::Reverse => {
                self.frame_index = anim.frames.len() - self.frame_index;
            },
        }
        self.paused = false;
        self.anim_index = index;

        Some(())
    }

    pub fn draw(&mut self, ui: &mut Ui, time: f32, hertz: f32) {
        self.anim_index = usize::min(
            self.anim_index,
            self.animations
                .as_ref()
                .map(|a| a.len())
                .unwrap_or(0)
                .saturating_sub(1),
        );
        let animation = {
            if let Some(anims) = &self.animations {
                anims.get(self.anim_index).cloned()
            } else {
                Some(self.default_animation.clone())
            }
        };
        if let Some(animation) = &animation {
            if !self.paused {
                self.update(animation, hertz);
            }
        };
        let (rect, _) = ui.allocate_exact_size(
            eframe::egui::Vec2::new(
                self.frame_size.x as f32 * self.scaling_factor,
                self.frame_size.y as f32 * self.scaling_factor,
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
                anim.frames
                    .get(self.frame_index)
                    .map(|f| f.value)
                    .unwrap_or(0)
            })
            .unwrap_or_default() as f32;
        let uniforms = Uniforms {
            current_frame,
            frames_amount: Vec2::new(
                (self.texture.width() / self.frame_size.x) as f32,
                (self.texture.height() / self.frame_size.y) as f32,
            ),
            frame_size: Vec2::new(self.frame_size.x as f32, self.frame_size.y as f32),
            time,
        };
        let inner = self.texture.clone().inner();
        let shader = self.shader.clone();
        let callback = eframe::egui::PaintCallback {
            rect,
            callback: Arc::new(egui_glow::CallbackFn::new(move |_, painter| {
                draw_texture(painter.gl(), inner, shader.clone(), uniforms)
            })),
        };
        ui.painter().add(callback);
    }
    fn update(&mut self, animation: &Animation, hertz: f32) {
        if self.frame_index == animation.frames.len() - 1 && !animation.looping {
            self.paused = true;
        }
        if animation.frames.is_empty() {
            self.frame_index = 0;
            return;
        }
        self.frame_index = usize::min(self.frame_index, animation.frames.len() - 1);
        self.frame_timer += hertz / DUCK_GAME_HERTZ;
        if self.frame_timer > animation.delay as f32 {
            self.frame_timer = 0.0;
            self.frame_index += 1;
            self.frame_index %= animation.frames.len();
        }
    }
}
