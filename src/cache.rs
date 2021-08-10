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
    cache: HashMap<String, PlotData<CAM16UCS>>
}
impl PlotCacher {
    pub fn new() -> Self {
        Self { cache: HashMap::new() }
    }
    pub fn get<F: Fn() -> PlotData<CAM16UCS>>(&mut self, key: &str, f: F) -> &PlotData<CAM16UCS> {
        let s: String = key.into();
        if !self.cache.contains_key(&s) {
            let data = f();
            self.cache.insert(s.clone(), data);
        }
        return &self.cache[&s];
    }
    // TODO: save/load once daemon mode is ready
}
