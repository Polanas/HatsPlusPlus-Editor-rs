use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use crate::file_utils::Ms;
use eframe::glow::{HasContext, NativeTexture};

use crate::{file_utils::file_modified_time, texture::Texture};

#[derive(Debug)]
struct TextureData {
    inner: Rc<RefCell<crate::texture::Inner>>,
    path: PathBuf,
    modified_time: Ms,
}

impl TextureData {
    fn new(inner: Rc<RefCell<crate::texture::Inner>>, path: PathBuf, modified_time: Ms) -> Self {
        Self {
            inner: inner.clone(),
            path,
            modified_time,
        }
    }
}
#[derive(Debug)]
pub struct TextureReloader {
    textures: Vec<TextureData>,
}

impl TextureReloader {
    pub fn new() -> Self {
        Self { textures: vec![] }
    }
    pub fn update_texture_program(&mut self, old: NativeTexture, new: NativeTexture) {
        for texture in &mut self.textures {
            if texture.inner.borrow_mut().native == old {
                texture.inner.borrow_mut().native = new;
                break;
            }
        }
    }
    pub fn add_texture(&mut self, texture: &Texture) {
        let Some(path) = texture.path() else {
            return;
        };
        let Some(modified_time) = file_modified_time(path) else {
            return;
        };
        self.textures.push(TextureData::new(
            texture.inner_rc().clone(),
            path.clone(),
            modified_time,
        ));
    }
    pub fn update(&mut self, gl: &eframe::glow::Context) {
        self.textures
            .retain(|t| unsafe { gl.is_texture(t.inner.borrow().native) });
        self.try_reload(gl);
    }
    fn try_reload(&mut self, gl: &eframe::glow::Context) {
        for texture in &mut self.textures {
            let Some(new_modified_time) = file_modified_time(&texture.path) else {
                return;
            };
            let old_modified_time = texture.modified_time;
            texture.modified_time = new_modified_time;
            if new_modified_time == old_modified_time {
                continue;
            }
            let Ok(new_texture) = Texture::from_path(gl, &texture.path) else {
                continue;
            };
            unsafe { gl.delete_texture(NativeTexture(texture.inner.borrow().native.0)) };
            let mut inner = texture.inner.borrow_mut();
            inner.native = new_texture.native();
            inner.width = new_texture.width();
            inner.height = new_texture.height();
        }
    }
}
