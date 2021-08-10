use image::{RgbImage, Rgb};

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
    w: u32,
    h: u32
}
impl ImageGraph {
    pub fn new(w: u32, h: u32) -> Self {
        let buffer = RgbImage::new(w, h);
        Self { buffer, w, h }
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
    pub fn plot<F: Fn(f32, f32) -> Option<CAM16UCS>>
            (&mut self, cacher: &mut PlotCacher,
             x0: i32, y0: i32, w: i32, h: i32,
             palette: &Palette, key: &str, f: F) {
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
        self.plot_data(x0, y0, w, h, palette, cacher.get(key, g));
    }
    pub fn plot_polar<F: Fn(f32, f32) -> Option<CAM16UCS>>
            (&mut self, cacher: &mut PlotCacher,
             x0: i32, y0: i32, w: i32, h: i32,
             palette: &Palette, key: &str, f: F) {
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
        self.plot_data(x0, y0, w, h, palette, cacher.get(key, g));
    }
    fn plot_data(&mut self,
            x0: i32, y0: i32, w: i32, h: i32,
            palette: &Palette, data: &PlotData<CAM16UCS>) {
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
    pub fn save(&self, name: String) -> Result<(), image::ImageError> {
        self.buffer.save(name)
    }
}
