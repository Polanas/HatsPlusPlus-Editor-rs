use core::panic;
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use crate::animation::{Animation, AnimationType};
use crate::file_utils::FileStemString;
use crate::hat_utils::*;
use crate::metapixels::Metapixels;
use crate::prelude::*;
use crate::tabs::SelectedHat;
use crate::texture::Texture;
use crate::texture_reloader::TextureReloader;
use crate::ui_text::UiText;
use anyhow::{bail, Result};
use bevy_math::IVec2;
use derivative::Derivative;
use downcast_rs::{impl_downcast, Downcast};
use eframe::glow::Context;
use num_traits::FromPrimitive;
use pixas::bitmap::Bitmap;
use pixas::pixel::Pixel;
use pixas::Rectanlge;

macro_rules! impl_abstract_hat {
    ($t:ty, $base_name:ident, $($anims_name:ident).+) => {
        impl AbstractHat for $t {
            fn base_mut(&mut self) -> &mut HatBase {
                &mut self.$base_name
            }
            fn base(&self) -> &HatBase {
                &self.$base_name
            }
            fn texture(&self) -> Option<&Texture> {
                self.$base_name.texture.as_ref()
            }
            fn animations(&self) -> Option<&[Animation]> {
                Some(&self.$($anims_name).+[..])
            }
            fn frames_amount(&self) -> u32 {
                let frames_x = (self.texture().map(|t| t.width()).unwrap_or(0)) / self.base().frame_size.x;
                let frames_y = (self.texture().map(|t| t.height()).unwrap_or(0)) / self.base().frame_size.y;
                (frames_x * frames_y) as u32
            }
            fn animations_mut(&mut self) -> Option<&mut [Animation]> {
                Some(&mut self.$($anims_name).+[..])
            }
        }
    };
    ($t:ty, $base_name:ident) => {
        impl AbstractHat for $t {
            fn base_mut(&mut self) -> &mut HatBase {
                &mut self.$base_name
            }
            fn base(&self) -> &HatBase {
                &self.$base_name
            }
            fn texture(&self) -> Option<&Texture> {
                self.$base_name.texture.as_ref()
            }
            fn animations(&self) -> Option<&[Animation]> {
                None
            }
            fn frames_amount(&self) -> u32 {
                let frames_x = (self.texture().map(|t| t.width()).unwrap_or(0)) / self.base().frame_size.x;
                let frames_y = (self.texture().map(|t| t.height()).unwrap_or(0)) / self.base().frame_size.y;
                (frames_x * frames_y) as u32
            }
            fn animations_mut(&mut self) -> Option<&mut [Animation]> {
                None
            }
        }
    };
}

impl_abstract_hat!(WereableHat, base, animations);
impl_abstract_hat!(WingsHat, base);
impl_abstract_hat!(RoomHat, base);
impl_abstract_hat!(ExtraHat, base);
impl_abstract_hat!(PreviewHat, base);
impl_abstract_hat!(WalkingPet, hat_base, pet_base.animations);
impl_abstract_hat!(FlyingPet, hat_base, pet_base.animations);

pub trait HatName: GetHatBase {
    fn hat_name(&self) -> Option<String> {
        let path = &self.get_base().path;
        path.file_stem_string()
    }
}
impl HatName for Box<dyn AbstractHat> {}

pub trait AbstractHat: Downcast + std::fmt::Debug {
    fn base(&self) -> &HatBase;
    #[allow(dead_code)]
    fn base_mut(&mut self) -> &mut HatBase;
    fn texture(&self) -> Option<&Texture>;
    fn animations(&self) -> Option<&[Animation]>;
    fn animations_mut(&mut self) -> Option<&mut [Animation]>;
    fn frames_amount(&self) -> u32;
}

pub trait GetHatBase {
    fn get_base(&self) -> &HatBase;
}

impl GetHatBase for Box<dyn AbstractHat> {
    fn get_base(&self) -> &HatBase {
        self.base()
    }
}

impl_downcast!(AbstractHat);
trait GenMetapixels {
    fn gen_metapixels(&self) -> Vec<Metapixel>;
}

macro_rules! gen_metapixels_branch {
    ($e: expr, $t:ty) => {
        if let Some(hat) = $e.downcast_ref::<$t>() {
            hat.gen_metapixels()
        } else {
            panic!("the type of the hat does not match its actual type (wierd, I know)")
        }
    };
}
impl GenMetapixels for Box<dyn AbstractHat> {
    fn gen_metapixels(&self) -> Vec<Metapixel> {
        match self.base().hat_type {
            HatType::Wereable => gen_metapixels_branch!(self, WereableHat),
            HatType::Wings => gen_metapixels_branch!(self, WingsHat),
            HatType::Extra => gen_metapixels_branch!(self, ExtraHat),
            HatType::WalkingPet => gen_metapixels_branch!(self, WalkingPet),
            HatType::FlyingPet => gen_metapixels_branch!(self, FlyingPet),
            HatType::Room => gen_metapixels_branch!(self, RoomHat),
            HatType::Preview => gen_metapixels_branch!(self, PreviewHat),
            HatType::Unspecified => panic!("Invalid hat type: {}", HatType::Unspecified),
        }
    }
}

impl SaveHat for Box<dyn AbstractHat> {}
trait SaveHat: GenMetapixels + GetHatBase {
    fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let area_size = self.get_base().hat_area_size;
        let file_name = format!(
            "{0}_{1}_{2}.png",
            self.get_base().hat_type.get_save_name(),
            area_size.x,
            area_size.y
        );
        let hat_bitmap = match &self.get_base().bitmap {
            Some(bitmap) => bitmap,
            None => {
                bail!("unable to save hat {0}: no bitmap found", file_name);
            }
        };
        let metapixels = self.gen_metapixels();
        let rows_amount = (metapixels.len() as f32 / hat_bitmap.height as f32).ceil() as i32;
        let final_image_size = IVec2::new(rows_amount + area_size.x, hat_bitmap.height as i32);
        let mut final_image =
            Bitmap::with_size(final_image_size.x as u32, final_image_size.y as u32);
        final_image.draw_from(hat_bitmap, 0, 0);
        let rect = Rectanlge::new(
            area_size.x,
            0,
            final_image.width - area_size.x as u32,
            hat_bitmap.height,
        );
        final_image.for_each_part_mut(rect, |pixel, _| {
            *pixel = Pixel::empty();
            ControlFlow::Continue(())
        });
        insert_metapixels(&mut final_image, &metapixels, area_size);
        final_image.save(path.as_ref().join(file_name)).ok();
        Ok(())
    }
}

pub trait LoadHat: Sized {
    fn load_from_path(path: impl AsRef<Path>, gl: &Context) -> Result<Self> {
        let name_and_size =
            get_name_and_size(&path.as_ref().file_stem_string().unwrap_or_default());
        LoadHat::load_from_name_and_size(path, name_and_size, gl)
    }
    fn load_from_name_and_size(
        path: impl AsRef<Path>,
        name_and_size: HatNameAndSize,
        gl: &Context,
    ) -> Result<Self>;
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum HatType {
    Wereable,
    Wings,
    Extra,
    FlyingPet,
    WalkingPet,
    Room,
    Preview,
    #[default]
    Unspecified,
}

impl TryFrom<SelectedHat> for HatType {
    type Error = ();
    fn try_from(value: SelectedHat) -> Result<Self, ()> {
        match value {
            SelectedHat::Wereable => Ok(Self::Wereable),
            SelectedHat::Extra => Ok(Self::Extra),
            SelectedHat::Wings => Ok(Self::Wings),
            SelectedHat::Room => Ok(Self::Room),
            SelectedHat::Preview => Ok(Self::Preview),
            SelectedHat::Pet(_) => Err(()),
        }
    }
}

impl Display for HatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = (match self {
            HatType::Wereable => "Wereable Hat",
            HatType::Wings => "Wings Hat",
            HatType::Extra => "Extra",
            HatType::FlyingPet => "Flying Pet",
            HatType::WalkingPet => "Walking Pet",
            HatType::Room => "Room",
            HatType::Preview => "Preview",
            HatType::Unspecified => panic!("reached unspecified"),
        })
        .to_owned();
        write!(f, "{name}")
    }
}
impl HatType {
    pub fn get_display_name(&self, ui_text: &UiText) -> String {
        match self {
            Self::Wereable => ui_text.get("Wereable"),
            Self::Wings => ui_text.get("Wings"),
            Self::Extra => ui_text.get("Extra"),
            Self::FlyingPet => ui_text.get("Flying pet"),
            Self::WalkingPet => ui_text.get("Walking pet"),
            Self::Room => ui_text.get("Room"),
            Self::Preview => ui_text.get("Preview"),
            Self::Unspecified => unreachable!(),
        }
    }
    fn get_save_name(&self) -> &str {
        match self {
            Self::Wereable => "hat",
            Self::Wings => "wings",
            Self::Extra => "extraHat",
            Self::FlyingPet => "flyingPet",
            Self::WalkingPet => "walkingPet",
            Self::Room => "room",
            Self::Preview => "preview",
            Self::Unspecified => "default",
        }
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum LinkFrameState {
    #[default]
    Default,
    Saved,
    Inverted,
}
impl Display for LinkFrameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display = match self {
            Self::Default => "None",
            Self::Saved => "Saved",
            Self::Inverted => "Inverted",
        };
        f.write_str(display)
    }
}

#[derive(Debug, Default)]
pub struct HatBase {
    pub hat_type: HatType,
    pub frame_size: IVec2,
    pub hat_area_size: IVec2,
    pub bitmap: Option<Bitmap>,
    pub texture: Option<Texture>,
    pub path: PathBuf,
}

#[derive(Debug, Default)]
pub struct FlyingPet {
    pub pet_base: PetBase,
    pub hat_base: HatBase,
    changes_angle: bool,
    speed: Option<i32>,
}
impl LoadHat for FlyingPet {
    fn load_from_name_and_size(
        path: impl AsRef<Path>,
        name_and_size: HatNameAndSize,
        gl: &Context,
    ) -> Result<Self> {
        let (metapixels, size) = get_metapixels_and_size(path.as_ref(), &name_and_size)?;
        let texture = Texture::from_path(gl, &path)?;
        let mut hat = FlyingPet {
            hat_base: HatBase {
                hat_area_size: size,
                bitmap: Bitmap::from_path(path.as_ref()).ok(),
                hat_type: HatType::FlyingPet,
                path: path.as_ref().to_owned(),
                texture: Some(texture),
                ..Default::default()
            },
            ..Default::default()
        };
        for (i, pixel) in metapixels.iter().enumerate() {
            match pixel.get_type() {
                MetapixelType::PetDistance => hat.pet_base.distance = Some(pixel.g as i32),
                MetapixelType::PetNoFlip => hat.pet_base.flipped = false,
                MetapixelType::FrameSize => {
                    hat.hat_base.frame_size = IVec2::new(pixel.g as i32, pixel.b as i32)
                }
                MetapixelType::AnimationType => {
                    if let Some(anim) = get_animation(&metapixels, i) {
                        hat.pet_base.animations.push(anim)
                    }
                }
                MetapixelType::IsBigHat => hat.pet_base.is_big = true,
                MetapixelType::PetChangesAngle => hat.changes_angle = true,
                MetapixelType::PetSpeed => hat.speed = Some(pixel.g as i32),
                _ => (),
            };
        }
        Ok(hat)
    }
}

#[derive(Debug, Default)]
pub struct WalkingPet {
    pub pet_base: PetBase,
    pub hat_base: HatBase,
}
impl LoadHat for WalkingPet {
    fn load_from_name_and_size(
        path: impl AsRef<Path>,
        name_and_size: HatNameAndSize,
        gl: &Context,
    ) -> Result<Self> {
        let (metapixels, size) = get_metapixels_and_size(path.as_ref(), &name_and_size)?;
        let texture = Texture::from_path(gl, &path)?;
        let mut hat = WalkingPet {
            hat_base: HatBase {
                hat_area_size: size,
                bitmap: Bitmap::from_path(path.as_ref()).ok(),
                hat_type: HatType::WalkingPet,
                path: path.as_ref().to_owned(),
                texture: Some(texture),
                ..Default::default()
            },
            ..Default::default()
        };
        for (i, pixel) in metapixels.iter().enumerate() {
            match pixel.get_type() {
                MetapixelType::PetDistance => hat.pet_base.distance = Some(pixel.g as i32),
                MetapixelType::PetNoFlip => hat.pet_base.flipped = false,
                MetapixelType::FrameSize => {
                    hat.hat_base.frame_size = IVec2::new(pixel.g as i32, pixel.b as i32)
                }
                MetapixelType::AnimationType => {
                    if let Some(anim) = get_animation(&metapixels, i) {
                        hat.pet_base.animations.push(anim)
                    }
                }
                MetapixelType::IsBigHat => hat.pet_base.is_big = true,
                _ => (),
            };
        }
        Ok(hat)
    }
}

#[derive(Debug, Derivative)]
#[derivative(Default)]
pub struct PetBase {
    pub distance: Option<i32>,
    #[derivative(Default(value = "true"))]
    pub flipped: bool,
    pub is_big: bool,
    pub link_frame_state: LinkFrameState,
    pub animations: Vec<Animation>,
}

impl GenMetapixels for FlyingPet {
    fn gen_metapixels(&self) -> Vec<Metapixel> {
        let mut metapixels = Metapixels::new();
        if let Some(dist) = self.pet_base.distance {
            metapixels.push(MetapixelType::PetDistance, dist as u8, 0);
        }
        if self.pet_base.flipped {
            metapixels.push(MetapixelType::PetNoFlip, 0, 0);
        }
        if self.base().frame_size.x > 32 || self.base().frame_size.y > 32 {
            metapixels.push(MetapixelType::IsBigHat, 0, 0);
        }
        metapixels.push(
            MetapixelType::FrameSize,
            self.base().frame_size.x as u8,
            self.base().frame_size.y as u8,
        );
        if !matches!(self.pet_base.link_frame_state, LinkFrameState::Default) {
            metapixels.push(
                MetapixelType::LinkFrameState,
                self.pet_base.link_frame_state as u8,
                0,
            );
        }
        if self.changes_angle {
            metapixels.push(MetapixelType::PetChangesAngle, 0, 0);
        }
        if let Some(speed) = self.speed {
            metapixels.push(MetapixelType::PetSpeed, speed as u8, 0);
        }

        for anim in &self.pet_base.animations {
            for pixel in anim.gen_metapixels() {
                metapixels.push_raw(pixel);
            }
        }

        metapixels.pixels
    }
}
impl GenMetapixels for WalkingPet {
    fn gen_metapixels(&self) -> Vec<Metapixel> {
        let mut metapixels = Metapixels::new();
        if let Some(dist) = self.pet_base.distance {
            metapixels.push(MetapixelType::PetDistance, dist as u8, 0);
        }
        if self.pet_base.flipped {
            metapixels.push(MetapixelType::PetNoFlip, 0, 0);
        }
        if self.base().frame_size.x > 32 || self.base().frame_size.y > 32 {
            metapixels.push(MetapixelType::IsBigHat, 0, 0);
        }
        metapixels.push(
            MetapixelType::FrameSize,
            self.base().frame_size.x as u8,
            self.base().frame_size.y as u8,
        );
        if !matches!(self.pet_base.link_frame_state, LinkFrameState::Default) {
            metapixels.push(
                MetapixelType::LinkFrameState,
                self.pet_base.link_frame_state as u8,
                0,
            );
        }

        for anim in &self.pet_base.animations {
            for pixel in anim.gen_metapixels() {
                metapixels.push_raw(pixel);
            }
        }

        metapixels.pixels
    }
}

#[derive(Debug, Default)]
pub struct PreviewHat {
    pub base: HatBase,
}

impl GenMetapixels for PreviewHat {
    fn gen_metapixels(&self) -> Vec<Metapixel> {
        vec![]
    }
}

impl LoadHat for PreviewHat {
    fn load_from_path(path: impl AsRef<Path>, gl: &Context) -> Result<Self> {
        let bitmap = Bitmap::from_path(path.as_ref())?;
        let texture = Texture::from_path(gl, &path)?;
        Ok(PreviewHat {
            base: HatBase {
                hat_type: HatType::Preview,
                frame_size: (bitmap.width as i32, bitmap.height as i32).into(),
                hat_area_size: (bitmap.width as i32, bitmap.height as i32).into(),
                bitmap: Some(bitmap),
                path: path.as_ref().to_owned(),
                texture: Some(texture),
            },
        })
    }

    fn load_from_name_and_size(
        path: impl AsRef<Path>,
        _: HatNameAndSize,
        gl: &Context,
    ) -> Result<Self> {
        LoadHat::load_from_path(path.as_ref(), gl)
    }
}

#[derive(Debug, Derivative)]
#[derivative(Default)]
pub struct WingsHat {
    #[derivative(Default(value = "IVec2::new(128,128)"))]
    pub general_offset: IVec2,
    #[derivative(Default(value = "IVec2::new(128,128)"))]
    pub crouch_offset: IVec2,
    #[derivative(Default(value = "IVec2::new(128,128)"))]
    pub ragdoll_offset: IVec2,
    #[derivative(Default(value = "IVec2::new(128,128)"))]
    pub slide_offset: IVec2,
    #[derivative(Default(value = "IVec2::new(128,128)"))]
    pub net_offset: IVec2,
    pub gen_animations: bool,
    pub auto_glide_frame: Option<i32>,
    pub auto_idle_frame: Option<i32>,
    pub auto_anim_speed: Option<i32>,
    pub changes_animations: bool,
    pub size_state: bool,
    pub base: HatBase,
}

impl LoadHat for WingsHat {
    fn load_from_name_and_size(
        path: impl AsRef<Path>,
        name_and_size: HatNameAndSize,
        gl: &Context,
    ) -> Result<Self> {
        let (metapixels, size) = get_metapixels_and_size(path.as_ref(), &name_and_size)?;
        let texture = Texture::from_path(gl, &path)?;
        let mut hat = WingsHat {
            base: HatBase {
                hat_area_size: size,
                bitmap: Bitmap::from_path(path.as_ref()).ok(),
                hat_type: HatType::Wings,
                path: path.as_ref().to_owned(),
                texture: Some(texture),
                ..Default::default()
            },
            ..Default::default()
        };
        for pixel in metapixels {
            match pixel.get_type() {
                MetapixelType::WingsNetOffset => {
                    hat.net_offset = IVec2::new(pixel.g as i32 - 128, pixel.b as i32 - 128)
                }
                MetapixelType::WingsGeneralOffset => {
                    hat.general_offset = IVec2::new(pixel.g as i32 - 128, pixel.b as i32 - 128)
                }
                MetapixelType::WingsSlideOffset => {
                    hat.slide_offset = IVec2::new(pixel.g as i32 - 128, pixel.b as i32 - 128)
                }
                MetapixelType::WingsRagdollOffset => {
                    hat.ragdoll_offset = IVec2::new(pixel.g as i32 - 128, pixel.b as i32 - 128)
                }
                MetapixelType::WingsCrouchOffset => {
                    hat.crouch_offset = IVec2::new(pixel.g as i32 - 128, pixel.b as i32 - 128)
                }
                MetapixelType::GenerateWingsAnimations => hat.gen_animations = true,
                MetapixelType::WingsAutoGlideFrame => hat.auto_glide_frame = Some(pixel.g as i32),
                MetapixelType::WingsAutoIdleFrame => hat.auto_idle_frame = Some(pixel.g as i32),
                MetapixelType::WingsAutoAnimationsSpeed => {
                    hat.auto_anim_speed = Some(pixel.g as i32)
                }
                MetapixelType::FrameSize => {
                    hat.base.frame_size = IVec2::new(pixel.g as i32, pixel.b as i32)
                }
                MetapixelType::ChangeAnimationsEveryLevel => hat.changes_animations = true,
                MetapixelType::IsBigHat => hat.size_state = true,
                _ => (),
            }
        }

        Ok(hat)
    }
}

//TODO: add animations for wings
impl GenMetapixels for WingsHat {
    fn gen_metapixels(&self) -> Vec<Metapixel> {
        let mut metapixels = Metapixels::new();
        if self.general_offset.x != 128 || self.general_offset.y != 128 {
            metapixels.push(
                MetapixelType::WingsGeneralOffset,
                self.general_offset.x as u8,
                self.general_offset.y as u8,
            );
        }
        if self.slide_offset.x != 128 || self.slide_offset.y != 128 {
            metapixels.push(
                MetapixelType::WingsSlideOffset,
                self.slide_offset.x as u8,
                self.slide_offset.y as u8,
            );
        }
        if self.ragdoll_offset.x != 128 || self.ragdoll_offset.y != 128 {
            metapixels.push(
                MetapixelType::WingsRagdollOffset,
                self.ragdoll_offset.x as u8,
                self.ragdoll_offset.y as u8,
            );
        }
        if self.crouch_offset.x != 128 || self.net_offset.y != 128 {
            metapixels.push(
                MetapixelType::WingsCrouchOffset,
                self.crouch_offset.x as u8,
                self.crouch_offset.y as u8,
            );
        }
        if self.net_offset.x != 128 || self.net_offset.y != 128 {
            metapixels.push(
                MetapixelType::WingsNetOffset,
                self.net_offset.x as u8,
                self.net_offset.y as u8,
            );
        }
        if self.base.frame_size.x > 32 || self.base.frame_size.y > 32 {
            metapixels.push(MetapixelType::IsBigHat, 0, 0);
        }
        if self.gen_animations {
            metapixels.push(MetapixelType::GenerateWingsAnimations, 0, 0);
        }
        if self.changes_animations {
            metapixels.push(MetapixelType::ChangeAnimationsEveryLevel, 0, 0);
        }
        if let Some(speed) = self.auto_anim_speed {
            metapixels.push(MetapixelType::WingsAutoAnimationsSpeed, speed as u8, 0);
        }
        if let Some(frame) = self.auto_glide_frame {
            metapixels.push(MetapixelType::WingsAutoGlideFrame, frame as u8, 0);
        }
        if let Some(frame) = self.auto_idle_frame {
            metapixels.push(MetapixelType::WingsAutoIdleFrame, frame as u8, 0);
        }
        metapixels.push(
            MetapixelType::FrameSize,
            self.base.frame_size.x as u8,
            self.base.frame_size.y as u8,
        );
        metapixels.pixels
    }
}

#[derive(Debug, Default)]
pub struct WereableHat {
    pub strapped_on: bool,
    pub is_big: bool,
    pub animations: Vec<Animation>,
    pub link_frame_state: LinkFrameState,
    pub on_spawn_animation: Option<AnimationType>,
    pub base: HatBase,
}

impl LoadHat for WereableHat {
    fn load_from_name_and_size(
        path: impl AsRef<Path>,
        name_and_size: HatNameAndSize,
        gl: &Context,
    ) -> Result<Self> {
        let texture = Texture::from_path(gl, &path)?;
        let (metapixels, size) = get_metapixels_and_size(path.as_ref(), &name_and_size)?;
        let mut hat: WereableHat = WereableHat {
            base: HatBase {
                hat_area_size: size,
                bitmap: Bitmap::from_path(path.as_ref()).ok(),
                hat_type: HatType::Wereable,
                path: path.as_ref().to_owned(),
                texture: Some(texture),
                ..Default::default()
            },
            ..Default::default()
        };
        for (i, pixel) in metapixels.iter().enumerate() {
            match pixel.get_type() {
                MetapixelType::OnSpawnAnimation => {
                    hat.on_spawn_animation = AnimationType::from_u8(pixel.g);
                }
                MetapixelType::StrappedOn => hat.strapped_on = true,
                MetapixelType::IsBigHat => hat.is_big = true,
                MetapixelType::FrameSize => {
                    hat.base.frame_size = IVec2::new(pixel.g as i32, pixel.b as i32)
                }
                MetapixelType::LinkFrameState => {
                    hat.link_frame_state = match pixel.g {
                        1 => LinkFrameState::Saved,
                        2 => LinkFrameState::Inverted,
                        _ => LinkFrameState::Default,
                    }
                }
                MetapixelType::AnimationType => {
                    if let Some(anim) = get_animation(&metapixels, i) {
                        hat.animations.push(anim)
                    }
                }
                _ => {}
            }
        }
        Ok(hat)
    }
}

impl GenMetapixels for WereableHat {
    fn gen_metapixels(&self) -> Vec<Metapixel> {
        let mut metapixels = Metapixels::new();
        if self.strapped_on {
            metapixels.push(MetapixelType::StrappedOn, 0, 0);
        }
        if self.base.frame_size.x > 32 || self.base.frame_size.y > 32 {
            metapixels.push(MetapixelType::IsBigHat, 0, 0);
        }
        metapixels.push(
            MetapixelType::FrameSize,
            self.base.frame_size.x as u8,
            self.base.frame_size.y as u8,
        );
        if let Some(spawn_anim) = self.on_spawn_animation {
            metapixels.push(MetapixelType::OnSpawnAnimation, spawn_anim as u8, 0);
        }
        if !matches!(self.link_frame_state, LinkFrameState::Default) {
            metapixels.push(
                MetapixelType::LinkFrameState,
                self.link_frame_state as u8,
                0,
            );
        }
        for anim in &self.animations {
            for pixel in anim.gen_metapixels() {
                metapixels.push_raw(pixel);
            }
        }
        metapixels.pixels
    }
}

#[derive(Debug, Default)]
pub struct RoomHat {
    pub base: HatBase,
}
impl LoadHat for RoomHat {
    fn load_from_name_and_size(
        path: impl AsRef<Path>,
        name_and_size: HatNameAndSize,
        gl: &Context,
    ) -> Result<Self> {
        let (_, size) = get_metapixels_and_size(path.as_ref(), &name_and_size)?;
        let texture = Texture::from_path(gl, &path)?;
        let hat = RoomHat {
            base: HatBase {
                hat_area_size: size,
                bitmap: Bitmap::from_path(path.as_ref()).ok(),
                hat_type: HatType::Room,
                path: path.as_ref().to_owned(),
                texture: Some(texture),
                ..Default::default()
            },
        };
        Ok(hat)
    }
}

impl GenMetapixels for RoomHat {
    fn gen_metapixels(&self) -> Vec<Metapixel> {
        vec![]
    }
}

#[derive(Debug, Default)]
pub struct ExtraHat {
    pub base: HatBase,
}

impl LoadHat for ExtraHat {
    fn load_from_name_and_size(
        path: impl AsRef<Path>,
        name_and_size: HatNameAndSize,
        gl: &Context,
    ) -> Result<Self> {
        let texture = Texture::from_path(gl, &path)?;
        let (metapixels, size) = get_metapixels_and_size(path.as_ref(), &name_and_size)?;
        let mut hat = ExtraHat {
            base: HatBase {
                hat_area_size: size,
                bitmap: Bitmap::from_path(path.as_ref()).ok(),
                hat_type: HatType::Extra,
                path: path.as_ref().to_owned(),
                texture: Some(texture),
                ..Default::default()
            },
        };

        for pixel in metapixels {
            if let MetapixelType::FrameSize = pixel.get_type() {
                hat.base.frame_size = IVec2::new(pixel.g as i32, pixel.b as i32)
            }
        }
        //there wasn't a frame size metapixel
        if hat.base.frame_size == IVec2::ZERO {
            hat.base.frame_size = size;
        }

        Ok(hat)
    }
}

impl GenMetapixels for ExtraHat {
    fn gen_metapixels(&self) -> Vec<Metapixel> {
        let mut metapixels = Metapixels::new();
        metapixels.push(
            MetapixelType::FrameSize,
            self.base.frame_size.x as u8,
            self.base.frame_size.y as u8,
        );
        metapixels.pixels
    }
}

const PREVIEW_NAME: &str = "preview";
const EXTRA_NAME: &str = "extrahat";
const WEREABLE_NAME: &str = "hat";
#[allow(dead_code)]
const ROOM_NAME: &str = "room";
const WALKING_PET_NAME: &str = "walkingpet";
const FLYING_PET_NAME: &str = "flyingpet";
const WINGS_NAME: &str = "wings";

#[derive(Debug, Default)]
pub struct Hat {
    pub unique_elemets: HashMap<HatType, Box<dyn AbstractHat>>,
    pub pets: Vec<Box<dyn AbstractHat>>,
    pub path: Option<PathBuf>,
}

impl Hat {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            pets: vec![],
            unique_elemets: HashMap::new(),
        }
    }
    pub fn name(&self) -> Option<String> {
        let path = self.path.as_ref()?;
        path.file_stem_string()
    }
    pub fn delete_textures(&self, gl: &eframe::glow::Context) {
        for element in self.iter_all_elements() {
            if let Some(texture) = element.texture() {
                texture.delete(gl);
            }
        }
    }
    pub fn has_elements(&self) -> bool {
        !self.unique_elemets.is_empty() || !self.pets.is_empty()
    }
    //what a return type... I'm so glad it was autogenerated
    #[allow(clippy::complexity)]
    #[allow(dead_code)]
    pub fn iter_all_elements_mut(
        &mut self,
    ) -> std::iter::Chain<
        std::collections::hash_map::ValuesMut<'_, HatType, std::boxed::Box<(dyn AbstractHat)>>,
        std::slice::IterMut<'_, std::boxed::Box<(dyn AbstractHat)>>,
    > {
        self.unique_elemets.values_mut().chain(self.pets.iter_mut())
    }
    #[allow(clippy::complexity)]
    pub fn iter_all_elements(
        &'_ self,
    ) -> std::iter::Chain<
        std::collections::hash_map::Values<'_, HatType, std::boxed::Box<(dyn AbstractHat)>>,
        std::slice::Iter<'_, std::boxed::Box<(dyn AbstractHat)>>,
    > {
        self.unique_elemets.values().chain(self.pets.iter())
    }
    pub fn add_textures_to_reloader(&self, reloader: &mut TextureReloader) {
        for element in self.iter_all_elements() {
            if let Some(texture) = element.texture() {
                reloader.add_texture(texture);
            }
        }
    }
    pub fn wereable(&self) -> Option<&WereableHat> {
        self.unique_elemets
            .get(&HatType::Wereable)
            .and_then(|e| e.downcast_ref::<WereableHat>())
    }
    pub fn wereable_mut(&mut self) -> Option<&mut WereableHat> {
        self.unique_elemets
            .get_mut(&HatType::Wereable)
            .and_then(|e| e.downcast_mut::<WereableHat>())
    }
    pub fn room(&self) -> Option<&RoomHat> {
        self.unique_elemets
            .get(&HatType::Room)
            .and_then(|e| e.downcast_ref::<RoomHat>())
    }
    pub fn room_mut(&mut self) -> Option<&mut RoomHat> {
        self.unique_elemets
            .get_mut(&HatType::Room)
            .and_then(|e| e.downcast_mut::<RoomHat>())
    }
    pub fn preview(&self) -> Option<&PreviewHat> {
        self.unique_elemets
            .get(&HatType::Preview)
            .and_then(|e| e.downcast_ref::<PreviewHat>())
    }
    pub fn walking_pet_mut(&mut self) -> Option<&mut WalkingPet> {
        self.unique_elemets
            .get_mut(&HatType::WalkingPet)
            .and_then(|e| e.downcast_mut::<WalkingPet>())
    }
    pub fn walking_pet(&self) -> Option<&WalkingPet> {
        self.unique_elemets
            .get(&HatType::WalkingPet)
            .and_then(|e| e.downcast_ref::<WalkingPet>())
    }
    pub fn flying_pet_mut(&mut self) -> Option<&mut FlyingPet> {
        self.unique_elemets
            .get_mut(&HatType::FlyingPet)
            .and_then(|e| e.downcast_mut::<FlyingPet>())
    }
    pub fn flying_pet(&self) -> Option<&FlyingPet> {
        self.unique_elemets
            .get(&HatType::FlyingPet)
            .and_then(|e| e.downcast_ref::<FlyingPet>())
    }
    pub fn extra_mut(&mut self) -> Option<&mut ExtraHat> {
        self.unique_elemets
            .get_mut(&HatType::Extra)
            .and_then(|e| e.downcast_mut::<ExtraHat>())
    }
    pub fn extra(&self) -> Option<&ExtraHat> {
        self.unique_elemets
            .get(&HatType::Extra)
            .and_then(|e| e.downcast_ref::<ExtraHat>())
    }
    pub fn wings_mut(&mut self) -> Option<&mut WereableHat> {
        self.unique_elemets
            .get_mut(&HatType::Wings)
            .and_then(|e| e.downcast_mut::<WereableHat>())
    }
    pub fn wings(&self) -> Option<&WereableHat> {
        self.unique_elemets
            .get(&HatType::Wings)
            .and_then(|e| e.downcast_ref::<WereableHat>())
    }
    pub fn remove_pet(&mut self, index: usize) {
        self.pets.remove(index);
    }
    pub fn add_pet(&mut self, hat: Box<dyn AbstractHat>) {
        self.pets.push(hat);
    }
    pub fn add_unique_hat(&mut self, hat_type: HatType, hat: Box<dyn AbstractHat>) {
        let is_specified = !matches!(hat_type, HatType::Unspecified);
        assert!(is_specified);
        self.unique_elemets.insert(hat_type, hat);
    }
    pub fn remove_unique_hat(&mut self, hat_type: HatType) {
        let is_specified = !matches!(hat_type, HatType::Unspecified);
        assert!(is_specified);
        self.unique_elemets.remove(&hat_type);
    }
    pub fn first_element(&self) -> Option<(&dyn AbstractHat, HatType)> {
        let first_unique = self
            .unique_elemets
            .values()
            .next()
            .map(|e| (&**e, e.base().hat_type));
        if first_unique.is_some() {
            return first_unique;
        }
        self.pets.first().map(|e| (&**e, e.base().hat_type))
    }
    pub fn save(&self, dir_path: impl AsRef<Path>) -> Result<()> {
        let path = dir_path.as_ref();
        for element in self.unique_elemets.values() {
            if let Some(preview) = element.downcast_ref::<PreviewHat>() {
                if let Some(bitmap) = &preview.base().bitmap {
                    bitmap.save(path.join("preview.png"))?;
                }
            }
            element.save(path)?;
        }
        for pet in &self.pets {
            pet.save(path)?;
        }
        Ok(())
    }

    pub fn load(dir_path: impl AsRef<Path>, gl: &eframe::glow::Context) -> Result<Hat> {
        let path = dir_path.as_ref();
        if !path.exists() {
            bail!("path to hat was not found: {:?}", path);
        }

        let mut hat = Hat::new(path.to_path_buf());

        for entry in std::fs::read_dir(path)?.flatten() {
            let Some(file_name) = entry
                .path()
                .file_stem()
                .and_then(|p| p.to_str())
                .map(|s| s.to_owned())
            else {
                continue;
            };

            let name_and_size = get_name_and_size(&file_name);

            match name_and_size.name.to_lowercase().as_str() {
                WINGS_NAME => {
                    if let Ok(wings) =
                        WingsHat::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_unique_hat(HatType::Wings, Box::new(wings));
                    }
                }
                EXTRA_NAME => {
                    if let Ok(extra) =
                        ExtraHat::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_unique_hat(HatType::Extra, Box::new(extra));
                    }
                }
                PREVIEW_NAME => {
                    if let Ok(preview) =
                        PreviewHat::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_unique_hat(HatType::Preview, Box::new(preview));
                    }
                }
                WEREABLE_NAME => {
                    if let Ok(wereable) =
                        WereableHat::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_unique_hat(HatType::Wereable, Box::new(wereable));
                    }
                }
                FLYING_PET_NAME => {
                    if let Ok(pet) =
                        FlyingPet::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_pet(Box::new(pet));
                    }
                }
                WALKING_PET_NAME => {
                    if let Ok(pet) =
                        WalkingPet::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_pet(Box::new(pet));
                    }
                }
                ROOM_NAME => {
                    if let Ok(room) =
                        RoomHat::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_unique_hat(HatType::Room, Box::new(room));
                    }
                }
                _ => {}
            };
        }
        Ok(hat)
    }
}

fn insert_metapixels(bitmap: &mut Bitmap, pixels: &[Metapixel], hat_area_size: IVec2) {
    let mut pixel_iter = pixels.iter().peekable();
    let (mut x, mut y) = (hat_area_size.x, 0);
    while x != bitmap.width as i32 {
        while y != bitmap.height as i32 {
            let Some(pixel) = pixel_iter.next() else {
                break;
            };
            let pixel = Pixel::from_rgb(pixel.r, pixel.g, pixel.b);
            bitmap.set_pixel(x, y, pixel);
            y += 1;

            if let Some(next) = pixel_iter.peek() {
                if matches!(
                    MetapixelType::from_u8(next.r).unwrap(),
                    MetapixelType::AnimationType
                ) {
                    y += 1;
                }
            }
        }

        x += 1;
    }
}
