use num_traits::FromPrimitive;

macro_rules! metapixels {
    ($(($pixel_type:expr, $g:expr, $b:expr )),+ $(,)? ) => {
        {
            let mut pixels = Vec::new();
            $(
                let p_type: crate::metapixels::MetapixelType = $pixel_type;
                let r = p_type as u8;
                if let Some(pixel) = crate::metapixels::Metapixel::new(r, $g, $b) {
                    pixels.push(pixel);
                }
            )*
            pixels
        }
    };
}

pub(crate) use metapixels;

#[derive(Debug, Clone, Copy, FromPrimitive)]
pub enum MetapixelType {
    StrappedOn,
    IsBigHat,
    FrameSize,
    AnimationType,
    AnimationDelay,
    AnimationLoop,
    AnimationFrame,
    AnimationFramePeriod,
    LinkFrameState,
    WingsGeneralOffset,
    WingsCrouchOffset,
    WingsRagdollOffset,
    WingsSlideOffset,
    GenerateWingsAnimations,
    PetChangesAngle,
    PetDistance,
    PetNoFlip,
    WingsAutoGlideFrame,
    WingsAutoIdleFrame,
    WingsAutoAnimationsSpeed,
    ChangeAnimationsEveryLevel,
    PetSpeed,
    WingsNetOffset,
    OnSpawnAnimation,
}

#[derive(Debug, Clone, Copy)]
pub struct Metapixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Metapixel {
    pub fn get_type(&self) -> MetapixelType {
        MetapixelType::from_u8(self.r).unwrap()
    }
}

impl Metapixel {
    pub fn new(r: u8, g: u8, b: u8) -> Option<Self> {
        if MetapixelType::from_u8(r).is_some() {
            return Some(Self { r, g, b });
        }
        None
    }
}

pub struct Metapixels {
    pub pixels: Vec<Metapixel>,
}

impl Metapixels {
    pub fn new() -> Self {
        Self { pixels: vec![] }
    }

    pub fn push(&mut self, pixel_type: MetapixelType, g: u8, b: u8) {
        let r = pixel_type as u8;
        if let Some(pixel) = Metapixel::new(r, g, b) {
            self.pixels.push(pixel);
        }
    }

    pub fn push_raw(&mut self, pixel: Metapixel) {
        self.pixels.push(pixel);
    }

    pub fn push_many(&mut self, pixels: &[Metapixel]) {
        self.pixels.extend_from_slice(pixels);
    }
}
