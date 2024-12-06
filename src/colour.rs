#[derive(Clone, Copy, Debug)]
pub struct Colour {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl Colour {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self { Self { r, g, b, a } }

    #[inline(always)]
    pub fn r(self) -> f32 { self.r }
    #[inline(always)]
    pub fn g(self) -> f32 { self.g }
    #[inline(always)]
    pub fn b(self) -> f32 { self.b }
    #[inline(always)]
    pub fn a(self) -> f32 { self.a }
}
