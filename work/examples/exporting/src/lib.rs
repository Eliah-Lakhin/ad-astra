use std::{
    f64::consts::PI,
    fmt::{Display, Formatter},
    ops::{Add, Mul, Neg},
};

use ad_astra::export;

/// An example package with basic 2D coordinate system transformation features.
#[export(package)]
#[derive(Default)]
struct Package;

/// Converts degrees to radians.
#[export]
pub fn deg(degrees: f64) -> f64 {
    PI * degrees / 180.0
}

/// Converts radians to degrees.
#[export]
pub fn rad(radians: f64) -> f64 {
    180.0 * radians / PI
}

/// Rounds a floating-point number up to the nearest integer.
#[export]
pub fn round(value: f64) -> i64 {
    value.round() as i64
}

/// A 2D vector.
#[export]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector {
    /// The x-coordinate of the vector.
    pub x: f64,

    /// The y-coordinate of the vector.
    pub y: f64,
}

#[export]
impl Neg for Vector {
    type Output = Self;

    fn neg(mut self) -> Self::Output {
        self.x = -self.x;
        self.y = -self.y;

        self
    }
}

#[export]
impl Add for Vector {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.x += rhs.x;
        self.y += rhs.y;

        self
    }
}

#[export]
impl Display for Vector {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("vec({}, {})", self.x, self.y))
    }
}

#[export]
impl Vector {
    /// Constructs a new 2D vector.
    #[export(name "vec")]
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Returns the magnitude (or length) of the vector.
    pub fn radius(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Returns the angle of the vector in the 2D coordinate system.
    pub fn angle(&self) -> f64 {
        self.y.atan2(self.x)
    }

    /// Normalizes this vector by setting its magnitude to 1 while preserving
    /// the original angle.
    pub fn normalize(&mut self) -> &mut Self {
        let r = self.radius();

        self.x /= r;
        self.y /= r;

        self
    }

    /// Transforms this vector using the provided transformation matrix.
    pub fn transform(&mut self, matrix: &Matrix) -> &mut Self {
        let x = matrix.x.x * self.x + matrix.x.y * self.y;
        let y = matrix.y.x * self.x + matrix.y.y * self.y;

        self.x = x;
        self.y = y;

        self
    }
}

/// A 2x2 transformation matrix.
#[export]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    x: Vector,
    y: Vector,
}

#[export]
impl Mul for Matrix {
    type Output = Self;

    fn mul(self, rhs: Matrix) -> Self::Output {
        Self {
            x: Vector {
                x: self.x.x * rhs.x.x + self.x.y * rhs.y.x,
                y: self.x.x * rhs.x.y + self.x.y * rhs.y.y,
            },
            y: Vector {
                x: self.y.x * rhs.x.x + self.y.y * rhs.y.x,
                y: self.y.x * rhs.x.y + self.y.y * rhs.y.y,
            },
        }
    }
}

#[export]
impl Matrix {
    /// Creates a 2x2 transformation matrix that rotates the coordinate system
    /// by the specified angle in radians.
    pub fn rotation(angle: f64) -> Self {
        let (sin, cos) = angle.sin_cos();

        Self {
            x: Vector { x: cos, y: -sin },
            y: Vector { x: sin, y: cos },
        }
    }

    /// Computes the determinant of this matrix.
    pub fn det(&self) -> f64 {
        self.x.x * self.y.y - self.x.y * self.y.x
    }

    /// Constructs a new matrix that is the inverse of this one.
    pub fn invert(&self) -> Self {
        let det = self.det();

        Self {
            x: Vector {
                x: self.y.y / det,
                y: -self.x.y / det,
            },
            y: Vector {
                x: -self.y.x / det,
                y: self.x.x / det,
            },
        }
    }
}
