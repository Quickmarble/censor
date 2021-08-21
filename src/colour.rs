use image::Rgb;
use serde::{Serialize, Deserialize};

use crate::util::{Clip, CyclicClip, Lerp};

use std::f32::consts::PI;

pub trait Vector {
    fn dist(x: &Self, y: &Self) -> f32;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RGB255 {
    pub r: u8,
    pub g: u8,
    pub b: u8
}
impl Vector for RGB255 {
    fn dist(x: &Self, y: &Self) -> f32 {
        f32::sqrt(
            (x.r as f32 - y.r as f32).powi(2) +
            (x.g as f32 - y.g as f32).powi(2) +
            (x.b as f32 - y.b as f32).powi(2)
        )
    }
}
impl RGB255 {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

impl From<RGB255> for Rgb<u8> {
    fn from(c: RGB255) -> Self {
        Self([c.r, c.g, c.b])
    }
}

fn ungamma(x: f32) -> f32 {
    if x <= 0.04045 {
        25. * x / 323.
    } else {
        ((200. * x + 11.) / 211.).powf(12. / 5.)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct RGB1 {
    pub r: f32,
    pub g: f32,
    pub b: f32
}
impl From<RGB255> for RGB1 {
    fn from(c: RGB255) -> Self {
        Self {
            r: (f32::from(c.r) / 255.).clip(0., 1.),
            g: (f32::from(c.g) / 255.).clip(0., 1.),
            b: (f32::from(c.b) / 255.).clip(0., 1.)
        }
    }
}
impl Vector for RGB1 {
    fn dist(x: &Self, y: &Self) -> f32 {
        f32::sqrt(
            (x.r - y.r).powi(2) +
            (x.g - y.g).powi(2) +
            (x.b - y.b).powi(2)
        )
    }
}
#[allow(dead_code)]
impl RGB1 {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct CIEXYZ {
    pub X: f32,
    pub Y: f32,
    pub Z: f32
}
impl From<RGB1> for CIEXYZ {
    fn from(c: RGB1) -> Self {
        let r = ungamma(c.r);
        let g = ungamma(c.g);
        let b = ungamma(c.b);
        let X = 0.4124 * r + 0.3576 * g + 0.1805 * b;
        let Y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let Z = 0.0193 * r + 0.1192 * g + 0.9505 * b;
        Self { X: X * 100., Y: Y * 100., Z: Z * 100. }
    }
}
impl From<RGB255> for CIEXYZ {
    fn from(c: RGB255) -> Self {
        Self::from(RGB1::from(c))
    }
}
impl From<Wavelength> for CIEXYZ {
    fn from(c: Wavelength) -> Self {
        let wl = c.wl as f64;
        let X = broken_gaussian(wl,  1.056, 5998., 379., 310.) +
                broken_gaussian(wl,  0.362, 4420., 160., 267.) +
                broken_gaussian(wl, -0.065, 5011., 204., 262.);
        let Y = broken_gaussian(wl,  0.821, 5688., 469., 405.) +
                broken_gaussian(wl,  0.286, 5309., 163., 311.);
        let Z = broken_gaussian(wl,  1.217, 4370., 118., 360.) +
                broken_gaussian(wl,  0.681, 4590., 260., 138.);
        Self { X: X * 100., Y: Y * 100., Z: Z * 100. }
    }
}
impl Vector for CIEXYZ {
    fn dist(x: &Self, y: &Self) -> f32 {
        f32::sqrt(
            (x.X - y.X).powi(2) +
            (x.Y - y.Y).powi(2) +
            (x.Z - y.Z).powi(2)
        )
    }
}
#[allow(dead_code)]
impl CIEXYZ {
    pub fn new(X: f32, Y: f32, Z: f32) -> Self {
        Self { X, Y, Z }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct CIExyY {
    pub x: f32,
    pub y: f32,
    pub Y: f32
}
impl From<CIEXYZ> for CIExyY {
    fn from(c: CIEXYZ) -> Self {
        let sum = c.X + c.Y + c.Z;
        if sum > 0. {
            Self { x: c.X / sum, y: c.Y / sum, Y: c.Y }
        } else {
            Self { x: 0., y: 0., Y: 0. }
        }
    }
}
impl From<RGB1> for CIExyY {
    fn from(c: RGB1) -> Self {
        Self::from(CIEXYZ::from(c))
    }
}
impl From<RGB255> for CIExyY {
    fn from(c: RGB255) -> Self {
        Self::from(RGB1::from(c))
    }
}
impl From<Wavelength> for CIExyY {
    fn from(c: Wavelength) -> Self {
        Self::from(CIEXYZ::from(c))
    }
}
impl Vector for CIExyY {
    fn dist(x: &Self, y: &Self) -> f32 {
        f32::sqrt(
            (x.x - y.x).powi(2) +
            (x.y - y.y).powi(2) +
            (x.Y - y.Y).powi(2)
        )
    }
}
#[allow(dead_code)]
impl CIExyY {
    pub fn new(x: f32, y: f32, Y: f32) -> Self {
        Self { x, y, Y }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct CIExy {
    pub x: f32,
    pub y: f32
}
impl From<CIExyY> for CIExy {
    fn from(c: CIExyY) -> Self {
        Self { x: c.x, y: c.y }
    }
}
impl From<CIEXYZ> for CIExy {
    fn from(c: CIEXYZ) -> Self {
        Self::from(CIExyY::from(c))
    }
}
impl From<RGB1> for CIExy {
    fn from(c: RGB1) -> Self {
        Self::from(CIEXYZ::from(c))
    }
}
impl From<RGB255> for CIExy {
    fn from(c: RGB255) -> Self {
        Self::from(RGB1::from(c))
    }
}
impl From<Wavelength> for CIExy {
    fn from(c: Wavelength) -> Self {
        Self::from(CIEXYZ::from(c))
    }
}
impl Vector for CIExy {
    fn dist(x: &Self, y: &Self) -> f32 {
        f32::sqrt(
            (x.x - y.x).powi(2) +
            (x.y - y.y).powi(2)
        )
    }
}
#[allow(dead_code)]
impl CIExy {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn from_T(T: f32) -> Self {
        let x = if T <= 7000. {
            0.244063
                + 0.09911 * 1000. / T
                + 2.9678 * 1000000. / T.powi(2)
                - 4.6070 * 1000000000. / T.powi(9)
        } else {
            0.237040
                + 0.24748 * 1000. / T
                + 1.9018 * 1000000. / T.powi(2)
                - 2.0064 * 1000000000. / T.powi(9)
        };
        let y = -3. * x.powi(2) + 2.87 * x - 0.275;
        Self { x, y }
    }
    pub fn D65() -> Self {
        Self { x: 0.31270, y: 0.32900 }
    }
    pub fn with_Y(self, Y: f32) -> CIExyY {
        CIExyY { x: self.x, y: self.y, Y }
    }
    pub fn hue_relative_to(self, o: Self) -> f32 {
        let dx = self.x - o.x;
        let dy = self.y - o.y;
        let mut a = f32::atan2(dy, dx);
        a += PI / 2.;
        a /= 2. * PI;
        return a.cyclic_clip(1.);
        //     0.5
        //    /   \
        //  0.75  0.25
        //    \   /
        //     1|0
    }
    pub fn has_spectral(self, o: Self) -> bool {
        let h = self.hue_relative_to(o);
        let w_min = CIExy::from(CIExyY::from(CIEXYZ::from(Wavelength::new(Wavelength::MIN as f32))));
        let w_max = CIExy::from(CIExyY::from(CIEXYZ::from(Wavelength::new(Wavelength::MAX as f32))));
        let h_min = w_max.hue_relative_to(o);
        let h_max = w_min.hue_relative_to(o);
        return (h_min <= h) && (h <= h_max);
    }
    pub fn nearest_spectral(self, o: Self) -> Wavelength {
        let h = self.hue_relative_to(o);
        let mut best_wl = 0;
        let mut min = f32::MAX;
        for wl in (Wavelength::MIN..=Wavelength::MAX).step_by(Wavelength::STEP) {
            let w = CIExy::from(CIExyY::from(CIEXYZ::from(Wavelength::new(wl as f32))));
            let s_h = w.hue_relative_to(o);
            let d = f32::abs(s_h - h);
            if d < min {
                best_wl = wl;
                min = d;
            }
        }
        return Wavelength::new(best_wl as f32);
    }
    pub fn try_nearest_spectral(self, o: Self) -> Option<Wavelength> {
        if self.has_spectral(o) {
            Some(self.nearest_spectral(o))
        } else {
            None
        }
    }
}

fn broken_gaussian(x: f64, a: f64, mu: f64, s1: f64, s2: f64) -> f32 {
    let s = if x <= mu { s1 } else { s2 };
    let t = (x - mu) / s;
    let y = a * f64::exp(-(t * t) / 2.);
    return y as f32;
}

#[derive(Clone, Copy, PartialEq)]
pub struct Wavelength {
    /// Wavelength in angstroms.
    pub wl: f32
}
impl Vector for Wavelength {
    fn dist(x: &Self, y: &Self) -> f32 {
        f32::abs(x.wl - y.wl)
    }
}
impl Wavelength {
    pub const MIN: usize = 4100;
    pub const MAX: usize = 6650;
    pub const STEP: usize = 5;
    pub fn new(wl: f32) -> Self {
        Self { wl }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct CIEuv {
    pub u: f32,
    pub v: f32
}
impl From<CIExy> for CIEuv {
    fn from(c: CIExy) -> Self {
        Self {
            u: (0.4661*c.x + 0.1593*c.y) / (c.y - 0.15735*c.x + 0.2424),
            v: 0.6581*c.y / (c.y - 0.15735*c.x + 0.2424)
        }
    }
}
impl From<CIExyY> for CIEuv {
    fn from(c: CIExyY) -> Self {
        Self::from(CIExy::from(c))
    }
}
impl From<CIEXYZ> for CIEuv {
    fn from(c: CIEXYZ) -> Self {
        Self::from(CIExyY::from(c))
    }
}
impl From<RGB1> for CIEuv {
    fn from(c: RGB1) -> Self {
        Self::from(CIEXYZ::from(c))
    }
}
impl From<RGB255> for CIEuv {
    fn from(c: RGB255) -> Self {
        Self::from(RGB1::from(c))
    }
}
impl From<Wavelength> for CIEuv {
    fn from(c: Wavelength) -> Self {
        Self::from(CIEXYZ::from(c))
    }
}
impl Vector for CIEuv {
    fn dist(x: &Self, y: &Self) -> f32 {
        f32::sqrt(
            (x.u - y.u).powi(2) +
            (x.v - y.v).powi(2)
        )
    }
}
#[allow(dead_code)]
impl CIEuv {
    pub const CCT_MIN: usize = 1000;
    pub const CCT_MAX: usize = 25000;
    pub const CCT_STEP: usize = 100;
    pub fn new(u: f32, v: f32) -> Self {
        Self { u, v }
    }
    // TODO: cache!
    pub fn CCT_table() -> Vec<(f32, CIEuv)> {
        let mut table = vec![];
        for T in (Self::CCT_MIN..=Self::CCT_MAX).step_by(Self::CCT_STEP) {
            let uv = Self::from(CIExy::from_T(T as f32));
            table.push((T as f32, uv));
        }
        return table;
    }
    pub fn CCT(self) -> Option<(f32, f32)> {
        let mut best_T = 0.;
        let mut min = f32::MAX;
        for (T, uv) in Self::CCT_table() {
            let d = Self::dist(&self, &uv);
            if d < min {
                best_T = T;
                min = d;
            }
        }
        if min <= 0.05 {
            return Some((best_T, min));
        } else {
            return None;
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct CAT16Illuminant {
    pub X_w: f32,
    pub Y_w: f32,
    pub Z_w: f32,
    pub SF: f32,
    pub Sc: f32,
    pub SN_c: f32,
    pub L_A: f32,
    pub R_w: f32,
    pub G_w: f32,
    pub B_w: f32,
    pub D: f32,
    pub D_R: f32,
    pub D_G: f32,
    pub D_B: f32,
    pub k: f32,
    pub F_L: f32,
    pub n: f32,
    pub z: f32,
    pub N_bb: f32,
    pub N_cb: f32,
    pub R_wc: f32,
    pub G_wc: f32,
    pub B_wc: f32,
    pub R_aw: f32,
    pub G_aw: f32,
    pub B_aw: f32,
    pub A_w: f32
}
impl CAT16Illuminant {
    pub fn new(xy: CIExy) -> Self {
        let x = xy.x;
        let y = xy.y;

        let Y_w = 100.;
        let X_w = Y_w * x / y;
        let Z_w = Y_w * (1. - x - y) / y;

        // dim surround
        let SF = 0.9;
        let Sc = 0.590;
        let SN_c = 0.9;

        let E_w = 64.;
        let L_w = E_w / PI;
        let Y_b = 20.;

        let L_A = L_w * Y_b / Y_w;

        let R_w =  0.401288 * X_w + 0.650173 * Y_w - 0.051461 * Z_w;
        let G_w = -0.250268 * X_w + 1.204414 * Y_w + 0.045854 * Z_w;
        let B_w = -0.002079 * X_w + 0.048952 * Y_w + 0.953127 * Z_w;

        let mut D = SF * (1. - (1. / 3.6) * f32::exp((-L_A - 42.) / 92.));
        D = D.clip(0., 1.);

        let D_R = D * Y_w / R_w + 1. - D;
        let D_G = D * Y_w / G_w + 1. - D;
        let D_B = D * Y_w / B_w + 1. - D;

        let k = 1. / (5. * L_A + 1.);
        let F_L = 0.2 * k.powi(4) * 5. * L_A + 0.1 * (1. - k.powi(4)).powi(2) * (5. * L_A).cbrt();
        let n = Y_b / Y_w;
        let z = 1.48 + n.sqrt();

        let N_bb = 0.725 * (1./n).powf(0.2);
        let N_cb = N_bb;

        let R_wc = D_R * R_w;
        let G_wc = D_G * G_w;
        let B_wc = D_B * B_w;

        let R_aw = 400. * (F_L*R_wc/100.).powf(0.42) / ((F_L*R_wc/100.).powf(0.42) + 27.13) + 0.1;
        let G_aw = 400. * (F_L*G_wc/100.).powf(0.42) / ((F_L*G_wc/100.).powf(0.42) + 27.13) + 0.1;
        let B_aw = 400. * (F_L*B_wc/100.).powf(0.42) / ((F_L*B_wc/100.).powf(0.42) + 27.13) + 0.1;

        let A_w = N_bb * (2. * R_aw + G_aw + 0.05 * B_aw - 0.305);

        CAT16Illuminant {
            X_w, Y_w, Z_w,
            SF, Sc, SN_c,
            L_A,
            R_w, G_w, B_w,
            D,
            D_R, D_G, D_B,
            k, F_L, n, z,
            N_bb, N_cb,
            R_wc, G_wc, B_wc,
            R_aw, G_aw, B_aw,
            A_w
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CAM16UCS {
    pub J: f32,
    pub a: f32,
    pub b: f32,
    pub C: f32
}
impl Vector for CAM16UCS {
    fn dist(x: &Self, y: &Self) -> f32 {
        f32::sqrt(
            (x.J - y.J).powi(2) +
            (x.a - y.a).powi(2) +
            (x.b - y.b).powi(2)
        )
    }
}
impl CAM16UCS {
    pub fn of(c: CIEXYZ, ill: &CAT16Illuminant) -> Self {
        let (X, Y, Z) = (c.X, c.Y, c.Z);
        let R =  0.401288 * X + 0.650173 * Y - 0.051461 * Z;
        let G = -0.250268 * X + 1.204414 * Y + 0.045854 * Z;
        let B = -0.002079 * X + 0.048952 * Y + 0.953127 * Z;

        let R_c = R * ill.D_R;
        let G_c = G * ill.D_G;
        let B_c = B * ill.D_B;

        let R_a = 400. * R_c.signum()
            * (ill.F_L * R_c.abs() / 100.).powf(0.42)
            / ((ill.F_L * R_c.abs() / 100.).powf(0.42) + 27.13)
            + 0.1;
        let G_a = 400. * G_c.signum()
            * (ill.F_L * G_c.abs() / 100.).powf(0.42)
            / ((ill.F_L * G_c.abs() / 100.).powf(0.42) + 27.13)
            + 0.1;
        let B_a = 400. * B_c.signum()
            * (ill.F_L * B_c.abs() / 100.).powf(0.42)
            / ((ill.F_L * B_c.abs() / 100.).powf(0.42) + 27.13)
            + 0.1;

        let a = R_a - 12. * G_a / 11. + B_a / 11.;
        let b = (R_a + G_a - 2. * B_a) / 9.;

        let h = (f32::atan2(b, a) / (2. * PI)).cyclic_clip(1.) * 360.;
        let hh = h + if h < 20.14 { 360. } else { 0. };

        let e_t = 0.25 * (f32::cos(hh / 180. * PI + 2.) + 3.8);
        let A = ill.N_bb * (2. * R_a + G_a + 0.05 * B_a - 0.305);
        let J = 100. * (A / ill.A_w).powf(ill.Sc * ill.z);
        let t = (50000./13. * ill.SN_c * ill.N_cb * e_t * f32::hypot(a, b))
            / (R_a + G_a + 21./20. * B_a);
        let C = t.powf(0.9) * (J/100.).sqrt() * (1.64 - 0.29f32.powf(ill.n)).powf(0.73);
        let M = C * ill.F_L.powf(0.25);
        let JJ = J * 1.7 / (1. + 0.007 * J);
        let MM = f32::ln(1. + 0.0228 * M) / 0.0228;
        let aa = MM * f32::cos(h / 360. * 2. * PI);
        let bb = MM * f32::sin(h / 360. * 2. * PI);
        Self {
            J: JJ,
            a: aa,
            b: bb,
            C
        }
    }
    pub fn complementary(&self) -> Self {
        Self {
            J: self.J,
            a: -self.a,
            b: -self.b,
            C: self.C // not quite correct, but who cares
        }
    }
    pub fn dist_limatch(self, other: Self, limatch: f32) -> f32 {
        (1. - limatch) * Self::dist(&self, &other) + limatch * f32::abs(self.J - other.J)
    }
    pub fn chr50(self) -> Self {
        Self {
            J: self.J,
            a: self.a / 2.,
            b: self.b / 2.,
            C: self.C / 2.
        }
    }
    pub fn li50(self) -> Self {
        Self {
            J: self.J / 2.,
            a: self.a,
            b: self.b,
            C: self.C
        }
    }
    pub fn mix(one: Self, another: Self, a: f32) -> Self {
        Self {
            J: f32::interpolate(one.J, another.J, a),
            a: f32::interpolate(one.a, another.a, a),
            b: f32::interpolate(one.b, another.b, a),
            C: f32::interpolate(one.C, another.C, a)
        }
    }
}
