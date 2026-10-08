//! Direct3D's vectors and matrices as Retained Mode used them: a
//! left-handed space (x right, y up, z into the screen), row vectors
//! (`v' = v * M`), a frame's matrix its axes as rows and its origin as the
//! last row.

use std::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub const fn v3(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3 { x, y, z }
}

impl Vec3 {
    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    /// The cross product (as a left-handed space's: x = y × z).
    pub fn cross(self, o: Vec3) -> Vec3 {
        v3(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }

    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Of length 1 (`None`: no direction).
    pub fn unit(self) -> Option<Vec3> {
        let l = self.len();
        (l > 1e-12 && l.is_finite()).then(|| self * (1.0 / l))
    }

    /// The vector made unit as D3DRM makes it (D3DRMVectorNormalize): a
    /// zero vector becomes (1, 0, 0). RC.EXE with RapidQ's d3drm.dll: a
    /// rotation about (0, 0, 0) turns about x (RapidQ's Lights_pyramid
    /// passes one), an orientation whose up is (0, 0, 0) is the one with
    /// up (1, 0, 0) (RapidQ_D3D.inc's QD3DCAMERA leaves it so: 3DPong), a
    /// direction of (0, 0, 0) looks along x.
    pub fn d3drm_unit(self) -> Vec3 {
        self.unit().unwrap_or(v3(1.0, 0.0, 0.0))
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        v3(-self.x, -self.y, -self.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, k: f64) -> Vec3 {
        v3(self.x * k, self.y * k, self.z * k)
    }
}

/// A 4 × 4 matrix, row-major, for row vectors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4(pub [[f64; 4]; 4]);

impl Default for Mat4 {
    fn default() -> Self {
        Mat4::IDENTITY
    }
}

impl Mat4 {
    pub const IDENTITY: Mat4 = Mat4([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);

    pub fn translation(t: Vec3) -> Mat4 {
        let mut m = Mat4::IDENTITY;
        m.0[3][0] = t.x;
        m.0[3][1] = t.y;
        m.0[3][2] = t.z;
        m
    }

    pub fn scaling(s: Vec3) -> Mat4 {
        let mut m = Mat4::IDENTITY;
        m.0[0][0] = s.x;
        m.0[1][1] = s.y;
        m.0[2][2] = s.z;
        m
    }

    /// A turn of `angle` radians about `axis`, clockwise seen along the axis
    /// from its tip — Direct3D's left-handed rotation (`D3DXMatrixRotationAxis`).
    pub fn rotation(axis: Vec3, angle: f64) -> Mat4 {
        let a = axis.d3drm_unit();
        let (s, c) = angle.sin_cos();
        let t = 1.0 - c;
        Mat4([
            [t * a.x * a.x + c, t * a.x * a.y + s * a.z, t * a.x * a.z - s * a.y, 0.0],
            [t * a.x * a.y - s * a.z, t * a.y * a.y + c, t * a.y * a.z + s * a.x, 0.0],
            [t * a.x * a.z + s * a.y, t * a.y * a.z - s * a.x, t * a.z * a.z + c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// The frame looking along `dir` with `up` up, at `at`: its axes as rows
    /// (z = dir, y = up made square to it, x = y × z).
    /// Each vector made unit as D3DRM does ([`Vec3::d3drm_unit`]): up along
    /// dir leaves x (1, 0, 0) (RC.EXE: dir and up both (0, 0, 1) is upright).
    pub fn oriented(dir: Vec3, up: Vec3, at: Vec3) -> Mat4 {
        let z = dir.d3drm_unit();
        let x = up.d3drm_unit().cross(z).d3drm_unit();
        let y = z.cross(x);
        Mat4([[x.x, x.y, x.z, 0.0], [y.x, y.y, y.z, 0.0], [z.x, z.y, z.z, 0.0], [at.x, at.y, at.z, 1.0]])
    }

    pub fn mul(&self, o: &Mat4) -> Mat4 {
        let mut r = [[0.0; 4]; 4];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..4).map(|k| self.0[i][k] * o.0[k][j]).sum();
            }
        }
        Mat4(r)
    }

    /// A point through the matrix.
    pub fn point(&self, p: Vec3) -> Vec3 {
        let m = &self.0;
        v3(
            p.x * m[0][0] + p.y * m[1][0] + p.z * m[2][0] + m[3][0],
            p.x * m[0][1] + p.y * m[1][1] + p.z * m[2][1] + m[3][1],
            p.x * m[0][2] + p.y * m[1][2] + p.z * m[2][2] + m[3][2],
        )
    }

    /// A direction through the matrix (no translation).
    pub fn vector(&self, p: Vec3) -> Vec3 {
        let m = &self.0;
        v3(p.x * m[0][0] + p.y * m[1][0] + p.z * m[2][0], p.x * m[0][1] + p.y * m[1][1] + p.z * m[2][1], p.x * m[0][2] + p.y * m[1][2] + p.z * m[2][2])
    }

    pub fn origin(&self) -> Vec3 {
        v3(self.0[3][0], self.0[3][1], self.0[3][2])
    }

    pub fn set_origin(&mut self, p: Vec3) {
        self.0[3][0] = p.x;
        self.0[3][1] = p.y;
        self.0[3][2] = p.z;
    }

    /// Its axis `i` (0 x, 1 y, 2 z) as a vector.
    pub fn axis(&self, i: usize) -> Vec3 {
        v3(self.0[i][0], self.0[i][1], self.0[i][2])
    }

    /// The inverse of a frame's matrix (rotation, uniform or not scale, and
    /// translation: a general 4 × 4 inverse, affine).
    pub fn inverse(&self) -> Mat4 {
        let m = &self.0;
        // The 3 × 3 part's inverse by cofactors.
        let a = |r: usize, c: usize| m[r][c];
        let det = a(0, 0) * (a(1, 1) * a(2, 2) - a(1, 2) * a(2, 1)) - a(0, 1) * (a(1, 0) * a(2, 2) - a(1, 2) * a(2, 0)) + a(0, 2) * (a(1, 0) * a(2, 1) - a(1, 1) * a(2, 0));
        if det.abs() < 1e-15 {
            return Mat4::IDENTITY;
        }
        let inv = 1.0 / det;
        let mut r = Mat4::IDENTITY;
        r.0[0][0] = (a(1, 1) * a(2, 2) - a(1, 2) * a(2, 1)) * inv;
        r.0[0][1] = (a(0, 2) * a(2, 1) - a(0, 1) * a(2, 2)) * inv;
        r.0[0][2] = (a(0, 1) * a(1, 2) - a(0, 2) * a(1, 1)) * inv;
        r.0[1][0] = (a(1, 2) * a(2, 0) - a(1, 0) * a(2, 2)) * inv;
        r.0[1][1] = (a(0, 0) * a(2, 2) - a(0, 2) * a(2, 0)) * inv;
        r.0[1][2] = (a(0, 2) * a(1, 0) - a(0, 0) * a(1, 2)) * inv;
        r.0[2][0] = (a(1, 0) * a(2, 1) - a(1, 1) * a(2, 0)) * inv;
        r.0[2][1] = (a(0, 1) * a(2, 0) - a(0, 0) * a(2, 1)) * inv;
        r.0[2][2] = (a(0, 0) * a(1, 1) - a(0, 1) * a(1, 0)) * inv;
        let t = r.vector(self.origin());
        r.set_origin(-t);
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: Vec3, b: Vec3) -> bool {
        (a - b).len() < 1e-9
    }

    #[test]
    fn left_handed_rotation_and_frames() {
        // A quarter turn about y takes +z to +x? D3D's left-handed y turn
        // takes +x to -z (seen from +y looking down, clockwise).
        let r = Mat4::rotation(v3(0.0, 1.0, 0.0), std::f64::consts::FRAC_PI_2);
        assert!(near(r.vector(v3(1.0, 0.0, 0.0)), v3(0.0, 0.0, -1.0)));
        assert!(near(r.vector(v3(0.0, 0.0, 1.0)), v3(1.0, 0.0, 0.0)));
        // A frame looking along +x with y up: its z axis is +x, its x axis -z.
        let f = Mat4::oriented(v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0), v3(1.0, 2.0, 3.0));
        assert!(near(f.axis(2), v3(1.0, 0.0, 0.0)));
        assert!(near(f.axis(0), v3(0.0, 0.0, -1.0)));
        assert!(near(f.point(v3(0.0, 0.0, 1.0)), v3(2.0, 2.0, 3.0)));
        let back = f.inverse().point(v3(2.0, 2.0, 3.0));
        assert!(near(back, v3(0.0, 0.0, 1.0)));
        let s = Mat4::scaling(v3(2.0, 3.0, 4.0)).mul(&Mat4::translation(v3(1.0, 1.0, 1.0)));
        assert!(near(s.inverse().point(s.point(v3(5.0, -2.0, 0.5))), v3(5.0, -2.0, 0.5)));
    }
}
