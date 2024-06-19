use anyhow::Result;
use bevy_math::IVec2;
use num_traits::FromPrimitive;
use pixas::bitmap::{Bitmap, BitmapLoadError};
use std::path::Path;

use crate::frames_from_range::frames_from_range;
use crate::prelude::*;

const SEPARATOR: &str = "_";

#[derive(Debug)]
pub struct HatNameAndSize {
    pub name: String,
    pub size: Option<IVec2>,
}

impl HatNameAndSize {
    fn new(name: String, size: Option<IVec2>) -> Self {
        Self { name, size }
    }
}

pub fn get_animation(metapixels: &[Metapixel], index: usize) -> Option<Animation> {
    let (Some(anim_type), Some(delay), Some(looping), Some(frame_or_range)) = (
        metapixels.get(index),
        metapixels.get(index + 1),
        metapixels.get(index + 2),
        metapixels.get(index + 3),
    ) else {
        return None;
    };

    let (MetapixelType::AnimationType, MetapixelType::AnimationDelay, MetapixelType::AnimationLoop) =
        (anim_type.get_type(), delay.get_type(), looping.get_type())
    else {
        return None;
    };

    match frame_or_range.get_type() {
        MetapixelType::AnimationFrame | MetapixelType::AnimationFramePeriod => {}
        _ => return None,
    };
    let anim_type = AnimationType::from_u8(anim_type.g)?;
    let looping = looping.g.max(1) == 0;

    if let MetapixelType::AnimationFramePeriod = frame_or_range.get_type() {
        let frames = frames_from_range(frame_or_range.g as i32, frame_or_range.b as i32);
        return Some(Animation::new(anim_type, delay.g as i32, looping, frames));
    }

    let frames = &metapixels[(index + 3)..]
        .iter()
        .take_while(|m| matches!(m.get_type(), MetapixelType::AnimationFrame))
        .map(|m| m.g as i32)
        .collect::<Vec<_>>();

    Some(Animation::new(
        anim_type,
        delay.g as i32,
        looping,
        frames.clone(),
    ))
}
pub fn get_name_and_size(name: &str) -> HatNameAndSize {
    if !name.contains(SEPARATOR) {
        return HatNameAndSize::new(name.to_string(), None);
    }
    let parts = name
        .split(SEPARATOR)
        .take(3)
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    let [name, size_x, size_y] = &parts[..] else {
        return HatNameAndSize::new(name.to_string(), None);
    };
    let (Ok(size_x), Ok(size_y)) = (size_x.parse(), size_y.parse()) else {
        return HatNameAndSize::new(name.to_string(), None);
    };

    if size_x <= 0 || size_y <= 0 {
        return HatNameAndSize::new(name.to_string(), None);
    }

    HatNameAndSize {
        name: name.clone(),
        size: Some(IVec2::new(size_x, size_y)),
    }
}
pub fn get_metapixels_and_size(
    path: &Path,
    name_and_size: &HatNameAndSize,
) -> Result<(Vec<Metapixel>, IVec2), BitmapLoadError> {
    let bitmap = Bitmap::from_path(path)?;
    let size = name_and_size
        .size
        .unwrap_or(IVec2::new(bitmap.width as i32, bitmap.height as i32));
    let metapixels = get_metapixels(&bitmap, size);
    Ok((metapixels, size))
}
fn get_metapixels(bitmap: &Bitmap, size: IVec2) -> Vec<Metapixel> {
    let metapixels_size = IVec2::new(bitmap.width as i32 - size.x, bitmap.height as i32);
    let mut metapixels: Vec<Metapixel> = vec![];

    for x in 0..=metapixels_size.x {
        for y in 0..=metapixels_size.y {
            let Some(pixel) = bitmap.get_pixel(size.x + x, y) else {
                continue;
            };

            if pixel.is_empty() {
                continue;
            }
            let Some(metapixel) = Metapixel::new(pixel.r, pixel.g, pixel.b) else {
                continue;
            };

            metapixels.push(metapixel);
        }
    }

    metapixels
}
