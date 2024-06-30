#![allow(dead_code)]
use std::cell::RefCell;
use std::sync::Arc;
use std::{collections::HashMap, path::Path, rc::Rc};

use bevy_math::IVec2;
use eframe::egui::Ui;
use eframe::glow::{
    self, HasContext, NativeBuffer, NativeFramebuffer, NativeVertexArray, PixelUnpackData,
};
use eframe::{egui::Color32, glow::Context};
use pixas::{bitmap::Bitmap, pixel::Pixel};

use crate::file_utils::FileStemString;
use crate::shader::Shader;
use crate::sprite::Sprite;
use crate::texture::Texture;

trait ToColor32 {
    fn to_color32(&self) -> Color32;
}
trait ToPixel {
    fn to_pixel(&self) -> Pixel;
}

impl ToColor32 for Pixel {
    fn to_color32(&self) -> Color32 {
        Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }
}
impl ToPixel for Color32 {
    fn to_pixel(&self) -> Pixel {
        Pixel::from_rgba(self.r(), self.g(), self.b(), self.a())
    }
}

#[derive(Debug)]
pub struct Bitmaps {
    pub maps: HashMap<String, Rc<Bitmap>>,
}

impl Bitmaps {
    pub fn new() -> Self {
        Self {
            maps: HashMap::new(),
        }
    }
    pub fn add_bitmap(&mut self, path: impl AsRef<Path>) -> Option<Rc<Bitmap>> {
        eprintln!();
        if let Ok(bitmap) = Bitmap::from_path(&path) {
            if let Some(name) = path.as_ref().file_stem_string() {
                let bitmap = Rc::new(bitmap);
                self.maps.insert(name, bitmap.clone());
                return Some(bitmap);
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ScreenUpdate {
    Clear,
    Preserve,
}
#[derive(Debug, Clone)]
struct RenderData {
    vertex_array: NativeVertexArray,
    shader: Shader,
    vertex_buffer: NativeBuffer,
    screen_texture: Texture,
    frame_buffer: NativeFramebuffer,
}

pub const RENDERER_SCREEN_SIZE: IVec2 = IVec2::splat(200);

impl RenderData {
    fn new(gl: &Context) -> Self {
        let vertices: [f32; 12] = [
            -1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0,
        ];
        let frag = include_str!("room_shader/frag.glsl");
        let vert = include_str!("room_shader/vert.glsl");
        let shader = Shader::from_text_with_path(
            gl,
            "src/room_shader/frag.glsl",
            frag,
            "src/room_shader/vert.glsl",
            vert,
        )
        .unwrap();
        let texture = Texture::with_size(gl, RENDERER_SCREEN_SIZE).unwrap();
        unsafe {
            let vertices_u8: &[u8] = core::slice::from_raw_parts(
                vertices.as_ptr() as *const u8,
                vertices.len() * core::mem::size_of::<f32>(),
            );
            let vertex_array = gl.create_vertex_array().ok();
            gl.bind_vertex_array(vertex_array);

            let frame_buffer = gl.create_framebuffer().ok();
            gl.bind_framebuffer(glow::FRAMEBUFFER, frame_buffer);
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture.native()),
                0,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            let vertex_buffer = gl.create_buffer().ok();
            gl.bind_buffer(glow::ARRAY_BUFFER, vertex_buffer);
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, vertices_u8, glow::STATIC_DRAW);
            gl.vertex_attrib_pointer_f32(
                0,
                2,
                glow::FLOAT,
                false,
                2 * core::mem::size_of::<f32>() as i32,
                0,
            );
            gl.enable_vertex_attrib_array(0);
            Self {
                frame_buffer: frame_buffer.unwrap(),
                vertex_array: vertex_array.unwrap(),
                shader: shader.clone(),
                vertex_buffer: vertex_buffer.unwrap(),
                screen_texture: texture.clone(),
            }
        }
    }
}

thread_local! {
    static RENDER_DATA: RefCell<Option<RenderData>> = const { RefCell::new(None) };
}

fn render_data(gl: &Context) -> RenderData {
    RENDER_DATA.with_borrow_mut(|d| {
        if d.is_none() {
            *d = Some(RenderData::new(gl));
        }
        d.as_mut().unwrap().clone()
    })
}

#[derive(Debug)]
pub struct Renderer {
    bitmaps: Bitmaps,
    sprites: Vec<Sprite>,
    screen: Bitmap,
    screen_update: ScreenUpdate,
}

impl Renderer {
    pub fn new(size: IVec2, screen_update: ScreenUpdate) -> Self {
        Self {
            bitmaps: Bitmaps::new(),
            sprites: vec![],
            screen: Bitmap::with_size(size.x as u32, size.y as u32),
            screen_update,
        }
    }
    pub fn add_bitmap(&mut self, path: impl AsRef<Path>) -> Option<Rc<Bitmap>> {
        self.bitmaps.add_bitmap(path)
    }
    pub fn bitmap(&self, name: &str) -> Option<Rc<Bitmap>> {
        self.bitmaps.maps.get(name).cloned()
    }
    pub fn sprite(&mut self, sprite: &Sprite) {
        self.sprites.push(sprite.clone());
    }
    pub fn draw(&mut self, gl: &Context, ui: &mut Ui) {
        self.draw_sprites();
        self.draw_to_texture(gl);
        self.draw_ui(gl, ui);
    }

    fn draw_ui(&mut self, gl: &Context, ui: &mut Ui) {
        let render_data = render_data(gl);
        let shader = render_data.shader;
        let texture = render_data.screen_texture;
        let (rect, _) = ui.allocate_exact_size(
            eframe::egui::Vec2::new(
                self.screen.width as f32 * 2.0,
                self.screen.height as f32 * 5.0,
            ),
            eframe::egui::Sense {
                click: false,
                drag: false,
                focusable: false,
            },
        );
        let inner = texture.clone().inner();
        let callback = eframe::egui::PaintCallback {
            rect,
            callback: Arc::new(egui_glow::CallbackFn::new(move |_, painter| {
                Renderer::draw_texture_gl(inner, painter.gl(), shader.clone())
            })),
        };
        ui.painter().add(callback);
    }

    fn draw_texture_gl(_screen: crate::texture::Inner, _gl: &Context, _shader: Shader) {}

    fn draw_to_texture(&mut self, gl: &Context) {
        let render_data = render_data(gl);
        let screen_texture = render_data.screen_texture;
        let pixels_data = self.screen.get_pixel_data();
        unsafe {
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                screen_texture.width(),
                screen_texture.height(),
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                PixelUnpackData::Slice(pixels_data),
            );
        }
    }

    fn draw_sprites(&mut self) {
        if matches!(self.screen_update, ScreenUpdate::Clear) {
            self.screen.clear();
        }
        self.sprites
            .sort_by(|s1, s2| s1.depth.0.partial_cmp(&s2.depth.0).unwrap());
        for sprite in &self.sprites {
            Self::draw_sprite(&mut self.screen, sprite);
        }
        self.sprites.clear();
    }
    fn draw_sprite(screen: &mut Bitmap, sprite: &Sprite) {
        let pos = IVec2::new(
            sprite.position.x.floor() as i32,
            sprite.position.y.floor() as i32,
        );
        let size = IVec2::new(sprite.size.x.floor() as i32, sprite.size.y.floor() as i32);
        for x in 0..size.x {
            for y in 0..size.y {
                let col = sprite
                    .bitmap
                    .as_ref()
                    .and_then(|b| b.get_pixel(x, y))
                    .map(|p| p.to_color32())
                    .unwrap_or_else(|| sprite.color);
                screen.set_pixel(x + pos.x, y + pos.y, col.to_pixel());
            }
        }
    }

    pub fn screen(&self) -> &Bitmap {
        &self.screen
    }
}
