use std::fmt::Display;

use crate::{
    is_range::is_range,
    metapixels::{self, Metapixels},
    prelude::{Metapixel, MetapixelType},
};

#[derive(Copy, Clone, Debug, FromPrimitive)]
pub enum AnimationType {
    OnDefault,
    OnPressQuack,
    OnReleaseQuack,
    OnStatic,
    OnApproach,
    OnDuckDeath,
    Flying,
    StartIdle,
    Gliding,
    StartGliding,
    Idle,
    OnRessurect,
}

impl Display for AnimationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            AnimationType::OnDefault => "On Default",
            AnimationType::OnPressQuack => "On Press Quack",
            AnimationType::OnReleaseQuack => "On Release Quack",
            AnimationType::OnStatic => "On Static",
            AnimationType::OnApproach => "On Approach",
            AnimationType::OnDuckDeath => "On Duck Death",
            AnimationType::Flying => "Flying",
            AnimationType::StartIdle => "Start Idle",
            AnimationType::Gliding => "Gliding",
            AnimationType::StartGliding => "Start Gliding",
            AnimationType::Idle => "Idle",
            AnimationType::OnRessurect => "On Ressurect",
        };
        write!(f, "{}", name)
    }
}

#[derive(Clone, Debug)]
pub struct Animation {
    pub anim_type: AnimationType,
    pub delay: i32,
    pub looping: bool,
    pub frames: Vec<i32>,
    pub new_frame: i32,
    pub new_range_start: i32,
    pub new_range_end: i32,
}

impl Animation {
    pub fn new(anim_type: AnimationType, delay: i32, looping: bool, frames: Vec<i32>) -> Self {
        Self {
            anim_type,
            delay,
            looping,
            frames,
            new_frame: 0,
            new_range_end: 0,
            new_range_start: 0
        }
    }

    pub fn gen_metapixels(&self) -> Vec<Metapixel> {
        let mut metapixels = Metapixels::new();
        metapixels.push_many(&metapixels::metapixels!(
            (MetapixelType::AnimationType, self.anim_type as u8, 0),
            (MetapixelType::AnimationDelay, self.delay as u8, 0),
            (MetapixelType::AnimationLoop, self.looping as u8, 0),
        ));

        if is_range(&self.frames[..]) && self.frames.len() > 1 {
            metapixels.push(
                MetapixelType::AnimationFramePeriod,
                self.frames[0] as u8,
                self.frames[self.frames.len() - 1] as u8,
            );
            return metapixels.pixels;
        }

        for frame in &self.frames {
            metapixels.push(MetapixelType::AnimationFrame, *frame as u8, 0);
        }

        metapixels.pixels
    }
}
