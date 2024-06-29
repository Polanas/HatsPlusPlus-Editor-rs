use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use anyhow::{bail, Result};
use bevy_math::IVec2;
use eframe::glow::{self, Context, HasContext, NativeTexture};
use pixas::bitmap::Bitmap;

#[derive(Debug, Clone, Copy)]
pub struct Inner {
    pub native: NativeTexture,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone)]
pub struct Texture {
    inner: Rc<RefCell<Inner>>,
    path: Option<PathBuf>,
}

impl Texture {
    pub fn delete(&self, gl: &eframe::glow::Context) {
        unsafe { gl.delete_texture(NativeTexture(self.inner.borrow().native.0)) };
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
                inner: Rc::new(RefCell::new(Inner {
                    width: size.x,
                    height: size.y,
                    native: texture,
                })),
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
                inner: Rc::new(RefCell::new(Inner {
                    width: bitmap.width as i32,
                    height: bitmap.height as i32,
                    native: texture,
                })),
                path: Some(path.as_ref().to_owned()),
            })
        }
    }

    pub fn replace_from_path(&mut self, gl: &Context, path: impl AsRef<Path>) -> Option<()> {
        self.delete(gl);
        {
            let new_texture = Texture::from_path(gl, path).ok()?;
            let binding = self.inner_rc().clone();
            let current_texture = &mut *binding.borrow_mut();
            current_texture.native = new_texture.native();
            current_texture.width = new_texture.width();
            current_texture.height = new_texture.height();
            self.path = new_texture.path.clone();
        }
        Some(())
    }

    pub fn width(&self) -> i32 {
        self.inner.borrow().width
    }

    pub fn height(&self) -> i32 {
        self.inner.borrow().height
    }

    pub fn inner_rc(&self) -> Rc<RefCell<Inner>> {
        self.inner.clone()
    }

    pub fn inner(&self) -> Inner {
        *self.inner.borrow()
    }

    pub fn native(&self) -> NativeTexture {
        NativeTexture(self.inner.borrow().native.0)
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }
}
