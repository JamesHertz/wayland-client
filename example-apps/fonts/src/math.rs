use std::ops::{Mul, Add};

#[derive(Clone, Copy)]
pub struct Vec2(pub f32, pub f32);

impl Mul<Vec2> for f32 {
    type Output = Vec2;
    fn mul(self, Vec2(x, y) : Vec2) -> Self::Output {
        Vec2(self * x, self * y)
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, Vec2(x, y): Self) -> Self::Output {
        Vec2(self.0 + x, self.1 + y)
    }
}

pub fn lerp(v1 : Vec2, v2 : Vec2, s : f32) -> Vec2 {
    (1.0 - s) * v1 + s * v2
}
