use std::rc::Rc;

use bevy_math::{IVec2, Vec2};
use derivative::Derivative;
use eframe::egui::Color32;
use pixas::bitmap::Bitmap;

#[derive(Clone, Copy, Default, Debug)]
pub struct Depth(pub f32);

#[derive(Clone, Debug, Derivative)]
#[derivative(Default)]
pub struct Sprite {
    pub size: Vec2,
    pub depth: Depth,
    pub bitmap: Option<Rc<Bitmap>>,
    #[derivative(Default(value = "Color32::from_rgb(255,255,255)"))]
    pub color: Color32,
    pub position: Vec2,
    pub angle: f32,
    pub flipped_horizontaly: bool,
    pub flipped_vertically: bool,
}

impl Sprite {
    pub fn new(bitmap: Rc<Bitmap>) -> Self {
        let size = Vec2::new(bitmap.width as f32, bitmap.height as f32);
        Self {
            bitmap: Some(bitmap),
            size,
            ..Default::default()
        }
    }
}
