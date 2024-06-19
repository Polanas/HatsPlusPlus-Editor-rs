use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Result};
use bevy_math::IVec2;
use eframe::glow::{self, Context, HasContext, NativeTexture};
use pixas::bitmap::Bitmap;

#[derive(Debug, Clone)]
pub struct Texture {
    native: Arc<NativeTexture>,
    width: i32,
    height: i32,
    path: Option<PathBuf>,
}

impl Texture {
    pub fn delete(&self, gl: &eframe::glow::Context) {
        unsafe { gl.delete_texture(NativeTexture(self.native.0)) };
    }
    #[allow(dead_code)]
    pub fn with_size(gl: &Context, size: IVec2) -> Result<Self> {
        if size.x == 0 || size.y == 0 {
            bail!("attempt to create empty texture with size {0}", size);
        }
        unsafe {
            let texture = match gl.create_texture() {
                Ok(texture) => texture,
                Err(err) => bail!("could not create texture: {}", err),
            };
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                size.x,
                size.y,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
            Ok(Self {
                width: size.x,
                height: size.y,
                native: Arc::new(texture),
                path: None,
            })
        }
    }

    pub fn from_path(gl: &Context, path: impl AsRef<Path>) -> Result<Self> {
        let bitmap = Bitmap::from_path(path.as_ref())?;
        if bitmap.width == 0 || bitmap.height == 0 {
            bail!(
                "tried to create empty texture with size {0}",
                IVec2::new(bitmap.width as i32, bitmap.height as i32)
            );
        }
        let data = bitmap.get_pixel_data();
        unsafe {
            let texture = match gl.create_texture() {
                Ok(texture) => texture,
                Err(err) => bail!("coud not create texture: {}", err),
            };
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                bitmap.width as i32,
                bitmap.height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(data),
            );
            gl.texture_parameter_i32(texture, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            gl.texture_parameter_i32(texture, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            gl.texture_parameter_i32(texture, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
            gl.texture_parameter_i32(texture, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
            Ok(Self {
                width: bitmap.width as i32,
                height: bitmap.height as i32,
                native: Arc::new(texture),
                path: Some(path.as_ref().to_owned()),
            })
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn native_arc(&self) -> Arc<NativeTexture> {
        self.native.clone()
    }

    pub fn native(&self) -> NativeTexture {
        NativeTexture(self.native.0)
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }
}
