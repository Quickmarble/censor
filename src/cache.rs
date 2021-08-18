use crate::util::{Clip, CyclicClip};
use crate::colour::*;

use std::collections::HashMap;

#[derive(Clone)]
pub struct PlotData<T: Copy> {
    pub data: Vec<Vec<Option<T>>>
}
#[allow(dead_code)]
impl<T: Copy> PlotData<T> {
    pub fn new(data: Vec<Vec<Option<T>>>) -> Self {
        Self { data }
    }
    pub fn empty(w: usize, h: usize) -> Self {
        Self {
            data: vec![vec![None; w]; h]
        }
    }
}

#[derive(Clone)]
pub struct PlotCacher {
    cache: HashMap<String, PlotData<CAM16UCS>>,
    cam16_boundary: Option<Vec<f32>>
}
impl PlotCacher {
    pub fn new() -> Self {
        Self { cache: HashMap::new(), cam16_boundary: None }
    }
    pub fn get<F: Fn() -> PlotData<CAM16UCS>>(&mut self, key: &str, f: F) -> &PlotData<CAM16UCS> {
        let s: String = key.into();
        if !self.cache.contains_key(&s) {
            let data = f();
            self.cache.insert(s.clone(), data);
        }
        return &self.cache[&s];
    }
    pub fn get_cam16_boundary(&mut self, ill: &CAT16Illuminant) -> &Vec<f32> {
        if let None = self.cam16_boundary {
            self.cam16_boundary = Some(Self::compute_cam16_boundary(ill));
        }
        self.cam16_boundary.as_ref().unwrap()
    }
    fn compute_cam16_boundary(ill: &CAT16Illuminant) -> Vec<f32> {
        use std::f32::consts::PI;
        let n = 400;
        let mut boundary = vec![0.; n];

        fn nearest_angle(n: usize, a: f32) -> usize {
            ((a * n as f32).round() as usize).clip(0, n) % n
        }
        fn consider(boundary: &mut Vec<f32>, ill: &CAT16Illuminant, r: u8, g: u8, b: u8) {
            use crate::colour::*;
            let n = boundary.len();
            let rgb = RGB255::new(r, g, b);
            let xyz = CIEXYZ::from(rgb);
            let cam16 = CAM16UCS::of(xyz, ill);
            let a = (f32::atan2(cam16.b, cam16.a) / (2. * PI)).cyclic_clip(1.);
            let C = cam16.C / 100.;
            let i = nearest_angle(n, a);
            boundary[i] = f32::max(C, boundary[i]);
        }

        // Iterating faces of the RGB cube should be enough
        for i in 0..=255 {
            for j in 0..=255 {
                consider(&mut boundary, ill, 0, i, j);
                consider(&mut boundary, ill, i, 0, j);
                consider(&mut boundary, ill, i, j, 0);
                consider(&mut boundary, ill, 255, i, j);
                consider(&mut boundary, ill, i, 255, j);
                consider(&mut boundary, ill, i, j, 255);
            }
        }

        return boundary;
    }
    // TODO: save/load once daemon mode is ready
}
