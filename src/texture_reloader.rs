use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use eframe::glow::{HasContext, NativeTexture};
use crate::file_utils::Ms;

use crate::{file_utils::file_modified_time, texture::Texture};

struct TextureData {
    handle: Arc<NativeTexture>,
    path: PathBuf,
    modified_time: Ms,
}

impl TextureData {
    fn new(handle: Arc<NativeTexture>, path: PathBuf, modified_time: Ms) -> Self {
        Self {
            handle,
            path,
            modified_time,
        }
    }
}


pub struct TextureReloader {
    textures: Vec<TextureData>,
}

impl TextureReloader {
    pub fn new() -> Self {
        Self { textures: vec![] }
    }

    pub fn add_texture(&mut self, texture: &Texture) {
        let Some(path) = texture.path() else {
            return;
        };
        let Some(modified_time) = file_modified_time(path) else {
            return;
        };
        self.textures.push(TextureData::new(
            texture.native_arc().clone(),
            path.clone(),
            modified_time,
        ));
    }
    pub fn try_reload(&mut self, gl: &eframe::glow::Context) {
        for texture in &mut self.textures {
            let Some(new_modified_time) = file_modified_time(&texture.path) else {
                return;
            };
            let old_modified_time = texture.modified_time;
            texture.modified_time = new_modified_time;
            if new_modified_time == old_modified_time {
                continue;
            }
            unsafe { gl.delete_texture(NativeTexture(texture.handle.0)) };
            let Ok(new_texture) = Texture::from_path(gl, &texture.path) else {
                continue;
            };
            let Some(handle) = Arc::get_mut(&mut texture.handle) else {
                continue;
            };
            *handle = new_texture.native();
        }
    }
}
