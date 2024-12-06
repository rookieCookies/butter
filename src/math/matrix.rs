use std::{mem::forget, ops::{Add, AddAssign, Index, IndexMut, Mul, Sub}};

use super::vector::{Point, Vec2, Vec3};


pub type Matrix4<T> = Matrix<4, 4, T>;


#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(C)]
pub struct Matrix<const COLUMN: usize, const ROW: usize, T> {
    cols: [[T; ROW]; COLUMN]
}


impl Matrix<4, 4, f32> {
    pub const IDENTITY : Matrix<4, 4, f32> = Matrix {
        cols: [[1.0, 0.0, 0.0, 0.0],
               [0.0, 1.0, 0.0, 0.0],
               [0.0, 0.0, 1.0, 0.0],
               [0.0, 0.0, 0.0, 1.0]],
    };


    pub fn look_at(eye: Point, centre: Point, up: Vec3) -> Self {
        Self::look_to(eye, centre - eye, up) 
    }

    pub fn look_to(eye: Point, dir: Point, up: Vec3) -> Self {
        let f = dir.unit();
        let s = f.cross(up).unit();
        let u = s.cross(f);

        Self::new([
            [s.x, u.x, -f.x, 0.0],
            [s.y, u.y, -f.y, 0.0],
            [s.z, u.z, -f.z, 0.0],
            [-eye.dot(s), -eye.dot(u), eye.dot(f), 1.0],
        ])
    }


    pub fn rotate_2d(r: f32) -> Self {
        Self::new([
            [r.cos(), r.sin(), 0.0, 0.0],
            [-r.sin(), r.cos(), 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }



    pub fn axis_rotation(axis: Vec3, angle_in_rads: f32) -> Self {
        let a = axis;
        let (s, c) = angle_in_rads.sin_cos();
        let _1subc = 1.0 - c;
        let sc = _1subc;
        Self::new([
            [sc * a.x * a.x + c, sc * a.x * a.y + s * a.z, sc * a.x * a.z - s * a.y, 0.0],
            [sc * a.x * a.y - s * a.z, sc * a.y * a.y + c, sc * a.y * a.z + s * a.x, 0.0],
            [sc * a.x * a.z + s * a.y, sc * a.y * a.z - s * a.x, sc * a.z * a.z + c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            ])
    }


    pub fn translation(v: Vec3) -> Self {
        Self::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [v.x, v.y, v.z, 1.0],
            ])
    }


    pub fn scaling(v: Vec3) -> Self {
        Self::new([
            [v.x, 0.0, 0.0, 0.0],
            [0.0, v.y, 0.0, 0.0],
            [0.0, 0.0, v.z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            ])
    }


    // this is just a inlined version of 
    // `Matrix::translation(pos) * Matrix::scaling(vec3(scale, 1.0)) *
    // Matrix::rotate2d(rot)`
    pub fn pos_scale_rot(position: Vec2, scale: Vec2, rotation: f32) -> Self {
        let px = position.x;
        let py = position.y;
        let sx = scale.x;
        let sy = scale.y;
        let rc = rotation.cos();
        let rz = rotation.sin();
        Matrix::new([
            [ sx*rc, sy*rz, 0.0, 0.0],
            [-sx*rz, sy*rc, 0.0, 0.0],
            [   0.0,   0.0, 1.0, 0.0],
            [    px,    py, 0.0, 1.0],
        ])
    }


    pub fn orthographic(l: f32, r: f32, b: f32, t: f32, n: f32, f: f32) -> Self {
        Self::new([
            [2.0/(r-l), 0.0, 0.0, 0.0],
            [0.0, 2.0/(t-b), 0.0, 0.0],
            [0.0, 0.0, -2.0/(f-n), 0.0],
            [-(r+l)/(r-l), -(t+b)/(t-b), -(f+n)/(f-n), 1.0],
        ])
    }


    pub fn perspective(fovy_in_rads: f32, aspect: f32, near: f32, far: f32) -> Self {
        let fovy = fovy_in_rads;
        let angle = fovy / 2.0;
        let ymax = near * angle.tan();
        let xmax = ymax * aspect;

        let left = -xmax;
        let right = xmax;
        let bottom = -ymax;
        let top = ymax;

        assert!(
            left <= right,
            "`left` cannot be greater than `right`, found: left: {:?} right: {:?}",
            left,
            right
        );
        assert!(
            bottom <= top,
            "`bottom` cannot be greater than `top`, found: bottom: {:?} top: {:?}",
            bottom,
            top
        );
        assert!(
            near <= far,
            "`near` cannot be greater than `far`, found: near: {:?} far: {:?}",
            near,
            far
        );

        let c0r0 = (2.0 * near) / (right - left);
        let c0r1 = 0.0;
        let c0r2 = 0.0;
        let c0r3 = 0.0;

        let c1r0 = 0.0;
        let c1r1 = (2.0 * near) / (top - bottom);
        let c1r2 = 0.0;
        let c1r3 = 0.0;

        let c2r0 = (right + left) / (right - left);
        let c2r1 = (top + bottom) / (top - bottom);
        let c2r2 = -(far + near) / (far - near);
        let c2r3 = -1.0;

        let c3r0 = 0.0;
        let c3r1 = 0.0;
        let c3r2 = -(2.0 * far * near) / (far - near);
        let c3r3 = 0.0;

        Self::new([
            [c0r0, c0r1, c0r2, c0r3],
            [c1r0, c1r1, c1r2, c1r3],
            [c2r0, c2r1, c2r2, c2r3],
            [c3r0, c3r1, c3r2, c3r3],
        ])
    }
}


impl<const COLUMN: usize, const ROW: usize, T> Matrix<COLUMN, ROW, T> {

    pub fn new_rows(rows: [[T; COLUMN]; ROW]) -> Self {
        let slf = Self::new(std::array::from_fn::<[T; ROW], COLUMN, _>(|i| {
            std::array::from_fn::<T, ROW, _>(|j| {
                unsafe { ((&rows[j][i]) as *const T).read() }
            })
        }));

        forget(rows);
        slf
    }


    pub fn new(cols: [[T; ROW]; COLUMN]) -> Self {
        Self {
            cols,
        }
    }
}



impl<const COLUMN: usize, const ROW: usize, T: Copy> Matrix<COLUMN, ROW, T> {
    pub fn scale<V, A: Copy + Mul<T, Output = V>>(self, scale_factor: A) -> Matrix<COLUMN, ROW, V> {
        let arr = std::array::from_fn::<[V; ROW], COLUMN, _>(|i| {
            std::array::from_fn::<V, ROW, _>(|j| {
                scale_factor * self.cols[i][j] 
            })
        });

        Matrix::new(arr)
    }
}

impl<const COLUMN: usize, const ROW: usize, V, T: Add<Output=V> + Copy> Add for Matrix<COLUMN, ROW, T> {
    type Output = Matrix<COLUMN, ROW, V>;

    fn add(self, rhs: Self) -> Self::Output {
        let arr = std::array::from_fn::<[V; ROW], COLUMN, _>(|i| {
            std::array::from_fn::<V, ROW, _>(|j| {
                self.cols[i][j] + rhs.cols[i][j]
            })
        });

        Matrix::new(arr)
        
    }
}


impl<const COLUMN: usize, const ROW: usize, V, T: Sub<Output=V> + Copy> Sub for Matrix<COLUMN, ROW, T> {
    type Output = Matrix<COLUMN, ROW, V>;

    fn sub(self, rhs: Self) -> Self::Output {
        let arr = std::array::from_fn::<[V; ROW], COLUMN, _>(|i| {
            std::array::from_fn::<V, ROW, _>(|j| {
                self.cols[i][j] - rhs.cols[i][j]
            })
        });

        Matrix::new(arr)
        
    }
}


impl<const COLUMN: usize, const ROW: usize, const COLUMN_TWO: usize, V: AddAssign, T: Mul<Output=V> + Copy>
            Mul<Matrix<COLUMN_TWO, COLUMN, T>> for Matrix<COLUMN, ROW, T> {
    type Output = Matrix<COLUMN_TWO, COLUMN, V>;

    fn mul(self, rhs: Matrix<COLUMN_TWO, COLUMN, T>) -> Self::Output {
        let arr = std::array::from_fn::<[V; COLUMN], COLUMN_TWO, _>(|i| {
            std::array::from_fn::<V, COLUMN, _>(|j| {
                let mut res = None;
                for k in 0..ROW {
                    let r = self.cols[k][j] * rhs.cols[i][k];
                    if let Some(res) = &mut res {
                        *res += r;
                    } else {
                        res = Some(r);
                    }
                }
                res.unwrap()
            })
        });

        let m = Matrix::new(arr);
        m
        
    }
}


impl<const COLUMN: usize, const ROW: usize, T> Index<usize> for Matrix<COLUMN, ROW, T> {
    type Output = [T; ROW];

    fn index(&self, index: usize) -> &Self::Output {
        &self.cols[index]
    }
}

impl<const COLUMN: usize, const ROW: usize, T> IndexMut<usize> for Matrix<COLUMN, ROW, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.cols[index]
    }
}


#[cfg(test)]
mod tests {
    use std::f32::EPSILON;


    use super::*;


    #[test]
    fn matrix_perspective() {
        let m1 = Matrix::perspective(
            60.0f32.to_radians(),
            16.0 / 9.0,
            0.01,
            1000.0,
        );

        let m2 = cgmath::perspective(
            cgmath::Rad(60.0f32.to_radians()),
            16.0 / 9.0,
            0.01,
            1000.0,
        );


        for i in 0..4 {
            for j in 0..4 {
                assert!((m1.cols[i][j] - m2[i][j]) <= EPSILON);
            }
        }
    }


    #[test]
    fn matrix_lookat() {
        let x = 12.0;
        let y = 5.23;
        let z = 63.4;
        let Vec3 { x: ux, y: uy, z: uz } = Vec3::new(213.203, 49385.23, 5498.198).unit();
        let m1 = Matrix::look_at(Point::new(x, y, z),
                                Point::new(0.0, 0.0, 0.0),
                                Vec3::new(ux, uy, uz));

        let m2 = cgmath::Matrix4::look_at_rh(
            cgmath::point3(x, y, z),
            cgmath::point3(0.0, 0.0, 0.0),
            cgmath::vec3(ux, uy, uz));

        for i in 0..4 {
            for j in 0..4 {
                assert!((m1.cols[i][j] - m2[i][j]) <= EPSILON, "{} vs {}", m1.cols[i][j], m2[i][j]);
            }
        }
    }


    #[test]
    fn matrix_ortho() {
        let l = 35.0;
        let r = 69.0;
        let b = 420.0;
        let t = 600.0;
        let n = 0.2;
        let f = 100.0;


        let m1 = Matrix::orthographic(l, r, b, t, n, f);
        let m2 = cgmath::ortho(l, r, b, t, n, f);

        for i in 0..4 {
            for j in 0..4 {
                assert!((m1.cols[i][j] - m2[i][j]) <= EPSILON, "{i},{j} found {} expected {}\n {:?} vs {:?}", m1[i][j], m2[i][j], m1, m2);
            }
        }
    }


    #[test]
    fn matrix_addition() {
        let m1 = Matrix::new([
            [1, 2],
            [3, 4],
        ]);

        let m2 = Matrix::new([
            [5, 6],
            [7, 8],
        ]);


        let m3 = Matrix::new([
            [6 , 8 ],
            [10, 12],
        ]);


        assert_eq!(m1 + m2, m3)
    }


    #[test]
    fn matrix_sub() {
        let m1 = Matrix::new([
            [4, 2],
            [1, 6],
        ]);

        let m2 = Matrix::new([
            [2, 4],
            [0, 1],
        ]);


        let m3 = Matrix::new([
            [2, -2],
            [1,  5],
        ]);


        assert_eq!(m1 - m2, m3)
    }


    #[test]
    fn matrix_scale() {
        let m1 = Matrix::new([
            [1, 3],
            [2, 4],
        ]);


        let m2 = Matrix::new([
            [2, 6],
            [4, 8],
        ]);


        assert_eq!(m1.scale(2), m2);
    }

    #[test]
    fn matrix_multiplication() {
        let m1 = Matrix::new([
            [1, 3],
            [2, 4],
        ]);

        let m2 = Matrix::new([
            [5, 7],
            [6, 8],
        ]);


        let m3 = Matrix::new([
            [19, 43],
            [22, 50],
        ]);


        assert_eq!(m1 * m2, m3);


        let m1 = Matrix::new([
            [1, 0, 0, 0],
            [0, 1, 0, 0],
            [0, 0, 1, 0],
            [0, 0, 0, 1],
        ]);

        let m2 = Matrix::new([
            [5, 7, 3, 5],
        ]);

        assert_eq!(m1 * m2, m2)
    }

}
