use core::panic;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::animations::{AnimType, Animation};
use crate::file_utils::FileStemString;
use crate::frames_from_range::frames_from_range;
use crate::hat_utils::*;
use crate::metapixels::Metapixels;
use crate::prelude::*;
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
            fn texture_mut(&mut self) -> Option<&mut Texture> {
                self.$base_name.texture.as_mut()
            }
            fn texture(&self) -> Option<&Texture> {
                self.$base_name.texture.as_ref()
            }
            fn animations(&self) -> Option<& Vec<AnimationCell>> {
                Some(&self.$($anims_name).+)
            }
            fn frames_amount(&self) -> u32 {
                let frames_x = (self.texture().map(|t| t.width()).unwrap_or(0)) / self.base().frame_size.x;
                let frames_y = (self.texture().map(|t| t.height()).unwrap_or(0)) / self.base().frame_size.y;
                (frames_x * frames_y) as u32
            }
            fn animations_mut(&mut self) -> Option<&mut Vec<AnimationCell>> {
                Some(&mut self.$($anims_name).+)
            }
            fn id(&self) -> HatElementId {
                self.base().id
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
            fn texture_mut(&mut self) -> Option<&mut Texture> {
                self.$base_name.texture.as_mut()
            }
            fn animations(&self) -> Option<&Vec<AnimationCell>> {
                None
            }
            fn frames_amount(&self) -> u32 {
                let frames_x = (self.texture().map(|t| t.width()).unwrap_or(0)) / self.base().frame_size.x;
                let frames_y = (self.texture().map(|t| t.height()).unwrap_or(0)) / self.base().frame_size.y;
                (frames_x * frames_y) as u32
            }
            fn animations_mut(&mut self) -> Option<&mut Vec<AnimationCell>> {
                None
            }
            fn id(&self) -> HatElementId {
                self.base().id
            }
        }
    };
}

impl_abstract_hat!(Wereable, base, animations);
impl_abstract_hat!(Wings, base, animations);
impl_abstract_hat!(Extra, base, animations);
impl_abstract_hat!(RoomHat, base);
impl_abstract_hat!(Preview, base);
impl_abstract_hat!(WalkingPet, hat_base, pet_base.animations);
impl_abstract_hat!(FlyingPet, hat_base, pet_base.animations);

const PREVIEW_NAME: &str = "preview";
const EXTRA_NAME: &str = "extrahat";
const WEREABLE_NAME: &str = "hat";
#[allow(dead_code)]
const ROOM_NAME: &str = "room";
const WALKING_PET_NAME: &str = "walkingpet";
const FLYING_PET_NAME: &str = "flyingpet";
const WINGS_NAME: &str = "wings";
///TODO: change these to more reasonable values
pub const DEFAULT_PET_SPEED: i32 = 10;
pub const DEFAULT_PET_DISTANCE: i32 = 10;
pub const MAX_PETS: usize = 5;
pub const DEFAULT_WINGS_IDLE_FRAME: i32 = 0;
pub const DEFAULT_AUTO_SPEED: i32 = 4;
pub const MAX_EXTRA_HAT_SIZE: IVec2 = IVec2::new(97, 56);
pub const MIN_FRAME_SIZE: i32 = 32;
pub const MAX_FRAME_SIZE: i32 = 64;

thread_local! {
    static HAT_ID_COUNTER: Cell<u32> = const { Cell::new(0) };
}

pub fn hat_id() -> HatElementId {
    let id = HAT_ID_COUNTER.get();
    HAT_ID_COUNTER.set(id + 1);
    HatElementId(id)
}

type AnimationCell = Rc<RefCell<Animation>>;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Default)]
pub struct HatElementId(u32);

pub trait AbstractHat: Downcast + std::fmt::Debug {
    fn base(&self) -> &HatBase;
    #[allow(dead_code)]
    fn base_mut(&mut self) -> &mut HatBase;
    fn texture(&self) -> Option<&Texture>;
    fn texture_mut(&mut self) -> Option<&mut Texture>;
    fn animations(&self) -> Option<&Vec<AnimationCell>>;
    fn animations_mut(&mut self) -> Option<&mut Vec<AnimationCell>>;
    fn frames_amount(&self) -> u32;
    fn id(&self) -> HatElementId;
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
            HatType::Wereable => gen_metapixels_branch!(self, Wereable),
            HatType::Wings => gen_metapixels_branch!(self, Wings),
            HatType::Extra => gen_metapixels_branch!(self, Extra),
            HatType::WalkingPet => gen_metapixels_branch!(self, WalkingPet),
            HatType::FlyingPet => gen_metapixels_branch!(self, FlyingPet),
            HatType::Room => gen_metapixels_branch!(self, RoomHat),
            HatType::Preview => gen_metapixels_branch!(self, Preview),
            HatType::Unspecified => panic!("Invalid hat type: {}", HatType::Unspecified),
        }
    }
}

impl SaveHat for Box<dyn AbstractHat> {}
trait SaveHat: GenMetapixels + GetHatBase {
    fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let base = &self.get_base();
        let area_size = base.hat_area_size;
        let metapixels = self.gen_metapixels();
        let save_name = base
            .name
            .as_ref()
            .cloned()
            .unwrap_or(base.hat_type.save_name().to_string());
        let file_name = if !metapixels.is_empty() {
            format!("{0}_{1}_{2}.png", save_name, area_size.x, area_size.y)
        } else {
            format!("{0}.png", save_name)
        };
        let hat_bitmap = match &base.bitmap {
            Some(bitmap) => bitmap,
            None => {
                bail!("unable to save hat {0}: no bitmap found", file_name);
            }
        };
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

pub trait LoadHat: Sized + AbstractHat {
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

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, FromPrimitive)]
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

impl Display for HatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = (match self {
            HatType::Wereable => "Wearable Hat",
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
            Self::Wereable => ui_text.get("Wearable"),
            Self::Wings => ui_text.get("Wings"),
            Self::Extra => ui_text.get("Extra"),
            Self::FlyingPet => ui_text.get("Flying pet"),
            Self::WalkingPet => ui_text.get("Walking pet"),
            Self::Room => ui_text.get("Room"),
            Self::Preview => ui_text.get("Preview"),
            Self::Unspecified => "".to_string(),
        }
    }
    pub fn save_name(&self) -> &str {
        match self {
            Self::Wereable => "hat",
            Self::Wings => "wings",
            Self::Extra => "extrahat",
            Self::FlyingPet => "flyingpet",
            Self::WalkingPet => "walkingpet",
            Self::Room => "room",
            Self::Preview => "preview",
            Self::Unspecified => "",
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
    pub name: Option<String>,
    pub id: HatElementId,
}

#[derive(Debug, Derivative)]
#[derivative(Default)]
pub struct FlyingPet {
    pub pet_base: PetBase,
    pub hat_base: HatBase,
    pub changes_angle: bool,
    #[derivative(Default(value = "DEFAULT_PET_SPEED"))]
    pub speed: i32,
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
                id: hat_id(),
                name: if name_and_size.is_name_valid() {
                    Some(name_and_size.name)
                } else {
                    None
                },
                hat_area_size: size,
                bitmap: Bitmap::from_path(path.as_ref()).ok(),
                hat_type: HatType::FlyingPet,
                frame_size: (MIN_FRAME_SIZE, MIN_FRAME_SIZE).into(),
                texture: Some(texture),
            },
            ..Default::default()
        };
        for (i, pixel) in metapixels.iter().enumerate() {
            match pixel.get_type() {
                MetapixelType::PetDistance => hat.pet_base.distance = pixel.g as i32,
                MetapixelType::PetNoFlip => hat.pet_base.flipped = false,
                MetapixelType::FrameSize => {
                    hat.hat_base.frame_size = IVec2::new(pixel.g as i32, pixel.b as i32)
                }
                MetapixelType::AnimationType => {
                    if let Some(anim) = get_animation(&metapixels, i) {
                        hat.pet_base.animations.push(RefCell::new(anim).into())
                    }
                }
                MetapixelType::IsBigHat => hat.pet_base.is_big = true,
                MetapixelType::PetChangesAngle => hat.changes_angle = true,
                MetapixelType::PetSpeed => hat.speed = pixel.g as i32,
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
                name: if name_and_size.is_name_valid() {
                    Some(name_and_size.name)
                } else {
                    None
                },
                id: hat_id(),
                frame_size: (MIN_FRAME_SIZE, MIN_FRAME_SIZE).into(),
                hat_area_size: size,
                bitmap: Bitmap::from_path(path.as_ref()).ok(),
                hat_type: HatType::WalkingPet,
                texture: Some(texture),
            },
            ..Default::default()
        };
        for (i, pixel) in metapixels.iter().enumerate() {
            match pixel.get_type() {
                MetapixelType::PetDistance => hat.pet_base.distance = pixel.g as i32,
                MetapixelType::PetNoFlip => hat.pet_base.flipped = false,
                MetapixelType::FrameSize => {
                    hat.hat_base.frame_size = IVec2::new(pixel.g as i32, pixel.b as i32)
                }
                MetapixelType::AnimationType => {
                    if let Some(anim) = get_animation(&metapixels, i) {
                        hat.pet_base.animations.push(RefCell::new(anim).into())
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
    #[derivative(Default(value = "DEFAULT_PET_DISTANCE"))]
    pub distance: i32,
    #[derivative(Default(value = "true"))]
    pub flipped: bool,
    pub is_big: bool,
    pub link_frame_state: LinkFrameState,
    pub animations: Vec<AnimationCell>,
}

impl GenMetapixels for FlyingPet {
    fn gen_metapixels(&self) -> Vec<Metapixel> {
        let mut metapixels = Metapixels::new();
        if self.pet_base.distance != DEFAULT_PET_DISTANCE {
            metapixels.push(MetapixelType::PetDistance, self.pet_base.distance as u8, 0);
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
        if self.speed != DEFAULT_PET_SPEED {
            metapixels.push(MetapixelType::PetSpeed, self.speed as u8, 0);
        }

        for anim in &self.pet_base.animations {
            for pixel in anim.borrow().gen_metapixels() {
                metapixels.push_raw(pixel);
            }
        }

        metapixels.pixels
    }
}
impl GenMetapixels for WalkingPet {
    fn gen_metapixels(&self) -> Vec<Metapixel> {
        let mut metapixels = Metapixels::new();
        if self.pet_base.distance != DEFAULT_PET_DISTANCE {
            metapixels.push(MetapixelType::PetDistance, self.pet_base.distance as u8, 0);
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
            for pixel in anim.borrow().gen_metapixels() {
                metapixels.push_raw(pixel);
            }
        }

        metapixels.pixels
    }
}

#[derive(Debug, Default)]
pub struct Preview {
    pub base: HatBase,
}

impl GenMetapixels for Preview {
    fn gen_metapixels(&self) -> Vec<Metapixel> {
        vec![]
    }
}

impl LoadHat for Preview {
    fn load_from_path(path: impl AsRef<Path>, gl: &Context) -> Result<Self> {
        let bitmap = Bitmap::from_path(path.as_ref())?;
        let texture = Texture::from_path(gl, &path)?;
        let name_and_size = get_name_and_size(&path.file_stem_string().unwrap_or_default());
        Ok(Preview {
            base: HatBase {
                name: if name_and_size.is_name_valid() {
                    Some(name_and_size.name)
                } else {
                    None
                },
                id: hat_id(),
                hat_type: HatType::Preview,
                frame_size: (MIN_FRAME_SIZE, MIN_FRAME_SIZE).into(),
                hat_area_size: (bitmap.width as i32, bitmap.height as i32).into(),
                bitmap: Some(bitmap),
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
pub struct Wings {
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
    pub auto_glide_frame: i32,
    pub auto_idle_frame: i32,
    pub auto_anim_speed: i32,
    pub changes_animations: bool,
    pub size_state: bool,
    pub base: HatBase,
    pub animations: Vec<AnimationCell>,
}

impl LoadHat for Wings {
    fn load_from_name_and_size(
        path: impl AsRef<Path>,
        name_and_size: HatNameAndSize,
        gl: &Context,
    ) -> Result<Self> {
        let (metapixels, size) = get_metapixels_and_size(path.as_ref(), &name_and_size)?;
        let texture = Texture::from_path(gl, &path)?;
        let mut hat = Wings {
            base: HatBase {
                name: if name_and_size.is_name_valid() {
                    Some(name_and_size.name)
                } else {
                    None
                },
                id: hat_id(),
                hat_area_size: size,
                bitmap: Bitmap::from_path(path.as_ref()).ok(),
                hat_type: HatType::Wings,
                frame_size: (MIN_FRAME_SIZE, MIN_FRAME_SIZE).into(),
                texture: Some(texture),
            },
            ..Default::default()
        };
        let mut has_auto_speed = false;

        hat.auto_glide_frame = hat.frames_amount() as i32;
        hat.auto_idle_frame = DEFAULT_WINGS_IDLE_FRAME;
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
                MetapixelType::WingsAutoGlideFrame => {
                    hat.auto_glide_frame = pixel.g.saturating_add(1) as i32
                }
                MetapixelType::WingsAutoIdleFrame => {
                    hat.auto_idle_frame = pixel.g.saturating_add(1) as i32
                }
                MetapixelType::WingsAutoAnimationsSpeed => {
                    hat.auto_anim_speed = pixel.g as i32;
                    has_auto_speed = true;
                }
                MetapixelType::FrameSize => {
                    hat.base.frame_size = IVec2::new(pixel.g as i32, pixel.b as i32)
                }
                MetapixelType::ChangeAnimationsEveryLevel => hat.changes_animations = true,
                MetapixelType::IsBigHat => hat.size_state = true,
                _ => (),
            }
        }
        hat.auto_anim_speed = if has_auto_speed {
            hat.auto_anim_speed
        } else {
            DEFAULT_AUTO_SPEED
        };
        hat.animations.push(RefCell::new(Animation::new(
            AnimType::OnDefault,
            hat.auto_anim_speed,
            false,
            frames_from_range(0, hat.frames_amount() as i32 - 1),
        )).into());

        Ok(hat)
    }
}

//TODO: add animations for wings
impl GenMetapixels for Wings {
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
        if self.auto_anim_speed != DEFAULT_AUTO_SPEED {
            metapixels.push(
                MetapixelType::WingsAutoAnimationsSpeed,
                self.auto_anim_speed as u8,
                0,
            );
        }
        //don't substract one since auto glide frame starts from one
        if self.auto_glide_frame != self.frames_amount() as i32 {
            metapixels.push(
                MetapixelType::WingsAutoGlideFrame,
                (self.auto_glide_frame as u8).saturating_sub(1),
                0,
            );
        }
        if self.auto_idle_frame != DEFAULT_WINGS_IDLE_FRAME {
            metapixels.push(
                MetapixelType::WingsAutoIdleFrame,
                (self.auto_idle_frame as u8).saturating_sub(1),
                0,
            );
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
pub struct Wereable {
    pub strapped_on: bool,
    pub is_big: bool,
    pub animations: Vec<AnimationCell>,
    pub link_frame_state: LinkFrameState,
    pub on_spawn_animation: Option<AnimType>,
    pub base: HatBase,
}

impl LoadHat for Wereable {
    fn load_from_name_and_size(
        path: impl AsRef<Path>,
        name_and_size: HatNameAndSize,
        gl: &Context,
    ) -> Result<Self> {
        let texture = Texture::from_path(gl, &path)?;
        let (metapixels, size) = get_metapixels_and_size(path.as_ref(), &name_and_size)?;
        let mut hat: Wereable = Wereable {
            base: HatBase {
                id: hat_id(),
                hat_area_size: size,
                bitmap: Bitmap::from_path(path.as_ref()).ok(),
                hat_type: HatType::Wereable,
                name: if name_and_size.is_name_valid() {
                    Some(name_and_size.name)
                } else {
                    None
                },
                frame_size: (MIN_FRAME_SIZE, MIN_FRAME_SIZE).into(),
                texture: Some(texture),
            },
            ..Default::default()
        };
        for (i, pixel) in metapixels.iter().enumerate() {
            match pixel.get_type() {
                MetapixelType::OnSpawnAnimation => {
                    hat.on_spawn_animation = AnimType::from_u8(pixel.g);
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
                        hat.animations.push(RefCell::new(anim).into())
                    }
                }
                _ => {}
            }
        }
        Ok(hat)
    }
}

impl GenMetapixels for Wereable {
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
            for pixel in anim.borrow().gen_metapixels() {
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
                id: hat_id(),
                hat_area_size: size,
                bitmap: Bitmap::from_path(path.as_ref()).ok(),
                hat_type: HatType::Room,
                frame_size: size,
                name: if name_and_size.is_name_valid() {
                    Some(name_and_size.name)
                } else {
                    None
                },
                texture: Some(texture),
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
pub struct Extra {
    pub base: HatBase,
    pub animations: Vec<AnimationCell>,
}

impl LoadHat for Extra {
    fn load_from_name_and_size(
        path: impl AsRef<Path>,
        name_and_size: HatNameAndSize,
        gl: &Context,
    ) -> Result<Self> {
        let texture = Texture::from_path(gl, &path)?;
        let (metapixels, size) = get_metapixels_and_size(path.as_ref(), &name_and_size)?;
        let mut hat = Extra {
            base: HatBase {
                name: if name_and_size.is_name_valid() {
                    Some(name_and_size.name)
                } else {
                    None
                },
                id: hat_id(),
                hat_area_size: size,
                bitmap: Bitmap::from_path(path.as_ref()).ok(),
                hat_type: HatType::Extra,
                frame_size: (
                    i32::min(texture.width(), MAX_EXTRA_HAT_SIZE.x),
                    i32::min(texture.height(), MAX_EXTRA_HAT_SIZE.y),
                )
                    .into(),
                texture: Some(texture),
            },
            ..Default::default()
        };

        hat.animations.push(RefCell::new(Animation::new(
            AnimType::OnDefault,
            4,
            false,
            frames_from_range(0, hat.frames_amount() as i32 - 1),
        )).into());

        for pixel in metapixels {
            if let MetapixelType::FrameSize = pixel.get_type() {
                hat.base.frame_size = IVec2::new(pixel.g as i32, pixel.b as i32)
            }
        }

        Ok(hat)
    }
}

impl GenMetapixels for Extra {
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
    pub fn hat_type_by_id(&self, id: HatElementId) -> Option<HatType> {
        self.iter_all_elements()
            .filter(|h| h.id() == id)
            .map(|h| h.base().hat_type)
            .next()
    }
    pub fn id_from_hat_type(&self, hat_type: HatType) -> Option<HatElementId> {
        self.iter_all_elements()
            .filter(|h| h.base().hat_type == hat_type)
            .map(|h| h.id())
            .next()
    }
    pub fn element_from_id_mut(&mut self, id: HatElementId) -> Option<&mut dyn AbstractHat> {
        self.iter_all_elements_mut().find(|h| h.id() == id)
    }
    #[allow(dead_code)]
    pub fn element_from_id(&self, id: HatElementId) -> Option<&dyn AbstractHat> {
        self.iter_all_elements().find(|h| h.id() == id)
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

    pub fn iter_all_elements_mut(&mut self) -> impl Iterator<Item = &mut dyn AbstractHat> {
        self.unique_elemets
            .values_mut()
            .chain(self.pets.iter_mut())
            .map(|h| &mut **h)
    }
    pub fn iter_all_elements(&self) -> impl Iterator<Item = &dyn AbstractHat> {
        self.unique_elemets
            .values()
            .chain(self.pets.iter())
            .map(|h| &**h)
    }
    pub fn add_textures_to_reloader(&self, reloader: &mut TextureReloader) {
        for element in self.iter_all_elements() {
            if let Some(texture) = element.texture() {
                reloader.add_texture(texture);
            }
        }
    }
    pub fn can_add_pets(&self) -> bool {
        self.pets.len() < MAX_PETS
    }
    pub fn wereable(&self) -> Option<&Wereable> {
        self.unique_elemets
            .get(&HatType::Wereable)
            .and_then(|e| e.downcast_ref::<Wereable>())
    }
    pub fn wereable_mut(&mut self) -> Option<&mut Wereable> {
        self.unique_elemets
            .get_mut(&HatType::Wereable)
            .and_then(|e| e.downcast_mut::<Wereable>())
    }
    pub fn room(&self) -> Option<&RoomHat> {
        self.unique_elemets
            .get(&HatType::Room)
            .and_then(|e| e.downcast_ref::<RoomHat>())
    }
    #[allow(dead_code)]
    pub fn room_mut(&mut self) -> Option<&mut RoomHat> {
        self.unique_elemets
            .get_mut(&HatType::Room)
            .and_then(|e| e.downcast_mut::<RoomHat>())
    }
    pub fn preview(&self) -> Option<&Preview> {
        self.unique_elemets
            .get(&HatType::Preview)
            .and_then(|e| e.downcast_ref::<Preview>())
    }
    #[allow(dead_code)]
    pub fn walking_pet_mut(&mut self) -> Option<&mut WalkingPet> {
        self.unique_elemets
            .get_mut(&HatType::WalkingPet)
            .and_then(|e| e.downcast_mut::<WalkingPet>())
    }
    #[allow(dead_code)]
    pub fn walking_pet(&self) -> Option<&WalkingPet> {
        self.unique_elemets
            .get(&HatType::WalkingPet)
            .and_then(|e| e.downcast_ref::<WalkingPet>())
    }
    #[allow(dead_code)]
    pub fn flying_pet_mut(&mut self) -> Option<&mut FlyingPet> {
        self.unique_elemets
            .get_mut(&HatType::FlyingPet)
            .and_then(|e| e.downcast_mut::<FlyingPet>())
    }
    #[allow(dead_code)]
    pub fn flying_pet(&self) -> Option<&FlyingPet> {
        self.unique_elemets
            .get(&HatType::FlyingPet)
            .and_then(|e| e.downcast_ref::<FlyingPet>())
    }
    #[allow(dead_code)]
    pub fn extra_mut(&mut self) -> Option<&mut Extra> {
        self.unique_elemets
            .get_mut(&HatType::Extra)
            .and_then(|e| e.downcast_mut::<Extra>())
    }
    pub fn extra(&self) -> Option<&Extra> {
        self.unique_elemets
            .get(&HatType::Extra)
            .and_then(|e| e.downcast_ref::<Extra>())
    }
    pub fn wings_mut(&mut self) -> Option<&mut Wings> {
        self.unique_elemets
            .get_mut(&HatType::Wings)
            .and_then(|e| e.downcast_mut::<Wings>())
    }
    pub fn wings(&self) -> Option<&Wings> {
        self.unique_elemets
            .get(&HatType::Wings)
            .and_then(|e| e.downcast_ref::<Wings>())
    }
    pub fn replace_element(&mut self, id: HatElementId, element: impl AbstractHat) {
        self.remove_element(id);
        self.add_element(element);
    }
    pub fn remove_element(&mut self, id: HatElementId) {
        self.unique_elemets.retain(|_, e| e.id() != id);
        self.pets.retain(|e| e.id() != id);
    }
    pub fn add_element(&mut self, element: impl AbstractHat) {
        match element.base().hat_type {
            HatType::WalkingPet | HatType::FlyingPet => self.add_pet(Box::new(element)),
            _ => self.add_unique_hat(element.base().hat_type, Box::new(element)),
        };
    }
    pub fn add_pet(&mut self, hat: Box<dyn AbstractHat>) {
        self.pets.push(hat);
    }
    pub fn add_unique_hat(&mut self, hat_type: HatType, hat: Box<dyn AbstractHat>) {
        let is_specified = !matches!(hat_type, HatType::Unspecified);
        assert!(is_specified);
        self.unique_elemets.insert(hat_type, hat);
    }
    #[allow(dead_code)]
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
        std::fs::remove_dir_all(path)?;
        std::fs::create_dir(path)?;
        for element in self.unique_elemets.values() {
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
            let name_lowercase = name_and_size.name.to_lowercase();
            match name_lowercase.as_str() {
                ROOM_NAME => {
                    if let Ok(room) =
                        RoomHat::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_unique_hat(HatType::Room, Box::new(room));
                    }
                }
                EXTRA_NAME => {
                    if let Ok(extra) =
                        Extra::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_unique_hat(HatType::Extra, Box::new(extra));
                    }
                }
                WINGS_NAME => {
                    if let Ok(wings) =
                        Wings::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_unique_hat(HatType::Wings, Box::new(wings));
                    }
                }
                _ if name_lowercase.contains(FLYING_PET_NAME) => {
                    if let Ok(pet) =
                        FlyingPet::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_pet(Box::new(pet));
                    }
                }
                _ if name_lowercase.contains(WALKING_PET_NAME) => {
                    if let Ok(pet) =
                        WalkingPet::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_pet(Box::new(pet));
                    }
                }
                PREVIEW_NAME => {
                    if let Ok(preview) =
                        Preview::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_unique_hat(HatType::Preview, Box::new(preview));
                    }
                }
                WEREABLE_NAME => {
                    if let Ok(wereable) =
                        Wereable::load_from_name_and_size(entry.path(), name_and_size, gl)
                    {
                        hat.add_unique_hat(HatType::Wereable, Box::new(wereable));
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
