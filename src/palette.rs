use crate::util::{Clip, PackedF32};
use crate::colour::*;

use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct Palette {
    pub n: usize,
    pub rgb: Vec<RGB255>,
    pub xyz: Vec<CIEXYZ>,
    pub cam16: Vec<CAM16UCS>,
    pub sorted: Vec<usize>,
    pub bl: usize,
    pub bg: usize,
    pub fg: usize,
    pub tl: usize,
    pub bl_rgb: RGB255,
    pub bg_rgb: RGB255,
    pub fg_rgb: RGB255,
    pub tl_rgb: RGB255
}
impl Palette {
    pub fn new(rgb: Vec<RGB255>, ill: &CAT16Illuminant, grey_ui: bool) -> Self {
        let n = rgb.len();
        let xyz: Vec<CIEXYZ> = rgb.iter()
            .map(|&RGB| CIEXYZ::from(RGB))
            .collect();
        let cam16: Vec<CAM16UCS> = xyz.iter()
            .map(|&XYZ| CAM16UCS::of(XYZ, ill))
            .collect();
        let mut sorted: Vec<(usize, CAM16UCS)> = cam16.iter()
            .zip(0..)
            .map(|(&c, i)| (i, c))
            .collect();
        sorted.sort_by_key(|(_, c)| PackedF32(c.J));
        let sorted = sorted.iter().map(|&(i, _)| i).collect();
        let bl = Self::minimise(&cam16, |_, c| {
            CAM16UCS::dist(&c, &CAM16UCS{J:0., a:0., b:0., C:0.})
        });
        let bg = Self::minimise(&cam16, |i, c| {
            if i == bl { return f32::MAX; }
            let not_grey = 100. - CAM16UCS::dist(&c, &CAM16UCS{J:50., a:0., b:0., C:0.});
            let not_bl = CAM16UCS::dist_limatch(c, cam16[bl], 0.6);
            let score = not_bl.powf(0.02) * not_grey.powf(0.98);
            return -score;
        });
        let fg = Self::minimise(&cam16, |i, c| {
            if i == bl { return f32::MAX; }
            -CAM16UCS::dist(&c, &cam16[bl])
        });
        let tl = Self::minimise(&cam16, |i, c| {
            if i == bg { return f32::MAX; }
            -CAM16UCS::dist_limatch(c, cam16[bg], 0.6)
        });
        let bl_rgb = if grey_ui { RGB255::new(0, 0, 0) } else { rgb[bl] };
        let bg_rgb = if grey_ui { RGB255::new(127, 127, 127) } else { rgb[bg] };
        let fg_rgb = if grey_ui { RGB255::new(255, 255, 255) } else { rgb[fg] };
        let tl_rgb = if grey_ui { RGB255::new(255, 255, 255) } else { rgb[tl] };
        Palette { n, rgb, xyz, cam16, sorted, bl, bg, fg, tl, bl_rgb, bg_rgb, fg_rgb, tl_rgb }
    }
    fn minimise<F: Fn(usize, CAM16UCS) -> f32>(cam16: &Vec<CAM16UCS>, score: F) -> usize {
        let mut min = f32::MAX;
        let mut argmin = 0;
        for i in 0..cam16.len() {
            let d = score(i, cam16[i]);
            if d < min {
                argmin = i;
                min = d;
            }
        }
        return argmin;
    }
    pub fn nearest(&self, x: CAM16UCS) -> RGB255 {
        let mut min = f32::MAX;
        let mut argmin = 0;
        for i in 0..self.n {
            let y = self.cam16[i];
            let d = CAM16UCS::dist(&x, &y);
            if d < min {
                argmin = i;
                min = d;
            }
        }
        return self.rgb[argmin];
    }
    pub fn nearest_limatch(&self, x: CAM16UCS, t: f32) -> RGB255 {
        let mut min = f32::MAX;
        let mut argmin = 0;
        for i in 0..self.n {
            let y = self.cam16[i];
            let d = CAM16UCS::dist(&x, &y) * (1. - t) + f32::abs(x.J - y.J) * t;
            if d < min {
                argmin = i;
                min = d;
            }
        }
        return self.rgb[argmin];
    }
    pub fn neutraliser(&self, x: CAM16UCS) -> usize {
        let z = x.complementary();
        return Self::minimise(&self.cam16, |_, c| CAM16UCS::dist_limatch(z, c, 0.1));
    }
    pub fn spectral_stats(&self, ill: &CAT16Illuminant)
                -> (HashMap<PackedF32, f32>, HashMap<usize, f32>) {
        let mut stats = HashMap::new();
        let mut points = HashMap::new();
        let o = CIExy::from(CIEXYZ::new(ill.X_w, ill.Y_w, ill.Z_w));
        for i in 0..self.n {
            let xy = CIExy::from(self.xyz[i]);
            match xy.try_nearest_spectral(o) {
                Some(Wavelength{wl}) => {
                    let k = PackedF32(wl);
                    if !stats.contains_key(&k) {
                        stats.insert(k, 0.);
                    }
                    let weight = (self.cam16[i].C / 100.).clip(0., 1.);
                    stats.insert(k, stats[&k] + weight);
                    points.insert(i, wl);
                }
                None => {}
            }
        }
        let norm: f32 = stats.values().sum();
        if norm > 0. {
            for (_, v) in stats.iter_mut() {
                *v /= norm;
            }
        }
        return (stats, points);
    }
    pub fn CCT_stats(&self) -> (HashMap<PackedF32, f32>, HashMap<usize, f32>) {
        let mut stats = HashMap::new();
        let mut points = HashMap::new();
        for i in 0..self.n {
            match CIEuv::from(self.xyz[i]).CCT() {
                Some((T, dist)) => {
                    let k = PackedF32(T);
                    if !stats.contains_key(&k) {
                        stats.insert(k, 0.);
                    }
                    let weight = 1. - dist * 20.;
                    stats.insert(k, stats[&k] + weight);
                    points.insert(i, T);
                }
                None => {}
            }
        }
        let norm: f32 = stats.values().sum();
        if norm > 0. {
            for (_, v) in stats.iter_mut() {
                *v /= norm;
            }
        }
        return (stats, points);
    }
    pub fn useful_mixes(&self, max: usize) -> Vec<(usize, usize)> {
        fn score_added(cam16: &Vec<CAM16UCS>, x: CAM16UCS) -> f32 {
            let mut min = f32::MAX;
            for y in cam16.iter() {
                let d = CAM16UCS::dist(&x, &y);
                if d < min {
                    min = d;
                }
            }
            min
        }
        fn mix(cam16: &Vec<CAM16UCS>, i: usize, j: usize) -> CAM16UCS {
            let x = cam16[i];
            let y = cam16[j];
            CAM16UCS {
                J: (x.J + y.J) / 2.,
                a: (x.a + y.a) / 2.,
                b: (x.b + y.b) / 2.,
                C: (x.C + y.C) / 2.
            }
        }
        let max = usize::min(max, self.n * (self.n - 1) / 2);
        let mut mixes: Vec<(usize, usize)> = vec![];
        let mut pairs = vec![];
        for i in 0..self.n-1 {
            for j in i+1..self.n {
                pairs.push((i, j));
            }
        }
        let mut scores: Vec<(f32, (usize, usize))> = pairs.iter()
            .map(|&(i, j)|
                (
                    score_added(&self.cam16, mix(&self.cam16, i, j)),
                    (i, j)
                )
            )
            .collect();
        while mixes.len() < max {
            scores.sort_by_key(|&(d, _)| PackedF32(d));
            let best_pair: (usize, usize) = scores.last().unwrap().1;
            mixes.push(best_pair);

            let mixed = mix(&self.cam16, best_pair.0, best_pair.1);
            scores.retain(|&(_, pair)| pair != best_pair);
            for i in 0..scores.len() {
                let pair = scores[i].1;
                scores[i].0 += CAM16UCS::dist(&mix(&self.cam16, pair.0, pair.1), &mixed);
            }
        }
        return mixes;
    }
    pub fn is_acyclic(&self) -> bool {
        let mut pairs: Vec<((usize, usize), f32)> = vec![];
        for i in 0..self.n-1 {
            for j in i+1..self.n {
                let d = CAM16UCS::dist(&self.cam16[i], &self.cam16[j]);
                pairs.push(((i, j), d));
            }
        }
        pairs.sort_by_key(|&(_, d)| PackedF32(d));
        let mut connected: HashSet<(usize, usize)> = HashSet::new();
        for i in 0..self.n {
            connected.insert((i, i));
        }
        let mut clusters: Vec<HashSet<usize>> = vec![];
        for i in 0..self.n {
            clusters.push(HashSet::new());
            clusters[i].insert(i);
        }
        fn find_k(clusters: &Vec<HashSet<usize>>, i: usize) -> usize {
            for k in 0..clusters.len() {
                if clusters[k].contains(&i) {
                    return k;
                }
            }
            return usize::MAX;
        }
        for ((i, j), _) in pairs {
            let ki = find_k(&clusters, i);
            let kj = find_k(&clusters, j);
            if ki != kj {
                // Join clusters
                let c: HashSet<usize> = HashSet::union(&clusters[ki], &clusters[kj])
                    .copied()
                    .collect();
                clusters[ki] = c;
                clusters.swap_remove(kj);
            } else {
                let mut cycle = true;
                for k in 0..self.n {
                    if connected.contains(&(i, k)) && connected.contains(&(k, j)) {
                        cycle = false;
                        break;
                    }
                }
                if cycle {
                    return false;
                }
            }
            connected.insert((i, j));
            connected.insert((j, i));
        }
        return true;
    }
    pub fn internal_similarity(&self) -> f32 {
        let mut min = f32::MAX;
        let mut mean = 0.;
        let pair_n = self.n * (self.n - 1) / 2;
        for i in 0..self.n {
            for j in i+1..self.n {
                let d = CAM16UCS::dist(&self.cam16[i], &self.cam16[j]);
                mean += d / pair_n as f32;
                if d < min {
                    min = d;
                }
            }
        }
        if min > 0. {
            let score = mean / min;
            return score / (self.n as f32).powf(2./3.);
        } else {
            return f32::NAN;
        }
    }
}
