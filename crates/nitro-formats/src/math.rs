//! Small column-major math helpers shared by the format decoders and exporters.
pub type Mat4 = [f32; 16];
pub const IDENTITY: Mat4 = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];
pub fn mul(a: Mat4, b: Mat4) -> Mat4 {
    std::array::from_fn(|i| (0..4).map(|k| a[k * 4 + i % 4] * b[i / 4 * 4 + k]).sum())
}
pub fn point(m: Mat4, p: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|r| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r])
}
pub fn vector(m: Mat4, p: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|r| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2])
}
pub fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
pub fn normalize(a: [f32; 3]) -> [f32; 3] {
    let n = dot(a, a).sqrt();
    if n > 0. {
        a.map(|v| v / n)
    } else {
        a
    }
}
pub fn trs(t: [f32; 3], r: [f32; 4], s: [f32; 3]) -> Mat4 {
    let [x, y, z, w] = r;
    [
        (1. - 2. * (y * y + z * z)) * s[0],
        2. * (x * y + z * w) * s[0],
        2. * (x * z - y * w) * s[0],
        0.,
        2. * (x * y - z * w) * s[1],
        (1. - 2. * (x * x + z * z)) * s[1],
        2. * (y * z + x * w) * s[1],
        0.,
        2. * (x * z + y * w) * s[2],
        2. * (y * z - x * w) * s[2],
        (1. - 2. * (x * x + y * y)) * s[2],
        0.,
        t[0],
        t[1],
        t[2],
        1.,
    ]
}
pub fn quaternion(m: [f32; 9]) -> [f32; 4] {
    let tr = m[0] + m[4] + m[8];
    let q = if tr > 0. {
        let s = (tr + 1.).sqrt() * 2.;
        [
            (m[5] - m[7]) / s,
            (m[6] - m[2]) / s,
            (m[1] - m[3]) / s,
            s / 4.,
        ]
    } else {
        let i = if m[0] > m[4] && m[0] > m[8] {
            0
        } else if m[4] > m[8] {
            1
        } else {
            2
        };
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        let s = (1. + m[i * 3 + i] - m[j * 3 + j] - m[k * 3 + k])
            .max(0.)
            .sqrt()
            * 2.;
        if s < 1e-8 {
            return [0., 0., 0., 1.];
        }
        let mut q = [0.; 4];
        q[i] = s / 4.;
        q[j] = (m[i * 3 + j] + m[j * 3 + i]) / s;
        q[k] = (m[i * 3 + k] + m[k * 3 + i]) / s;
        q[3] = (m[j * 3 + k] - m[k * 3 + j]) / s;
        q
    };
    let n = q.iter().map(|v| v * v).sum::<f32>().sqrt();
    q.map(|v| v / n)
}
pub fn inverse(m: Mat4) -> Option<Mat4> {
    let mut a = [[0.; 8]; 4];
    for r in 0..4 {
        for c in 0..4 {
            a[r][c] = m[c * 4 + r]
        }
        a[r][4 + r] = 1.;
    }
    for c in 0..4 {
        let p = (c..4).max_by(|&i, &j| a[i][c].abs().total_cmp(&a[j][c].abs()))?;
        if a[p][c].abs() < 1e-10 {
            return None;
        }
        a.swap(c, p);
        let d = a[c][c];
        for x in &mut a[c] {
            *x /= d
        }
        for r in 0..4 {
            if r != c {
                let v = a[r][c];
                for k in 0..8 {
                    a[r][k] -= v * a[c][k]
                }
            }
        }
    }
    Some(std::array::from_fn(|i| a[i % 4][4 + i / 4]))
}
