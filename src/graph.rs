#[cfg(target_arch = "wasm32")]
use hex;
use image::{RgbImage, Rgb};
use img_parts::{png::Png, ImageICC};
use crossbeam_channel::{Receiver, Sender};

use crate::text::*;
use crate::cache::*;
use crate::colour::*;
use crate::palette::Palette;
use crate::util::{abs_diff, CyclicClip};

pub trait GraphPixel: Into<Rgb<u8>>+Copy+std::fmt::Debug {}
impl<T: Into<Rgb<u8>>+Copy+std::fmt::Debug> GraphPixel for T {}

pub trait PixelWriter<T: GraphPixel> {
    fn put_pixel(&mut self, x: i32, y: i32, c: T);
}
impl<T: GraphPixel> PixelWriter<T> for ImageGraph {
    fn put_pixel(&mut self, x: i32, y: i32, c: T) {
        self.put_pixel(x, y, c);
    }
}

#[derive(Clone)]
pub struct ImageGraph {
    buffer: RgbImage,
    icc_profile: Option<img_parts::Bytes>,
    w: u32,
    h: u32
}
impl AsMut<ImageGraph> for ImageGraph {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}
impl ImageGraph {
    pub fn new(w: u32, h: u32) -> Self {
        let buffer = RgbImage::new(w, h);
        Self { buffer, w, h, icc_profile: None }
    }
    pub fn with_icc_profile(self, profile: img_parts::Bytes) -> Self {
        Self {
            buffer: self.buffer,
            icc_profile: Some(profile),
            w: self.w,
            h: self.h
        }
    }
    pub fn put_pixel<T: GraphPixel>(&mut self, x: i32, y: i32, c: T) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as u32, y as u32);
        if x < self.w && y < self.h {
            self.buffer.put_pixel(x, y, c.into());
        }
    }
    pub fn frame<T: GraphPixel>
            (&mut self, x0: i32, y0: i32,
                        w: i32, h: i32,
                        c: T) {
        for x in x0..x0+w {
            self.put_pixel(x, y0, c);
            self.put_pixel(x, y0 + h - 1, c);
        }
        for y in y0..y0+h {
            self.put_pixel(x0, y, c);
            self.put_pixel(x0 + w - 1, y, c);
        }
    }
    pub fn block<T: GraphPixel>
            (&mut self, x0: i32, y0: i32,
                        w: i32, h: i32,
                        c: T) {
        for x in x0..x0+w {
            for y in y0..y0+h {
                self.put_pixel(x, y, c);
            }
        }
    }
    pub fn dither<T: GraphPixel>
            (&mut self, x0: i32, y0: i32,
                        w: i32, h: i32,
                        c1: T, c2: T) {
        let c = [c1, c2];
        for x in x0..x0+w {
            for y in y0..y0+h {
                let k = (x - x0 + y - y0) % 2;
                self.put_pixel(x, y, c[k as usize]);
            }
        }
    }
    pub fn line<T: GraphPixel>
            (&mut self, x0: i32, y0: i32,
                        x1: i32, y1: i32,
                        c: T, dotted: Option<i32>) {
        if x0 == x1 {
            let (y0, y1) = (i32::min(y0, y1), i32::max(y0, y1));
            for i in 0..=y1-y0 {
                match dotted {
                    Some(dot) => {
                        if i % dot == 0 {
                            self.put_pixel(x0, y0 + i, c);
                        }
                    }
                    None => {
                        self.put_pixel(x0, y0 + i, c);
                    }
                }
            }
            return;
        }
        let mut dc: i32 = 0;
        let (mut x0, mut x1) = (x0, x1);
        let (mut y0, mut y1) = (y0, y1);
        if abs_diff(x0, x1) >= abs_diff(y0, y1) {
            if x1 < x0 {
                std::mem::swap(&mut x0, &mut x1);
                std::mem::swap(&mut y0, &mut y1);
            }
            let dx = x1 - x0;
            let dy = y1 - y0;
            for i in 0..=dx {
                let x = x0 + i;
                let y = f32::round(y0 as f32 + i as f32 * dy as f32 / dx as f32) as i32;
                match dotted {
                    Some(dot) => {
                        if dc % dot == 0 {
                            self.put_pixel(x, y, c);
                        }
                    }
                    None => {
                        self.put_pixel(x, y, c);
                    }
                }
                dc += 1;
            }
        } else {
            if y1 < y0 {
                std::mem::swap(&mut x0, &mut x1);
                std::mem::swap(&mut y0, &mut y1);
            }
            let dx = x1 - x0;
            let dy = y1 - y0;
            for i in 0..=dy {
                let y = y0 + i;
                let x = f32::round(x0 as f32 + i as f32 * dx as f32 / dy as f32) as i32;
                match dotted {
                    Some(dot) => {
                        if dc % dot == 0 {
                            self.put_pixel(x, y, c);
                        }
                    }
                    None => {
                        self.put_pixel(x, y, c);
                    }
                }
                dc += 1;
            }
        }
    }
    pub fn circle<T: GraphPixel>
            (&mut self, x0: i32, y0: i32,
                        d: i32,
                        c: T, dotted: Option<i32>) {
        let r = (d as f32 - 1.) / 2.;
        let cx = x0 as f32 + r;
        let cy = y0 as f32 + r;
        let mut y = f32::floor(cy) as i32;
        let mut dc: i32 = 0;
        for x in x0..=x0+d/2 {
            while f32::hypot(x as f32 - cx, y as f32 - cy) <= r {
                let dx = x - x0;
                let dy = y - y0;
                if dx > dy {
                    return;
                }
                match dotted {
                    Some(dot) => {
                        if dc % dot == 0 {
                            self.put_pixel(x, y, c);
                            self.put_pixel(x, y0 + d - 1 - dy, c);
                            self.put_pixel(x0 + d - 1 - dx, y, c);
                            self.put_pixel(x0 + d - 1 - dx, y0 + d - 1 - dy, c);
                            self.put_pixel(y - y0 + x0, x - x0 + y0, c);
                            self.put_pixel(x0 + d - 1 - (y - y0), x - x0 + y0, c);
                            self.put_pixel(y - y0 + x0, y0 + d - 1 - (x - x0), c);
                            self.put_pixel(x0 + d - 1 - (y - y0), y0 + d - 1 - (x - x0), c);
                        }
                    }
                    None => {
                        self.put_pixel(x, y, c);
                        self.put_pixel(x, y0 + d - 1 - dy, c);
                        self.put_pixel(x0 + d - 1 - dx, y, c);
                        self.put_pixel(x0 + d - 1 - dx, y0 + d - 1 - dy, c);
                        self.put_pixel(y - y0 + x0, x - x0 + y0, c);
                        self.put_pixel(x0 + d - 1 - (y - y0), x - x0 + y0, c);
                        self.put_pixel(y - y0 + x0, y0 + d - 1 - (x - x0), c);
                        self.put_pixel(x0 + d - 1 - (y - y0), y0 + d - 1 - (x - x0), c);
                    }
                }
                dc += 1;
                y -= 1;
            }
        }
    }
    pub fn disc<T: GraphPixel>
            (&mut self, x0: i32, y0: i32,
                        d: i32, c: T) {
        if d == 1 {
            self.put_pixel(x0, y0, c);
            return;
        }
        if d == 2 {
            self.block(x0, y0, d, d, c);
            return;
        }
        for i in 0..d {
            let dx = (i as f32 / (d - 1) as f32) * 2. - 1.;
            for j in 0..d {
                let dy = (j as f32 / (d - 1) as f32) * 2. - 1.;
                let r = f32::hypot(dx, dy);
                if r <= 1. {
                    self.put_pixel(x0 + i, y0 + j, c);
                }
            }
        }
    }
    pub fn text<T: GraphPixel>
            (&mut self, s: &str, x0: i32, y0: i32,
                        p: TextAnchor, font: &Font, c: T) {
        let w = font.str_width(s);
        let h = font.str_height(s);
        let (dx, dy) = p.align(w, h);
        font.render_string(self, x0 + dx, y0 + dy, s, c);
    }
    pub fn vtext<T: GraphPixel>
            (&mut self, s: &str, x0: i32, y0: i32,
                        p: HorizontalTextAnchor, font: &Font, c: T) {
        let anchor = TextAnchor { horizontal: p, vertical: VerticalTextAnchor::Top };
        let mut y = y0;
        let chars: Vec<char> = s.chars().collect();
        for i in 0..chars.len() {
            self.text(&format!("{}", chars[i]), x0, y, anchor, font, c);
            y += 1 + font.char_height(chars[i]);
        }
    }
    pub fn plot<F: Fn(f32, f32) -> Option<CAM16UCS>, P: CacheProvider, PR: AsRef<Palette>>
            (&mut self, cacher: &mut P,
             x0: i32, y0: i32, w: i32, h: i32,
             palette: PR, key: &str, f: F) {
        let g = || {
            let mut plot_data = PlotData::<CAM16UCS>::empty(w as usize, h as usize);
            for i in 0..w {
                let x = i as f32 / (w as f32 - 1.);
                for j in 0..h {
                    let y = (h - 1 - j) as f32 / (h as f32 - 1.);
                    plot_data.data[j as usize][i as usize] = f(x, y);
                }
            }
            return plot_data;
        };
        self.plot_data(x0, y0, w, h, palette, cacher.get_plot(key, g));
    }
    pub fn plot_polar<F: Fn(f32, f32) -> Option<CAM16UCS>, P: CacheProvider, PR: AsRef<Palette>>
            (&mut self, cacher: &mut P,
             x0: i32, y0: i32, w: i32, h: i32,
             palette: PR, key: &str, f: F) {
        let g = || {
            let mut plot_data = PlotData::<CAM16UCS>::empty(w as usize, h as usize);
            for i in 0..w {
                let x = (i as f32 / (w - 1) as f32) * 2. - 1.;
                for j in 0..h {
                    let y = ((h - 1 - j) as f32 / (h - 1) as f32) * 2. - 1.;
                    let r = f32::hypot(x, y);
                    let a = f32::atan2(y, x) / (2. * std::f32::consts::PI);
                    let a = a.cyclic_clip(1.);
                    if r <= 1. {
                        plot_data.data[j as usize][i as usize] = f(r, a);
                    }
                }
            }
            return plot_data;
        };
        self.plot_data(x0, y0, w, h, palette, cacher.get_plot(key, g));
    }
    pub fn plot_data<PR: AsRef<Palette>>(&mut self,
            x0: i32, y0: i32, w: i32, h: i32,
            palette: PR, data: PlotData<CAM16UCS>) {
        let palette = palette.as_ref();
        for i in 0..w {
            for j in 0..h {
                match data.data[j as usize][i as usize] {
                    Some(c) => {
                        self.put_pixel(x0 + i, y0 + j, palette.nearest(c));
                    }
                    None => {}
                }
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    pub fn save(&self, name: String) -> Result<(), image::ImageError> {
        use crate::web;
        let mut data: Vec<u8> = vec![];
        let buf: std::io::Cursor<&mut Vec<u8>> = std::io::Cursor::new(&mut data);
        let encoder = image::codecs::png::PngEncoder::new(buf);
        let _ = encoder.encode(self.buffer.as_raw(), self.w, self.h, image::ColorType::Rgb8);
        let data: &Vec<u8> = &data;
        let encoded = hex::encode(data);
        web::write_storage(&name, encoded);
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self, name: String) -> Result<(), image::ImageError> {
        self.buffer.save(&name)?;

        // Writes an ICC profile if should.
        // Fails silently.
        if let Some(ref icc_profile) = self.icc_profile {
            let data = match std::fs::read(&name) {
                Ok(x) => { x }
                Err(_) => { return Ok(()); }
            };
            let mut png = match Png::from_bytes(data.into()) {
                Ok(x) => { x }
                Err(_) => { return Ok(()); }
            };
            png.set_icc_profile(Some(icc_profile.clone()));
            let file = match std::fs::File::create(&name) {
                Ok(x) => { x }
                Err(_) => { return Ok(()); }
            };
            let _ = png.encoder().write_to(file);
        }
        Ok(())
    }
}

pub trait GraphProvider<T: GraphPixel>: PixelWriter<T> {
    fn put_pixel(&mut self, x: i32, y: i32, c: T);
    fn frame(&mut self, x0: i32, y0: i32, w: i32, h: i32, c: T);
    fn block(&mut self, x0: i32, y0: i32, w: i32, h: i32, c: T);
    fn dither(&mut self, x0: i32, y0: i32, w: i32, h: i32, c1: T, c2: T);
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: T, dotted: Option<i32>);
    fn circle(&mut self, x0: i32, y0: i32, d: i32, c: T, dotted: Option<i32>);
    fn disc(&mut self, x0: i32, y0: i32, d: i32, c: T);
    fn text(&mut self, s: &str, x0: i32, y0: i32, p: TextAnchor, font: &Font, c: T);
    fn vtext(&mut self, s: &str, x0: i32, y0: i32, p: HorizontalTextAnchor, font: &Font, c: T);
    fn plot<F: Fn(f32, f32) -> Option<CAM16UCS>, P: CacheProvider, PR: AsRef<Palette>>
            (&mut self, cacher: &mut P,
             x0: i32, y0: i32, w: i32, h: i32,
             palette: PR, key: &str, f: F);
    fn plot_polar<F: Fn(f32, f32) -> Option<CAM16UCS>, P: CacheProvider, PR: AsRef<Palette>>
            (&mut self, cacher: &mut P,
             x0: i32, y0: i32, w: i32, h: i32,
             palette: PR, key: &str, f: F);
}

impl<T: GraphPixel> GraphProvider<T> for ImageGraph {
    fn put_pixel(&mut self, x: i32, y: i32, c: T) {
        self.put_pixel(x, y, c);
    }
    fn frame(&mut self, x0: i32, y0: i32, w: i32, h: i32, c: T) {
        self.frame(x0, y0, w, h, c);
    }
    fn block(&mut self, x0: i32, y0: i32, w: i32, h: i32, c: T) {
        self.block(x0, y0, w, h, c);
    }
    fn dither(&mut self, x0: i32, y0: i32, w: i32, h: i32, c1: T, c2: T) {
        self.dither(x0, y0, w, h, c1, c2);
    }
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: T, dotted: Option<i32>) {
        self.line(x0, y0, x1, y1, c, dotted);
    }
    fn circle(&mut self, x0: i32, y0: i32, d: i32, c: T, dotted: Option<i32>) {
        self.circle(x0, y0, d, c, dotted);
    }
    fn disc(&mut self, x0: i32, y0: i32, d: i32, c: T) {
        self.disc(x0, y0, d, c);
    }
    fn text(&mut self, s: &str, x0: i32, y0: i32, p: TextAnchor, font: &Font, c: T) {
        self.text(s, x0, y0, p, font, c);
    }
    fn vtext(&mut self, s: &str, x0: i32, y0: i32, p: HorizontalTextAnchor, font: &Font, c: T) {
        self.vtext(s, x0, y0, p, font, c);
    }
    fn plot<F: Fn(f32, f32) -> Option<CAM16UCS>, P: CacheProvider, PR: AsRef<Palette>>
            (&mut self, cacher: &mut P,
             x0: i32, y0: i32, w: i32, h: i32,
             palette: PR, key: &str, f: F) {
        self.plot(cacher, x0, y0, w, h, palette, key, f);
    }
    fn plot_polar<F: Fn(f32, f32) -> Option<CAM16UCS>, P: CacheProvider, PR: AsRef<Palette>>
            (&mut self, cacher: &mut P,
             x0: i32, y0: i32, w: i32, h: i32,
             palette: PR, key: &str, f: F) {
        self.plot_polar(cacher, x0, y0, w, h, palette, key, f);
    }
}

pub enum GraphRequest<T: GraphPixel> {
    Pixel { x: i32, y: i32, c: T },
    Frame { x0: i32, y0: i32, w: i32, h: i32, c: T },
    Block { x0: i32, y0: i32, w: i32, h: i32, c: T },
    Dither { x0: i32, y0: i32, w: i32, h: i32, c1: T, c2: T },
    Line { x0: i32, y0: i32, x1: i32, y1: i32, c: T, dotted: Option<i32> },
    Circle { x0: i32, y0: i32, d: i32, c: T, dotted: Option<i32> },
    Disc { x0: i32, y0: i32, d: i32, c: T },
    Text { s: String, x0: i32, y0: i32, p: TextAnchor, c: T },
    VText { s: String, x0: i32, y0: i32, p: HorizontalTextAnchor, c: T },
    PlotData { x0: i32, y0: i32, w: i32, h: i32, data: PlotData<CAM16UCS> }
}
unsafe impl<T: GraphPixel> Send for GraphRequest<T> {}

pub struct MultithreadedGraphProvider<T: GraphPixel> {
    sender: Sender<GraphRequest<T>>
}
impl<T: GraphPixel> MultithreadedGraphProvider<T> {
    pub fn new(sender: Sender<GraphRequest<T>>) -> Self {
        Self { sender }
    }
}
impl<T: GraphPixel> PixelWriter<T> for MultithreadedGraphProvider<T> {
    fn put_pixel(&mut self, x: i32, y: i32, c: T) {
        self.sender.send(GraphRequest::Pixel { x, y, c }).unwrap();
    }
}
impl<T: GraphPixel> GraphProvider<T> for MultithreadedGraphProvider<T> {
    fn put_pixel(&mut self, x: i32, y: i32, c: T) {
        self.sender.send(GraphRequest::Pixel { x, y, c }).unwrap();
    }
    fn frame(&mut self, x0: i32, y0: i32, w: i32, h: i32, c: T) {
        self.sender.send(GraphRequest::Frame { x0, y0, w, h, c }).unwrap();
    }
    fn block(&mut self, x0: i32, y0: i32, w: i32, h: i32, c: T) {
        self.sender.send(GraphRequest::Block { x0, y0, w, h, c }).unwrap();
    }
    fn dither(&mut self, x0: i32, y0: i32, w: i32, h: i32, c1: T, c2: T) {
        self.sender.send(GraphRequest::Dither { x0, y0, w, h, c1, c2 }).unwrap();
    }
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: T, dotted: Option<i32>) {
        self.sender.send(GraphRequest::Line { x0, y0, x1, y1, c, dotted }).unwrap();
    }
    fn circle(&mut self, x0: i32, y0: i32, d: i32, c: T, dotted: Option<i32>) {
        self.sender.send(GraphRequest::Circle { x0, y0, d, c, dotted }).unwrap();
    }
    fn disc(&mut self, x0: i32, y0: i32, d: i32, c: T) {
        self.sender.send(GraphRequest::Disc { x0, y0, d, c }).unwrap();
    }
    fn text(&mut self, s: &str, x0: i32, y0: i32, p: TextAnchor, _font: &Font, c: T) {
        self.sender.send(GraphRequest::Text { s: String::from(s), x0, y0, p, c }).unwrap();
    }
    fn vtext(&mut self, s: &str, x0: i32, y0: i32, p: HorizontalTextAnchor, _font: &Font, c: T) {
        self.sender.send(GraphRequest::VText { s: String::from(s), x0, y0, p, c }).unwrap();
    }
    fn plot<F: Fn(f32, f32) -> Option<CAM16UCS>, P: CacheProvider, PR: AsRef<Palette>>
            (&mut self, cacher: &mut P,
             x0: i32, y0: i32, w: i32, h: i32,
             _palette: PR, key: &str, f: F) {
        let g = || {
            let mut plot_data = PlotData::<CAM16UCS>::empty(w as usize, h as usize);
            for i in 0..w {
                let x = i as f32 / (w as f32 - 1.);
                for j in 0..h {
                    let y = (h - 1 - j) as f32 / (h as f32 - 1.);
                    plot_data.data[j as usize][i as usize] = f(x, y);
                }
            }
            return plot_data;
        };
        let data = cacher.get_plot(key, g);
        self.sender.send(GraphRequest::PlotData { x0, y0, w, h, data }).unwrap();
    }
    fn plot_polar<F: Fn(f32, f32) -> Option<CAM16UCS>, P: CacheProvider, PR: AsRef<Palette>>
            (&mut self, cacher: &mut P,
             x0: i32, y0: i32, w: i32, h: i32,
             _palette: PR, key: &str, f: F) {
        let g = || {
            let mut plot_data = PlotData::<CAM16UCS>::empty(w as usize, h as usize);
            for i in 0..w {
                let x = (i as f32 / (w - 1) as f32) * 2. - 1.;
                for j in 0..h {
                    let y = ((h - 1 - j) as f32 / (h - 1) as f32) * 2. - 1.;
                    let r = f32::hypot(x, y);
                    let a = f32::atan2(y, x) / (2. * std::f32::consts::PI);
                    let a = a.cyclic_clip(1.);
                    if r <= 1. {
                        plot_data.data[j as usize][i as usize] = f(r, a);
                    }
                }
            }
            return plot_data;
        };
        let data = cacher.get_plot(key, g);
        self.sender.send(GraphRequest::PlotData { x0, y0, w, h, data }).unwrap();
    }
}

pub struct GraphHoster<'a, T: GraphPixel> {
    palette: Palette,
    font: Font,
    connections: Vec<Receiver<GraphRequest<T>>>,
    graph: &'a mut ImageGraph
}
impl<'a, T: GraphPixel> GraphHoster<'a, T> {
    pub fn new(graph: &'a mut ImageGraph, palette: Palette, font: Font) -> Self {
        Self { palette, font, graph, connections: vec![] }
    }
    pub fn register(&mut self) -> Sender<GraphRequest<T>> {
        let (send, recv) = crossbeam_channel::bounded(0);
        self.connections.push(recv);
        return send;
    }
    pub fn process(&mut self) {
        while self.connections.len() > 0 {
            let mut select = crossbeam_channel::Select::new();
            for recv in self.connections.iter() {
                select.recv(recv);
            }
            let op = select.select();
            let i = op.index();
            match op.recv(&self.connections[i]) {
                Ok(GraphRequest::Pixel { x, y, c }) => {
                    self.graph.put_pixel(x, y, c);
                }
                Ok(GraphRequest::Frame { x0, y0, w, h, c }) => {
                    self.graph.frame(x0, y0, w, h, c);
                }
                Ok(GraphRequest::Block { x0, y0, w, h, c }) => {
                    self.graph.block(x0, y0, w, h, c);
                }
                Ok(GraphRequest::Dither { x0, y0, w, h, c1, c2 }) => {
                    self.graph.dither(x0, y0, w, h, c1, c2);
                }
                Ok(GraphRequest::Line { x0, y0, x1, y1, c, dotted }) => {
                    self.graph.line(x0, y0, x1, y1, c, dotted);
                }
                Ok(GraphRequest::Circle { x0, y0, d, c, dotted }) => {
                    self.graph.circle(x0, y0, d, c, dotted);
                }
                Ok(GraphRequest::Disc { x0, y0, d, c }) => {
                    self.graph.disc(x0, y0, d, c);
                }
                Ok(GraphRequest::Text { s, x0, y0, p, c }) => {
                    self.graph.text(&s, x0, y0, p, &self.font, c);
                }
                Ok(GraphRequest::VText { s, x0, y0, p, c }) => {
                    self.graph.vtext(&s, x0, y0, p, &self.font, c);
                }
                Ok(GraphRequest::PlotData { x0, y0, w, h, data }) => {
                    self.graph.plot_data(x0, y0, w, h, &self.palette, data);
                }
                Err(_) => {
                    self.connections.remove(i);
                }
            }
        }
    }
}
