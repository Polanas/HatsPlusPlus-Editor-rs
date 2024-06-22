use std::{cell::Cell, fmt::Display};

use once_cell::sync::Lazy;

use crate::{
    hats::HatType,
    is_range::is_range,
    metapixels::{self, Metapixels},
    prelude::{Metapixel, MetapixelType},
};
use AnimationType as AT;
//TODO: add on death/ressurect animations?
pub static WINGS_ANIMATIONS: Lazy<Vec<AnimationType>> = Lazy::new(|| {
    vec![
        AT::Flying,
        AT::StartIdle,
        AT::Gliding,
        AT::StartGliding,
        AT::Idle,
    ]
});
pub static PET_AIMATIONS: Lazy<Vec<AnimationType>> = Lazy::new(|| {
    vec![
        AT::OnApproach,
        AT::OnDuckDeath,
        AT::OnStatic,
        AT::OnDefault,
        AT::OnRessurect,
    ]
});
pub static WEREABLE_ANIMATIONS: Lazy<Vec<AnimationType>> = Lazy::new(|| {
    vec![
        AT::OnDefault,
        AT::OnPressQuack,
        AT::OnReleaseQuack,
        AT::OnDuckDeath,
        AT::OnRessurect,
    ]
});

pub fn avalible_animations<'a>(hat_type: HatType) -> Option<&'a [AnimationType]> {
    match hat_type {
        HatType::Wereable => Some(&WEREABLE_ANIMATIONS),
        HatType::Wings => Some(&WINGS_ANIMATIONS),
        HatType::FlyingPet | HatType::WalkingPet => Some(&PET_AIMATIONS),
        _ => None,
    }
}

#[derive(Copy, Clone, Debug, FromPrimitive, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, Hash)]
pub struct FrameId(pub u32);

#[derive(Debug, Hash)]
pub struct Frame {
    pub value: i32,
    id: FrameId,
}

impl Clone for Frame {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            id: frame_id(),
        }
    }
}

impl Frame {
    pub fn new(value: i32) -> Self {
        Self {
            value,
            id: frame_id(),
        }
    }
    pub fn id(&self)  -> FrameId {
        self.id
    }
}

impl From<Frame> for u32 {
    fn from(frame: Frame) -> Self {
        frame.value as u32
    }
}
impl From<Frame> for i32 {
    fn from(frame: Frame) -> Self {
        frame.value
    }
}
impl From<u32> for Frame {
    fn from(value: u32) -> Self {
        Self {
            value: value as i32,
            id: frame_id(),
        }
    }
}
impl From<i32> for Frame {
    fn from(value: i32) -> Self {
        Self {
            value,
            id: frame_id(),
        }
    }
}

thread_local! {
    static FRAME_ID_COUNTER: Cell<u32> = Cell::new(0);
}

pub fn frame_id() -> FrameId {
    let id = FRAME_ID_COUNTER.get();
    FRAME_ID_COUNTER.set(id + 1);
    FrameId(id)
}

#[derive(Clone, Debug)]
pub struct Animation {
    pub anim_type: AnimationType,
    pub delay: i32,
    pub looping: bool,
    pub frames: Vec<Frame>,
    pub new_frame: i32,
    pub new_range_start: i32,
    pub new_range_end: i32,
}

impl Animation {
    pub fn new(anim_type: AnimationType, delay: i32, looping: bool, frames: Vec<Frame>) -> Self {
        Self {
            anim_type,
            delay,
            looping,
            frames,
            new_frame: 0,
            new_range_end: 1,
            new_range_start: 1,
        }
    }

    pub fn gen_metapixels(&self) -> Vec<Metapixel> {
        let mut metapixels = Metapixels::new();
        metapixels.push_many(&metapixels::metapixels!(
            (MetapixelType::AnimationType, self.anim_type as u8, 0),
            (MetapixelType::AnimationDelay, self.delay as u8, 0),
            (MetapixelType::AnimationLoop, self.looping as u8, 0),
        ));

        if is_range(&self.frames.iter().map(|f| f.value).collect::<Vec<_>>()[..])
            && self.frames.len() > 1
        {
            metapixels.push(
                MetapixelType::AnimationFramePeriod,
                self.frames[0].value as u8,
                self.frames[self.frames.len() - 1].value as u8,
            );
            return metapixels.pixels;
        }

        for frame in &self.frames {
            metapixels.push(MetapixelType::AnimationFrame, frame.value as u8, 0);
        }

        metapixels.pixels
    }
}
