use rand::seq::SliceRandom;

use crate::colour::*;
use crate::palette::*;
use crate::util::{CyclicClip, PackedF32};
use crate::cache::*;

pub trait ThresholdStructure {
    /// Output shall be in [0; 1]
    fn at(&self, x: usize, y: usize) -> f32;
}

pub struct ThresholdMatrix {
    w: usize,
    h: usize,
    order: Vec<Vec<usize>>
}
#[allow(dead_code)]
impl ThresholdMatrix {
    pub fn new(w: usize, h: usize, order: Vec<Vec<usize>>) -> Self {
        Self { w, h, order }
    }
    pub fn binary(&self) -> Vec<Vec<bool>> {
        let mut data = vec![vec![false; self.w]; self.h];
        for i in 0..self.w {
            for j in 0..self.h {
                data[j][i] = self.at(i, j) > 0.5;
            }
        }
        return data;
    }
    pub fn bayer(n: u32) -> Self {
        let mut d = 1;
        let mut order = vec![vec![0]];
        for _ in 0..n {
            let new_d = d * 2;
            let mut new_order = vec![vec![0; new_d]; new_d];
            for i in 0..d {
                for j in 0..d {
                    new_order[j][i] = 4 * order[j][i];
                    new_order[j][d + i] = 4 * order[j][i] + 2;
                    new_order[d + j][i] = 4 * order[j][i] + 3;
                    new_order[d + j][d + i] = 4 * order[j][i] + 1;
                }
            }
            d = new_d;
            order = new_order;
        }
        Self {
            w: d, h: d,
            order
        }
    }
    pub fn whitenoise(w: usize, h: usize) -> Self {
        fn random_permutation(n: usize) -> Vec<usize> {
            let mut data = vec![0; n];
            for i in 0..n {
                data[i] = i;
            }
            let mut rng = rand::thread_rng();
            data.shuffle(&mut rng);
            return data;
        }
        let perm = random_permutation(w * h);
        let mut order = vec![vec![0; w]; h];
        for i in 0..w {
            for j in 0..h {
                order[j][i] = perm[j * w + i];
            }
        }
        Self { w, h, order }
    }
    fn cluster<T: PartialEq>(data: &Vec<Vec<T>>, w: usize, h: usize, val: T) -> Vec<Vec<f32>> {
        let s = 1.5;
        let mut cluster = vec![vec![0.; w]; h];
        let radius = i32::min(w as i32, h as i32) / 2;
        for x in 0..w as i32 {
            for y in 0..h as i32 {
                if data[y as usize][x as usize] == val {
                    for xi in x-radius..=x+radius {
                        let xx = xi.cyclic_clip(w as i32);
                        for yi in y-radius..=y+radius {
                            let yy = yi.cyclic_clip(h as i32);
                            if data[yy as usize][xx as usize] == val {
                                let xmin = i32::min(x, xx);
                                let xmax = i32::max(x, xx);
                                let ymin = i32::min(y, yy);
                                let ymax = i32::max(y, yy);
                                let dx = i32::min(xmax - xmin, w as i32 + xmin - xmax) as f32;
                                let dy = i32::min(ymax - ymin, h as i32 + ymin - ymax) as f32;
                                let dr = f32::hypot(dx, dy);
                                let t = dr / s;
                                cluster[y as usize][x as usize] += f32::exp(-t.powi(2) / 2.);
                            }
                        }
                    }
                }
            }
        }
        return cluster;
    }
    fn argmax(data: &Vec<Vec<f32>>, w: usize, h: usize) -> (usize, usize) {
        let (mut x, mut y) = (0, 0);
        let mut v_max = data[0][0];
        for i in 0..w {
            for j in 0..h {
                let v = data[j][i];
                if v > v_max {
                    x = i;
                    y = j;
                    v_max = v;
                }
            }
        }
        return (x, y);
    }
    pub fn bluenoise(w: usize, h: usize) -> Self {
        // generate the initial binary pattern
        let mut initial = Self::whitenoise(w, h).binary();
        loop {
            let cluster = Self::cluster(&initial, w, h, true);
            let (x1, y1) = Self::argmax(&cluster, w, h);
            initial[y1][x1] = false;
            let void = Self::cluster(&initial, w, h, false);
            let (x0, y0) = Self::argmax(&void, w, h);
            if (x0, y0) == (x1, y1) {
                initial[y1][x1] = true;
                break;
            }
            initial[y0][x0] = true;
        }
        let mut order = vec![vec![0; w]; h];
        let ones: usize = initial.iter().map(
            |row| row.iter().map(|&b| b as usize).sum::<usize>()
        ).sum();
        let mut state = initial.clone();
        for rank in (0..ones).rev() {
            let cluster = Self::cluster(&state, w, h, true);
            let (x1, y1) = Self::argmax(&cluster, w, h);
            state[y1][x1] = false;
            order[y1][x1] = rank;
        }
        state = initial;
        for rank in ones..w*h {
            let void = Self::cluster(&state, w, h, false);
            let (x0, y0) = Self::argmax(&void, w, h);
            state[y0][x0] = true;
            order[y0][x0] = rank;
        }
        Self { w, h, order }
    }
}
impl ThresholdStructure for ThresholdMatrix {
    fn at(&self, x: usize, y: usize) -> f32 {
        let max = self.w * self.h - 1;
        if max == 0 { return 0.; }
        let i = x.cyclic_clip(self.w);
        let j = y.cyclic_clip(self.h);
        return self.order[j][i] as f32 / max as f32;
    }
}

pub struct OrderedDither {}
impl OrderedDither {
    pub fn dither<T: ThresholdStructure, P: AsRef<Palette>>
            (input: PlotData<CAM16UCS>, palette: P, threshold: &T) -> PlotData<RGB255> {
        let J_min: f32 = palette.as_ref().cam16.iter().map(|c| PackedF32(c.J)).min().unwrap().0;
        let J_max: f32 = palette.as_ref().cam16.iter().map(|c| PackedF32(c.J)).max().unwrap().0;
        let J_spread = (J_max - J_min) / palette.as_ref().n as f32;
        let h = input.data.len();
        let w = input.data[0].len();
        let mut output = vec![vec![None; w]; h];
        for i in 0..w {
            for j in 0..h {
                let mut c = match input.data[j][i] {
                    Some(x) => { x }
                    None => { continue; }
                };
                c.J += J_spread * (threshold.at(i, j) - 0.5);
                output[j][i] = Some(palette.as_ref().nearest(c));
            }
        }
        return PlotData::new(output);
    }
}

#[derive(Clone, Copy)]
pub enum DitheringMethod {
    None,
    Bayer(u32),
    WhiteNoise(usize, usize),
    BlueNoise(usize, usize)
}
impl Default for DitheringMethod {
    fn default() -> Self {
        Self::BlueNoise(14, 14)
    }
}

// TODO: cache
pub struct Ditherer {}
impl Ditherer {
    pub fn dither<P: AsRef<Palette>>
            (input: PlotData<CAM16UCS>, palette: P, method: DitheringMethod, verbose: bool)
                -> PlotData<RGB255> {
        match method {
            DitheringMethod::None => {
                let matrix = ThresholdMatrix::bayer(0);
                if verbose { eprintln!("Dithering in progress...") }
                OrderedDither::dither(input, palette, &matrix)
            }
            DitheringMethod::Bayer(n) => {
                if verbose {
                    eprintln!("Creating threshold matrix (Bayer, {}x{})", 2u32.pow(n), 2u32.pow(n))
                }
                let matrix = ThresholdMatrix::bayer(n);
                if verbose { eprintln!("Dithering in progress...") }
                OrderedDither::dither(input, palette, &matrix)
            }
            DitheringMethod::WhiteNoise(w, h) => {
                if verbose { eprintln!("Creating threshold matrix (White noise, {}x{})", w, h) }
                let matrix = ThresholdMatrix::whitenoise(w, h);
                if verbose { eprintln!("Dithering in progress...") }
                OrderedDither::dither(input, palette, &matrix)
            }
            DitheringMethod::BlueNoise(w, h) => {
                if verbose { eprintln!("Creating threshold matrix (Blue noise, {}x{})", w, h) }
                let matrix = ThresholdMatrix::bluenoise(w, h);
                if verbose { eprintln!("Dithering in progress...") }
                OrderedDither::dither(input, palette, &matrix)
            }
        }
    }
}
