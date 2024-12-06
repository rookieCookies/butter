use std::ops::{Add, AddAssign, Div, DivAssign, Index, Mul, MulAssign, Neg, Sub};

use serde::{Deserialize, Serialize};
use tracing::error;

use super::matrix::Matrix;

pub type Point = Vec3;
pub type Colour = Vec4;

#[derive(Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}


#[derive(Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}


#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(C)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}


#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(C)]
pub struct Vec2i {
    pub x: i32,
    pub y: i32,
}

impl Vec2i {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}


impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}


impl Vec4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}


impl Vec3 {
    pub const ZERO : Vec3 = Vec3::new(0.0, 0.0, 0.0);
    pub const ONE  : Vec3 = Vec3::new(1.0, 1.0, 1.0);

    #[inline(always)]
    pub const fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }


    #[inline(always)]
    pub fn near_zero(self) -> bool {
        const TRESHOLD : f32 = 1e-8;
        self.x.abs() < TRESHOLD
        && self.y.abs() < TRESHOLD
        && self.z.abs() < TRESHOLD
    }

    #[inline(always)]
    pub fn reflect(self, oth: Vec3) -> Vec3 {
        self - 2.0 * self.dot(oth) * oth
    }

    #[inline(always)]
    pub fn refract(self, n: Vec3, etai_over_etat: f32) -> Vec3 {
        let cos_theta = (-self).dot(n).min(1.0);
        let r_out_perp = etai_over_etat * (self + cos_theta*n);
        let r_out_parallel = -(1.0 - r_out_perp.length_squared()).abs().sqrt() * n;
        r_out_perp + r_out_parallel
    }

    #[inline(always)]
    pub fn length_squared(self) -> f32 {
        self.x * self.x +
        self.y * self.y +
        self.z * self.z 
    }

    #[inline(always)]
    pub fn length(self) -> f32 { self.length_squared().sqrt() }

    #[inline(always)]
    pub fn dot(self, rhs: Vec3) -> f32 {
        self.x * rhs.x +
        self.y * rhs.y +
        self.z * rhs.z 
    }

    #[inline(always)]
    pub fn cross(self, rhs: Vec3) -> Vec3 {
        Self::new(self.y * rhs.z - self.z * rhs.y,
                  self.z * rhs.x - self.x * rhs.z,
                  self.x * rhs.y - self.y * rhs.x)
    }


    #[inline(always)]
    pub fn unit(self) -> Vec3 {
        self / self.length()
    }


    #[inline(always)]
    pub fn to_matrix(self) -> Matrix<4, 1, f32> {
        Matrix::new([
            [self.x],
            [self.y],
            [self.z],
            [1.0],
        ])
    }

    pub fn splat(f: f32) -> Vec3 {
        Vec3::new(f, f, f)
    }

}

impl Default for Vec3 {
    #[inline(always)]
    fn default() -> Self { Self::new(0.0, 0.0, 0.0) }
}


impl Neg for Vec3 {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output { Vec3::new(-self.x, -self.y, -self.z) }

}


impl AddAssign for Vec3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}


impl MulAssign<f32> for Vec3 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
    }
}


impl DivAssign<f32> for Vec3 {
    #[inline(always)]
    fn div_assign(&mut self, rhs: f32) {
        *self *= 1.0 / rhs
    }
}


impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}


impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}


impl Mul<Vec3> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
    }
}


impl Mul<Vec3> for f32 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        Vec3::new(self * rhs.x, self * rhs.y, self * rhs.z)
    }
}


impl Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        (1.0 / rhs) * self
    }
}


impl Mul<Vec4> for Vec4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z, self.w * rhs.w)
    }
}





impl Index<usize> for Vec3 {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        if index == 0 { return &self.x }
        if index == 1 { return &self.y }
        if index == 2 { return &self.z }
        unreachable!()
    }
}


impl core::fmt::Debug for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl core::fmt::Display for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}


impl core::fmt::Debug for Vec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl core::fmt::Display for Vec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}


impl core::fmt::Debug for Vec4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}

impl core::fmt::Display for Vec4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}


impl Vec2 {
    pub fn from_table(parent_name: &str, table: &toml::Table) -> Option<Self> {
        let f = || { Some(Self::new(table.get("x")?.as_float()? as f32,
                                    table.get("y")?.as_float()? as f32)) };
        match f() {
            Some(val) => Some(val),
            None => {
                error!("unable to read a vec2 in '{parent_name}'");
                None
            }
        }
    }

    pub fn to_table(self) -> toml::Table {
        let mut table = toml::Table::new();
        table.insert("x".to_string(), self.x.into());
        table.insert("y".to_string(), self.y.into());
        table
    }
}

impl Vec3 {
    pub fn from_table(parent_name: &str, table: &toml::Table) -> Option<Self> {
        let f = || { Some(Self::new(table.get("x")?.as_float()? as f32,
                                    table.get("y")?.as_float()? as f32,
                                    table.get("z")?.as_float()? as f32)) };
        match f() {
            Some(val) => Some(val),
            None => {
                error!("unable to read a vec3 in '{parent_name}'");
                None
            }
        }
    }

    pub fn to_table(self) -> toml::Table {
        let mut table = toml::Table::new();
        table.insert("x".to_string(), self.x.into());
        table.insert("y".to_string(), self.y.into());
        table.insert("z".to_string(), self.z.into());
        table
    }
}

impl Vec4 {
    pub fn from_table(parent_name: &str, table: &toml::Table) -> Option<Self> {
        let f = || { Some(Self::new(table.get("x")?.as_float()? as f32,
                                    table.get("y")?.as_float()? as f32,
                                    table.get("z")?.as_float()? as f32,
                                    table.get("w")?.as_float()? as f32)) };
        match f() {
            Some(val) => Some(val),
            None => {
                error!("unable to read a vec4 in '{parent_name}'");
                None
            }
        }
    }

    pub fn to_table(self) -> toml::Table {
        let mut table = toml::Table::new();
        table.insert("x".to_string(), self.x.into());
        table.insert("y".to_string(), self.y.into());
        table.insert("z".to_string(), self.z.into());
        table.insert("w".to_string(), self.w.into());
        table
    }
}
