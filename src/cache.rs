use serde::{Serialize, Deserialize};
use bincode;
use directories::ProjectDirs;
use crossbeam_channel::{Receiver, Sender};

use crate::util::{Clip, CyclicClip, PackedF32, Lerp};
use crate::colour::*;

use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize)]
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

#[derive(Clone, Serialize, Deserialize)]
pub struct BigCacher {
    version: u64,
    plots: HashMap<(PackedF32, String), PlotData<CAM16UCS>>,
    spectra: HashMap<(PackedF32, PackedF32), Vec<CAM16UCS>>,
    cam16_boundaries: HashMap<PackedF32, Vec<f32>>
}
impl BigCacher {
    pub const VERSION: u64 = 2;
    pub fn new() -> Self {
        Self {
            plots: HashMap::new(),
            spectra: HashMap::new(),
            cam16_boundaries: HashMap::new(),
            version: Self::VERSION
        }
    }
    pub fn get_plot(&self, T: f32, key: &str) -> Option<&PlotData<CAM16UCS>> {
        let k = (PackedF32(T), String::from(key));
        return self.plots.get(&k);
    }
    pub fn set_plot(&mut self, T: f32, key: &str, p: PlotData<CAM16UCS>) {
        let k = (PackedF32(T), String::from(key));
        self.plots.insert(k, p);
    }
    pub fn get_spectrum(&self, T: f32, ratio: f32) -> Option<&Vec<CAM16UCS>> {
        let k = (PackedF32(T), PackedF32(ratio));
        return self.spectra.get(&k);
    }
    pub fn set_spectrum(&mut self, T: f32, ratio: f32, spectrum: Vec<CAM16UCS>) {
        let k = (PackedF32(T), PackedF32(ratio));
        self.spectra.insert(k, spectrum);
    }
    pub fn get_cam16_boundary(&self, T: f32) -> Option<&Vec<f32>> {
        let k = PackedF32(T);
        return self.cam16_boundaries.get(&k);
    }
    pub fn set_cam16_boundary(&mut self, T: f32, boundary: Vec<f32>) {
        let k = PackedF32(T);
        self.cam16_boundaries.insert(k, boundary);
    }
    pub fn compute_cam16_boundary(ill: &CAT16Illuminant) -> Vec<f32> {
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
    pub fn compute_spectrum(ill: &CAT16Illuminant, ratio: f32) -> Vec<CAM16UCS> {
        let n = 800;
        let mut data = vec![];
        let min = CAM16UCS::of(CIEXYZ::from(Wavelength::new(Wavelength::MIN as f32)), ill);
        let max = CAM16UCS::of(CIEXYZ::from(Wavelength::new(Wavelength::MAX as f32)), ill);
        for i in 0..n {
            let mut x = i as f32 / (n - 1) as f32;
            if x <= ratio {
                x /= ratio;
                let wl = f32::interpolate(Wavelength::MIN as f32, Wavelength::MAX as f32, x);
                let xyz = CIEXYZ::from(Wavelength::new(wl));
                let cam16 = CAM16UCS::of(xyz, ill);
                data.push(cam16);
            } else {
                x = (x - ratio) / (1. - ratio);
                let cam16 = CAM16UCS::mix(max, min, x);
                data.push(cam16);
            }
        }
        return data;
    }
    pub fn save(&self) -> std::io::Result<()> {
        use std::io::{Error, ErrorKind};
        let dirs = ProjectDirs::from("app", "Quickmarble", "censor")
            .ok_or(
                Error::new(ErrorKind::Other, "couldn't choose app cache directory")
            )?;
        let cache_path = dirs.cache_dir();
        std::fs::create_dir_all(cache_path)?;
        let cache_file = cache_path.join("cache.bin");
        let encoded = bincode::serialize(self)
            .map_err(
                |_| Error::new(ErrorKind::Other, "couldn't encode cache")
            )?;
        std::fs::write(cache_file, encoded)
    }
    pub fn load() -> std::io::Result<Self> {
        use std::io::{Error, ErrorKind};
        let dirs = ProjectDirs::from("app", "Quickmarble", "censor")
            .ok_or(
                Error::new(ErrorKind::Other, "couldn't choose app cache directory")
            )?;
        let cache_path = dirs.cache_dir();
        let cache_file = cache_path.join("cache.bin");
        let encoded = std::fs::read(cache_file)?;
        let decoded: Self = bincode::deserialize(encoded.as_slice())
            .map_err(
                |_| Error::new(ErrorKind::Other, "couldn't decode cache")
            )?;
        if decoded.version == Self::VERSION {
            return Ok(decoded);
        } else {
            return Ok(Self::new());
        }
    }
    pub fn init(verbose: bool) -> Self {
        match Self::load() {
            Ok(x) => { x }
            Err(e) => {
                if verbose {
                    eprintln!("Cache loading failed: {}", e);
                }
                Self::new()
            }
        }
    }
}

/// Cache provider runs in the same thread as widgets.
/// It communicates with the actual cache on other (or not) thread when it must.
/// It stores illuminant data already.
pub trait CacheProvider {
    fn get_plot<F: Fn() -> PlotData<CAM16UCS>>(&mut self, key: &str, f: F) -> PlotData<CAM16UCS>;
    fn get_cam16_boundary(&mut self) -> Vec<f32>;
    fn get_spectrum(&mut self, ratio: f32) -> Vec<CAM16UCS>;
    fn uncached(&self) -> NoCacheProvider;
}

#[derive(Clone)]
pub struct NoCacheProvider {
    T: f32,
    ill: CAT16Illuminant
}
impl<'a> NoCacheProvider {
    pub fn new(T: f32, ill: CAT16Illuminant) -> Self {
        Self { T, ill }
    }
}
impl CacheProvider for NoCacheProvider {
    fn get_plot<F: Fn() -> PlotData<CAM16UCS>>(&mut self, _key: &str, f: F) -> PlotData<CAM16UCS> {
        return f();
    }
    fn get_cam16_boundary(&mut self) -> Vec<f32> {
        return BigCacher::compute_cam16_boundary(&self.ill);
    }
    fn get_spectrum(&mut self, ratio: f32) -> Vec<CAM16UCS> {
        return BigCacher::compute_spectrum(&self.ill, ratio);
    }
    fn uncached(&self) -> NoCacheProvider {
        self.clone()
    }
}

pub struct SinglethreadedCacheProvider<'a> {
    T: f32,
    ill: &'a CAT16Illuminant,
    cacher: &'a mut BigCacher
}
impl<'a> SinglethreadedCacheProvider<'a> {
    pub fn new(T: f32, ill: &'a CAT16Illuminant, cacher: &'a mut BigCacher) -> Self {
        Self { T, ill, cacher }
    }
}
impl<'a> CacheProvider for SinglethreadedCacheProvider<'a> {
    fn get_plot<F: Fn() -> PlotData<CAM16UCS>>(&mut self, key: &str, f: F) -> PlotData<CAM16UCS> {
        match self.cacher.get_plot(self.T, key) {
            Some(data) => { data.clone() }
            None => {
                let data = f();
                self.cacher.set_plot(self.T, key, data.clone());
                return data;
            }
        }
    }
    fn get_cam16_boundary(&mut self) -> Vec<f32> {
        match self.cacher.get_cam16_boundary(self.T) {
            Some(data) => { data.clone() }
            None => {
                let data = BigCacher::compute_cam16_boundary(self.ill);
                self.cacher.set_cam16_boundary(self.T, data.clone());
                return data;
            }
        }
    }
    fn get_spectrum(&mut self, ratio: f32) -> Vec<CAM16UCS> {
        match self.cacher.get_spectrum(self.T, ratio) {
            Some(data) => { data.clone() }
            None => {
                let data = BigCacher::compute_spectrum(self.ill, ratio);
                self.cacher.set_spectrum(self.T, ratio, data.clone());
                return data;
            }
        }
    }
    fn uncached(&self) -> NoCacheProvider {
        NoCacheProvider::new(self.T, self.ill.clone())
    }
}

pub enum CacheRequest {
    PlotState { T: f32, key: String },
    PlotWrite { T: f32, key: String, data: PlotData<CAM16UCS> },
    CAM16BoundaryState { T: f32 },
    CAM16BoundaryWrite { T: f32, data: Vec<f32> },
    SpectrumState { T: f32, ratio: f32 },
    SpectrumWrite { T: f32, ratio: f32, data: Vec<CAM16UCS> }
}
unsafe impl Send for CacheRequest {}

pub enum CacheResponse {
    Plot(Option<PlotData<CAM16UCS>>),
    CAM16Boundary(Option<Vec<f32>>),
    Spectrum(Option<Vec<CAM16UCS>>)
}
unsafe impl Send for CacheResponse {}

pub struct MultithreadedCacheProvider {
    T: f32,
    ill: CAT16Illuminant,
    sender: Sender<CacheRequest>,
    receiver: Receiver<CacheResponse>
}
impl MultithreadedCacheProvider {
    pub fn new(T: f32, ill: CAT16Illuminant,
               sender: Sender<CacheRequest>,
               receiver: Receiver<CacheResponse>) -> Self {
        Self { T, ill, sender, receiver }
    }
}
impl CacheProvider for MultithreadedCacheProvider {
    fn get_plot<F: Fn() -> PlotData<CAM16UCS>>(&mut self, key: &str, f: F) -> PlotData<CAM16UCS> {
        self.sender.send(CacheRequest::PlotState {
            T: self.T,
            key: String::from(key)
        }).unwrap();
        match self.receiver.recv() {
            Ok(CacheResponse::Plot(Some(data))) => { data }
            Ok(CacheResponse::Plot(None)) => {
                let data = f();
                self.sender.send(CacheRequest::PlotWrite {
                    T: self.T,
                    key: String::from(key),
                    data: data.clone()
                }).unwrap();
                return data;
            }
            Ok(_) => { panic!("I never asked for this") }
            Err(_) => { panic!("The cache is dead!") }
        }
    }
    fn get_cam16_boundary(&mut self) -> Vec<f32> {
        self.sender.send(CacheRequest::CAM16BoundaryState { T: self.T }).unwrap();
        match self.receiver.recv() {
            Ok(CacheResponse::CAM16Boundary(Some(data))) => { data }
            Ok(CacheResponse::CAM16Boundary(None)) => {
                let data = BigCacher::compute_cam16_boundary(&self.ill);
                self.sender.send(CacheRequest::CAM16BoundaryWrite {
                    T: self.T,
                    data: data.clone()
                }).unwrap();
                return data;
            }
            Ok(_) => { panic!("I never asked for this") }
            Err(_) => { panic!("The cache is dead!") }
        }
    }
    fn get_spectrum(&mut self, ratio: f32) -> Vec<CAM16UCS> {
        self.sender.send(CacheRequest::SpectrumState { T: self.T, ratio }).unwrap();
        match self.receiver.recv() {
            Ok(CacheResponse::Spectrum(Some(data))) => { data }
            Ok(CacheResponse::Spectrum(None)) => {
                let data = BigCacher::compute_spectrum(&self.ill, ratio);
                self.sender.send(CacheRequest::SpectrumWrite {
                    T: self.T,
                    ratio,
                    data: data.clone()
                }).unwrap();
                return data;
            }
            Ok(_) => { panic!("I never asked for this") }
            Err(_) => { panic!("The cache is dead!") }
        }
    }
    fn uncached(&self) -> NoCacheProvider {
        NoCacheProvider::new(self.T, self.ill.clone())
    }
}

pub struct CacheHoster<'a> {
    connections: Vec<(Receiver<CacheRequest>, Sender<CacheResponse>)>,
    cacher: &'a mut BigCacher
}
impl<'a> CacheHoster<'a> {
    pub fn new(cacher: &'a mut BigCacher) -> Self {
        Self { cacher, connections: vec![] }
    }
    pub fn register(&mut self) -> (Receiver<CacheResponse>, Sender<CacheRequest>) {
        let (req_send, req_recv) = crossbeam_channel::bounded(0);
        let (resp_send, resp_recv) = crossbeam_channel::bounded(0);
        self.connections.push((req_recv, resp_send));
        return (resp_recv, req_send);
    }
    pub fn process(&mut self) {
        while self.connections.len() > 0 {
            let mut select = crossbeam_channel::Select::new();
            for (recv, _) in self.connections.iter() {
                select.recv(recv);
            }
            let op = select.select();
            let i = op.index();
            match op.recv(&self.connections[i].0) {
                Ok(CacheRequest::PlotState { T, key }) => {
                    self.connections[i].1.send(
                        CacheResponse::Plot(self.cacher.get_plot(T, &key).cloned())
                    ).unwrap();
                }
                Ok(CacheRequest::PlotWrite { T, key, data }) => {
                    self.cacher.set_plot(T, &key, data);
                }
                Ok(CacheRequest::CAM16BoundaryState { T }) => {
                    self.connections[i].1.send(
                        CacheResponse::CAM16Boundary(self.cacher.get_cam16_boundary(T).cloned())
                    ).unwrap();
                }
                Ok(CacheRequest::CAM16BoundaryWrite { T, data }) => {
                    self.cacher.set_cam16_boundary(T, data);
                }
                Ok(CacheRequest::SpectrumState { T, ratio }) => {
                    self.connections[i].1.send(
                        CacheResponse::Spectrum(self.cacher.get_spectrum(T, ratio).cloned())
                    ).unwrap();
                }
                Ok(CacheRequest::SpectrumWrite { T, ratio, data }) => {
                    self.cacher.set_spectrum(T, ratio, data);
                }
                Err(_) => {
                    self.connections.remove(i);
                }
            }
        }
    }
}
