use crate::colour::*;
use crate::palette::Palette;
use crate::cache::PlotCacher;
use crate::graph::ImageGraph;
use crate::util::{Clip, CyclicClip, PackedF32, Lerp};
use crate::text::{Font, TextAnchor};

use std::collections::HashMap;
use std::f32::consts::PI;

pub trait Widget {
    fn render(&self,
              graph: &mut ImageGraph,
              cacher: &mut PlotCacher,
              palette: &Palette,
              ill: &CAT16Illuminant,
              font: &Font,
              x0: i32, y0: i32);
}

pub struct RectJChWidget {
    w: i32,
    h: i32,
    C: f32
}
impl RectJChWidget {
    pub fn new(w: i32, h: i32, C: f32) -> Self {
        Self { w, h, C }
    }
}
impl Widget for RectJChWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        graph.plot(
            cacher, x0, y0, self.w, self.h,
            palette, &format!("RectJCh:C={:.2}", self.C),
            |x, y| { Some(CAM16UCS{
                J: (1. - y) * 100.,
                a: self.C * f32::cos(x * 2. * PI),
                b: self.C * f32::sin(x * 2. * PI),
                C: self.C
            })}
        );
    }
}

pub struct IndexedWidget {
    slots_x: i32,
    slots_y: i32,
    ww: i32,
    hh: i32
}
impl IndexedWidget {
    pub fn new(slots_x: i32, slots_y: i32, ww: i32, hh: i32) -> Self {
        Self { slots_x, slots_y, ww, hh }
    }
}
impl Widget for IndexedWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        graph.frame(
            x0, y0,
            self.ww * self.slots_x + 4, self.hh * self.slots_y + 4,
            palette.bg_rgb
        );
        for ix in 0..self.slots_x {
            let x = x0 + 2 + ix * self.ww;
            for iy in 0..self.slots_y {
                let y = y0 + 2 + iy * self.hh;
                let i = (iy * self.slots_x + ix) as usize;
                if i < palette.n {
                    graph.block(x, y, self.ww, self.hh, palette.rgb[i]);
                } else {
                    graph.block(x, y, self.ww, self.hh, palette.rgb[palette.n - 1]);
                    graph.block(x + 1, y + 1, self.ww - 2, 1, palette.fg_rgb);
                    graph.block(x + 1, y + self.hh - 2, self.ww - 2, 1, palette.bl_rgb);
                }
            }
        }
    }
}

pub struct CloseLiMatchWidget {
    ww: i32,
    hh: i32,
    n: usize,
    limatch: f32
}
impl CloseLiMatchWidget {
    pub fn new(ww: i32, hh: i32, n: usize, limatch: f32) -> Self {
        Self { ww, hh, n, limatch }
    }
}
impl Widget for CloseLiMatchWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        let mut pairs = vec![];
        for i in 0..palette.n {
            for j in i+1..palette.n {
                let d = CAM16UCS::dist_limatch(palette.cam16[i], palette.cam16[j], self.limatch);
                pairs.push((i, j, d));
            }
        }
        pairs.sort_by_key(|&(_, _, d)| { PackedF32(d) });
        for k in 0..self.n {
            let x = x0 + (self.ww + 1) * k as i32;
            if k < pairs.len() {
                let (i, j) = (pairs[k].0, pairs[k].1);
                graph.block(x, y0, self.ww, self.hh, palette.rgb[i]);
                graph.block(x, y0 + self.hh, self.ww, self.hh, palette.rgb[j]);
                if i == palette.bl || j == palette.bl {
                    graph.frame(x, y0, self.ww, self.hh * 2, palette.bg_rgb);
                }
            } else {
                graph.dither(
                    x, y0, self.ww, self.hh * 2,
                    palette.bg_rgb, palette.bl_rgb
                );
            }
        }
    }
}

pub struct SpectrumWidget {
    w: i32,
    h: i32,
    ratio: f32
}
impl SpectrumWidget {
    pub fn new(w: i32, h: i32) -> Self {
        Self { w, h, ratio: 0.8 }
    }
}
impl Widget for SpectrumWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              cacher: &mut PlotCacher,
              palette: &Palette,
              ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        let w_spectral = (self.w as f32 * self.ratio) as i32;
        let w_extra = self.w - w_spectral;
        graph.plot(
            cacher, x0, y0, w_spectral, self.h,
            palette, "Spectrum",
            |x, _| {
                let wl = Wavelength::MIN as f32
                    + x * (Wavelength::MAX as f32 - Wavelength::MIN as f32);
                Some(CAM16UCS::of(Wavelength::new(wl).into(), ill))
            }
        );
        graph.plot(
            cacher, x0, y0 + self.h + 1, w_spectral, self.h,
            palette, "Spectrum:chr50",
            |x, _| {
                let wl = Wavelength::MIN as f32
                    + x * (Wavelength::MAX as f32 - Wavelength::MIN as f32);
                Some(CAM16UCS::of(Wavelength::new(wl).into(), ill).chr50())
            }
        );
        graph.plot(
            cacher, x0, y0 + (self.h + 1) * 2, w_spectral, self.h,
            palette, "Spectrum:li50",
            |x, _| {
                let wl = Wavelength::MIN as f32
                    + x * (Wavelength::MAX as f32 - Wavelength::MIN as f32);
                Some(CAM16UCS::of(Wavelength::new(wl).into(), ill).li50())
            }
        );
        let min = CAM16UCS::of(Wavelength::new(Wavelength::MIN as f32).into(), ill);
        let max = CAM16UCS::of(Wavelength::new(Wavelength::MAX as f32).into(), ill);
        graph.plot(
            cacher, x0 + w_spectral, y0, w_extra, self.h,
            palette, "SpectrumExtra",
            |x, _| { Some(CAM16UCS::mix(max, min, x)) }
        );
        graph.plot(
            cacher, x0 + w_spectral, y0 + self.h + 1, w_extra, self.h,
            palette, "SpectrumExtra:chr50",
            |x, _| { Some(CAM16UCS::mix(max, min, x).chr50()) }
        );
        graph.plot(
            cacher, x0 + w_spectral, y0 + (self.h + 1) * 2, w_extra, self.h,
            palette, "SpectrumExtra:li50",
            |x, _| { Some(CAM16UCS::mix(max, min, x).li50()) }
        );
    }
}

pub struct SpectroBoxWidget {
    w: i32,
    h: i32,
    ratio: f32
}
impl SpectroBoxWidget {
    pub fn new(w: i32, h: i32) -> Self {
        Self { w, h, ratio: 0.8 }
    }
}
impl Widget for SpectroBoxWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              cacher: &mut PlotCacher,
              palette: &Palette,
              ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        let w_spectral = (self.w as f32 * self.ratio) as i32;
        let w_extra = self.w - w_spectral;
        graph.plot(
            cacher, x0, y0, w_spectral, self.h,
            palette, "SpectroBox",
            |x, y| {
                let t = 2. * y - 1.;
                let wl = Wavelength::MIN as f32
                    + x * (Wavelength::MAX as f32 - Wavelength::MIN as f32);
                let c = CAM16UCS::of(Wavelength::new(wl).into(), ill);
                let J = if t < 0. {
                    f32::interpolate(c.J, 0., -t)
                } else {
                    f32::interpolate(c.J, 100., t)
                };
                // incorrect, maybe replace later
                let C = f32::hypot(c.a, c.b) * (1. - t * t) / 100.;
                let a = C * c.a;
                let b = C * c.b;
                Some(CAM16UCS { J, a, b, C })
            }
        );
        let min = CAM16UCS::of(Wavelength::new(Wavelength::MIN as f32).into(), ill);
        let max = CAM16UCS::of(Wavelength::new(Wavelength::MAX as f32).into(), ill);
        graph.plot(
            cacher, x0 + w_spectral, y0, w_extra, self.h,
            palette, "SpectroBoxExtra",
            |x, y| {
                let t = 2. * y - 1.;
                let c = CAM16UCS::mix(max, min, x);
                let J = if t < 0. {
                    f32::interpolate(c.J, 0., -t)
    } else {
                    f32::interpolate(c.J, 100., t)
                };
                // incorrect, maybe replace later
                let C = f32::hypot(c.a, c.b) * (1. - t * t) / 100.;
                let a = C * c.a;
                let b = C * c.b;
                Some(CAM16UCS { J, a, b, C })
            }
        );
    }
}

#[derive(Clone, Copy)]
pub enum EvalState {
    Ok, Warn, Alert
}
impl Widget for EvalState {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              font: &Font,
              x0: i32, y0: i32) {
        let d = 11;
        graph.frame(x0, y0, d, d, palette.bg_rgb);
        let glyph = match self {
            Self::Ok => { &font.ok }
            Self::Warn => { &font.warn }
            Self::Alert => { &font.alert }
        };
        font.render_glyph(graph, x0 + 2, y0 + 2, glyph, palette.fg_rgb);
    }
}

pub struct BarBoxWidget {
    w: i32,
    h: i32,
    text: Vec<String>,
    v: f32,
    threshold: Option<f32>
}
impl BarBoxWidget {
    pub fn new(w: i32, h: i32, text: Vec<String>, v: f32, threshold: Option<f32>) -> Self {
        Self { w, h, text, v, threshold }
    }
}
impl Widget for BarBoxWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              font: &Font,
              x0: i32, y0: i32) {
        graph.frame(x0, y0, self.w, self.h, palette.bg_rgb);
        let text_x = x0 + self.w / 2;
        let text_y0 = y0 + 2;
        for i in 0..self.text.len() {
            let y = text_y0 + 6 * i as i32;
            let s = &self.text[i];
            graph.text(s, text_x, y, TextAnchor::n(), font, palette.fg_rgb);
        }

        let bar_x = x0 + 2;
        let bar_y = y0 + self.h - 7;
        let bar_w = self.w - 4;
        let bar_h = 4;
        let progress = self.v.clip(0., 1.);
        let progress_w = (((bar_w as f32 - 2.) * progress) as i32).clip(0, bar_w - 2);
        graph.frame(bar_x, bar_y, bar_w, bar_h, palette.bg_rgb);
        graph.block(bar_x + 1, bar_y + 1, progress_w, bar_h - 2, palette.fg_rgb);

        if let Some(t) = self.threshold {
            let t = t.clip(0., 1.);
            let threshold_w = (((bar_w as f32 - 2.) * t) as i32).clip(0, bar_w - 2);
            graph.line(
                bar_x + 1 + threshold_w, bar_y - 1,
                bar_x + bar_w - 1, bar_y - 1,
                palette.bg_rgb, None
            );
            graph.line(
                bar_x + 1 + threshold_w, bar_y + bar_h,
                bar_x + bar_w - 1, bar_y + bar_h,
                palette.bg_rgb, None
            );
        }
    }
}

pub struct YesNoBoxWidget {
    w: i32,
    h: i32,
    text: Vec<String>,
    v: bool
}
impl YesNoBoxWidget {
    pub fn new(w: i32, h: i32, text: Vec<String>, v: bool) -> Self {
        Self { w, h, text, v }
    }
}
impl Widget for YesNoBoxWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              font: &Font,
              x0: i32, y0: i32) {
        graph.frame(x0, y0, self.w, self.h, palette.bg_rgb);
        let text_x = x0 + self.w / 2;
        let text_y0 = y0 + 2;
        for i in 0..self.text.len() {
            let y = text_y0 + 6 * i as i32;
            let s = &self.text[i];
            graph.text(s, text_x, y, TextAnchor::n(), font, palette.fg_rgb);
        }

        let result_y = y0 + self.h - 3;
        let s = if self.v { "<yes>" } else { "<no>" };
        graph.text(s, text_x, result_y, TextAnchor::s(), font, palette.fg_rgb);
    }
}

pub struct ISSWidget {
    w: i32,
    h: i32,
    warn: f32,
    alert: f32
}
impl ISSWidget {
    pub fn new(w: i32, h: i32, warn: f32, alert: f32) -> Self {
        Self { w, h, warn, alert }
    }
}
impl Widget for ISSWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              cacher: &mut PlotCacher,
              palette: &Palette,
              ill: &CAT16Illuminant,
              font: &Font,
              x0: i32, y0: i32) {
        let iss = palette.internal_similarity();
        let iss_min = 0.4;
        let barbox = BarBoxWidget::new(
            self.w, self.h,
            vec!["internal".into(), "similarity".into()],
            (iss - iss_min) / (self.alert - iss_min),
            Some((self.warn - iss_min) / (self.alert - iss_min))
        );
        barbox.render(graph, cacher, palette, ill, font, x0, y0);
        let eval = match iss {
            x if x < self.warn => { EvalState::Ok }
            x if x < self.alert => { EvalState::Warn }
            _ => { EvalState::Alert }
        };
        let eval_x = x0 + self.w - 1;
        eval.render(graph, cacher, palette, ill, font, eval_x, y0);
    }
}

pub struct AcyclicWidget {
    w: i32,
    h: i32
}
impl AcyclicWidget {
    pub fn new(w: i32, h: i32) -> Self {
        Self { w, h }
    }
}
impl Widget for AcyclicWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              cacher: &mut PlotCacher,
              palette: &Palette,
              ill: &CAT16Illuminant,
              font: &Font,
              x0: i32, y0: i32) {
        let acyclic = palette.is_acyclic();
        let yesnobox = YesNoBoxWidget::new(
            self.w, self.h,
            vec!["acyclic?".into()],
            acyclic
        );
        yesnobox.render(graph, cacher, palette, ill, font, x0, y0);
        let eval = match acyclic {
            false => { EvalState::Ok }
            true if palette.n > 3 => { EvalState::Warn }
            _ => { EvalState::Ok }
        };
        let eval_x = x0 - 10;
        let eval_y = y0 + self.h - 11;
        eval.render(graph, cacher, palette, ill, font, eval_x, eval_y);
    }
}

pub struct DistributionWidget {
    w: i32,
    h: i32,
    dist: HashMap<PackedF32, f32>,
    dist_points: HashMap<usize, f32>,
    s: f32
}
impl DistributionWidget {
    pub fn new(w: i32, h: i32,
               dist: HashMap<PackedF32, f32>, dist_points: HashMap<usize, f32>,
               s: f32) -> Self {
        Self { w, h, dist, dist_points, s }
    }
}
impl Widget for DistributionWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        graph.frame(x0, y0, self.w, self.h, palette.bg_rgb);

        let plot_x = x0 + 2;
        let plot_y = y0 + 2;
        let plot_w = self.w - 4;
        let plot_h = self.h - 4;
        
        let mut data: Vec<f32> = vec![0.; plot_w as usize];
        for i in 0..plot_w {
            let x = i as f32 / (plot_w as f32 - 1.);
            for (PackedF32(y), w) in self.dist.iter() {
                let t = (x - y) / self.s;
                data[i as usize] += w * f32::exp(-(t * t) / 2.);
            }
        }
        let norm = data.iter().map(|&f| PackedF32(f)).max().unwrap().0;
        if norm > 0. {
            for i in 0..plot_w {
                data[i as usize] /= norm;
            }
        }

        let mut dist_points: Vec<(usize, f32)> = self.dist_points.iter()
            .map(|(&i, &xx)| (i, xx))
            .collect();
        dist_points.sort_by_key(|(i, _)| PackedF32(palette.cam16[*i].C));
        let mut marks = vec![0; plot_w as usize];
        for &(i, xx) in dist_points.iter() {
            let c = palette.rgb[i];
            let xi = ((xx * (plot_w - 1) as f32) as i32).clip(0, plot_w - 1);
            let x = plot_x + xi;
            let yy_max = (((plot_h - 1) as f32 * data[xi as usize]) as i32).clip(0, plot_h - 1) + 1;
            let y = y0 + self.h - 2 - (marks[xi as usize] % yy_max);
            graph.put_pixel(x, y, c);
            marks[xi as usize] += 1;
        }
        for i in 0..plot_w-1 {
            let from = data[i as usize];
            let to = data[(i + 1) as usize];
            let from_y = (((plot_h - 1) as f32 * from) as i32).clip(0, plot_h - 1);
            let to_y = (((plot_h - 1) as f32 * to) as i32).clip(0, plot_h - 1);
            graph.line(
                plot_x + i, plot_y + plot_h - 1 - from_y,
                plot_x + i + 1, plot_y + plot_h - 1 - to_y,
                palette.fg_rgb, None
            );
        }
    }
}

pub struct SpectralDistributionWidget {
    w: i32,
    h: i32
}
impl SpectralDistributionWidget {
    pub fn new(w: i32, h: i32) -> Self {
        Self { w, h }
    }
}
impl Widget for SpectralDistributionWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              cacher: &mut PlotCacher,
              palette: &Palette,
              ill: &CAT16Illuminant,
              font: &Font,
              x0: i32, y0: i32) {
        let min = Wavelength::MIN as f32;
        let max = Wavelength::MAX as f32;
        let (dist, points) = palette.spectral_stats(ill);
        let dist = dist.iter()
            .map(|(&PackedF32(k), &v)| (
                PackedF32((k - min) / (max - min)),
                v
            ))
            .collect();
        let points = points.iter()
            .map(|(&i, &x)| (i, (x - min) / (max - min)))
            .collect();
        let distribution = DistributionWidget::new(self.w, self.h, dist, points, 0.02083333);
        distribution.render(graph, cacher, palette, ill, font, x0, y0);
        graph.text(
            &format!("{}", Wavelength::MIN),
            x0, y0 + self.h + 1,
            TextAnchor::nw(), font,
            palette.bg_rgb
        );
        graph.text(
            &format!("{}", Wavelength::MAX),
            x0 + self.w, y0 + self.h + 1,
            TextAnchor::ne(), font,
            palette.bg_rgb
        );
    }
}

pub struct TemperatureDistributionWidget {
    w: i32,
    h: i32
}
impl TemperatureDistributionWidget {
    pub fn new(w: i32, h: i32) -> Self {
        Self { w, h }
    }
}
impl Widget for TemperatureDistributionWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              cacher: &mut PlotCacher,
              palette: &Palette,
              ill: &CAT16Illuminant,
              font: &Font,
              x0: i32, y0: i32) {
        let max = f32::log10(CIEuv::CCT_MAX as f32);
        let min = f32::log10(CIEuv::CCT_MIN as f32);
        let (dist, points) = palette.CCT_stats();
        let dist = dist.iter()
            .map(|(&PackedF32(k), &v)| (
                PackedF32(1. - (f32::log10(k) - min) / (max - min)),
                v
            ))
            .collect();
        let points = points.iter()
            .map(|(&i, &x)| (i, 1. - (f32::log10(x) - min) / (max - min)))
            .collect();
        let distribution = DistributionWidget::new(self.w, self.h, dist, points, 0.02083333);
        distribution.render(graph, cacher, palette, ill, font, x0, y0);
        graph.text(
            "COLD",
            x0, y0 + self.h + 1,
            TextAnchor::nw(), font,
            palette.bg_rgb
        );
        graph.text(
            "WARM",
            x0 + self.w, y0 + self.h + 1,
            TextAnchor::ne(), font,
            palette.bg_rgb
        );
    }
}

pub struct LiMatchGreyscaleWidget {
    w: i32,
    h: i32
}
impl LiMatchGreyscaleWidget {
    pub fn new(w: i32, h: i32) -> Self {
        Self { w, h }
    }
}
impl Widget for LiMatchGreyscaleWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        for i in 0..self.w {
            let x = i as f32 / (self.w as f32 - 1.);
            for j in 0..self.h {
                let y = (self.h - 1 - j) as f32 / (self.h as f32 - 1.);
                let J = y * 100.;
                let c = palette.nearest_limatch(CAM16UCS { J, a:0., b:0., C:0. }, x);
                graph.put_pixel(x0 + i, y0 + j, c);
            }
        }
        let (mdx, mw) = if palette.n <= 64 { (3, 2) } else { (2, 1) };
        let mut marks = vec![0; self.h as usize];
        for i in 0..palette.n {
            let yy = ((palette.cam16[i].J / 100. * (self.h - 1) as f32) as i32).clip(0, self.h - 1);
            let x = x0 + self.w + 1 + marks[yy as usize] * (mdx + mw);
            graph.block(x, y0 + self.h - 1 - yy, mw, 1, palette.rgb[i]);
            marks[yy as usize] += 1;
        }
    }
}

pub struct IsometricCubeWidget {
    w: i32,
    points: Vec<(f32, f32, f32, usize)>
}
impl IsometricCubeWidget {
    pub fn new(w: i32, points: Vec<(f32, f32, f32, usize)>) -> Self {
        Self { w, points }
    }
}
impl Widget for IsometricCubeWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        let h = (self.w as f32 * f32::sqrt(1.25)) as i32;
        let cx = x0 + self.w / 2;
        let cy = y0 + h / 2;
        let dy = h / 4;
        let dd = ((32. / f32::sqrt(palette.n as f32)) as i32).clip(2, 5);
        let vertices = [
            (cx, y0),
            (x0 + self.w, y0 + dy),
            (x0 + self.w, y0 + h - dy),
            (cx, y0 + h),
            (x0, y0 + h - dy),
            (x0, y0 + dy)
        ];
        for i in 0..6 {
            let (p1, p2) = (vertices[i], vertices[(i + 1) % 6]);
            graph.line(p1.0, p1.1, p2.0, p2.1, palette.bg_rgb, None);
        }
        graph.line(cx, cy, vertices[0].0, vertices[0].1, palette.bg_rgb, None);
        graph.line(cx, cy, vertices[2].0, vertices[2].1, palette.bg_rgb, None);
        graph.line(cx, cy, vertices[4].0, vertices[4].1, palette.bg_rgb, None);

        let mut sorted = self.points.iter()
            .map(|&(px, py, pz, i)| (px+py+pz, px, py, pz, i))
            .collect::<Vec<_>>();
        sorted.sort_by_key(|&(a, _, _, _, _)| PackedF32(a));
        let sorted = sorted.into_iter()
            .map(|(_, px, py, pz, i)| (px, py, pz, i))
            .collect::<Vec<_>>();

        for &(px, py, pz, i) in sorted.iter() {
            let xx = ((py - px) * self.w as f32) as i32 / 2;
            let yy = ((px + py) * dy as f32 - pz * h as f32 / 2.) as i32;
            graph.disc(cx + xx - dd / 2, cy + yy - dd / 2, dd, palette.rgb[i]);
            if i == palette.bl {
                graph.circle(
                    cx + xx - dd / 2 - 1, cy + yy - dd / 2 - 1, dd + 1,
                    palette.bg_rgb, None
                );
            }
        }
    }
}

pub struct CAM16IsoCubesWidget {
    ww: i32,
    dx: i32
}
impl CAM16IsoCubesWidget {
    pub fn new(ww: i32, dx: i32) -> Self {
        Self { ww, dx }
    }
}
impl Widget for CAM16IsoCubesWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              cacher: &mut PlotCacher,
              palette: &Palette,
              ill: &CAT16Illuminant,
              font: &Font,
              x0: i32, y0: i32) {
        let mut points: Vec<_> = (0..palette.n)
            .map(|i| (
                (palette.cam16[i].a / 200. + 0.5).clip(0., 1.),
                (palette.cam16[i].b / 200. + 0.5).clip(0., 1.),
                (palette.cam16[i].J / 100.).clip(0., 1.),
                i
            ))
            .collect();
        let cube1 = IsometricCubeWidget::new(self.ww, points.clone());
        points = points.iter()
            .map(|&(px, py, pz, i)| (1. - py, px, pz, i))
            .collect();
        let cube2 = IsometricCubeWidget::new(self.ww, points);
        cube1.render(graph, cacher, palette, ill, font, x0, y0);
        cube2.render(graph, cacher, palette, ill, font, x0 + self.ww + self.dx, y0);
    }
}

pub struct ChromaLightnessHueWidget {
    w1: i32,
    hh1: i32,
    w2: i32,
    h2: i32
}
impl ChromaLightnessHueWidget {
    pub fn new(w1: i32, hh1: i32, w2: i32, h2: i32) -> Self {
        Self { w1, hh1, w2, h2 }
    }
}
impl Widget for ChromaLightnessHueWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              font: &Font,
              x0: i32, y0: i32) {
        let h1 = (self.hh1 - 1) * 3 + 1;
        graph.text("CHR", x0, y0 - 1, TextAnchor::sw(), font, palette.fg_rgb);
        for i in 0..3 {
            let y = y0 + (self.hh1 - 1) * i;
            graph.frame(x0, y, self.w1, self.hh1, palette.bg_rgb);
        }
        graph.frame(x0 - 4, y0, 3, h1, palette.bg_rgb);
        let mut chroma_stats = [0; 3];
        for i in 0..palette.n {
            let c = palette.cam16[i];
            let C = (c.C / 100.).clip(0., 1.);
            let group = ((C * 3.) as i32).clip(0, 2);
            let inner_x = x0 + 2;
            let inner_y = y0 + (2 - group) * (self.hh1 - 1) + 2;
            let inner_w = self.w1 - 4;
            let inner_h = self.hh1 - 4;
            let x = ((c.J / 100. * (inner_w - 1) as f32) as i32).clip(0, inner_w - 1);
            let h = (f32::atan2(c.b, c.a) / (2. * PI)).cyclic_clip(1.);
            let y = ((h * (inner_h - 1) as f32) as i32).clip(0, inner_h - 1);
            graph.put_pixel(inner_x + x, inner_y + inner_h - 1 - y, palette.rgb[i]);
            chroma_stats[group as usize] += 1;
        }
        for i in 0..3 {
            if chroma_stats[i] == 0 { continue; }
            let p = chroma_stats[i] as f32 / chroma_stats.iter().sum::<i32>() as f32;
            let l = (p * (self.hh1 - 1) as f32) as i32;
            let x = x0 - 3;
            let y = y0 + (2 - i as i32) * (self.hh1 - 1) + (self.hh1 - 1) / 2;
            graph.line(x, y - l / 2 + 1, x, y + l / 2 - 1, palette.fg_rgb, None);
        }
        let x0 = x0 + self.w1 + 1;
        graph.text("LI-HUE", x0 + self.w2, y0 - 1, TextAnchor::se(), font, palette.fg_rgb);
        graph.frame(x0, y0, self.w2, self.h2, palette.bg_rgb);
        let x_offset = 5;
        let y_offset = 5;
        let inner_x = x0 + x_offset;
        let inner_y = y0 + y_offset;
        let inner_w = self.w2 - 2 * x_offset;
        let inner_h = self.h2 - 2 * y_offset;
        let dd = ((48. / f32::sqrt(palette.n as f32)) as i32).clip(1, 7);
        for i in 1..6 {
            let y = inner_y + i * inner_h / 6;
            graph.line(x0, y, x0 + self.w2 - 1, y, palette.bg_rgb, Some(2));
        }
        graph.line(x0, inner_y, x0 + self.w2 - 1, inner_y, palette.bg_rgb, None);
        graph.line(
            x0, y0 + self.h2 - 1 - y_offset,
            x0 + self.w2 - 1, y0 + self.h2 - 1 - y_offset,
            palette.bg_rgb, None
        );
        graph.line(inner_x, y0, inner_x, y0 + self.h2 - 1, palette.bg_rgb, None);
        graph.line(
            x0 + self.w2 / 2, y0,
            x0 + self.w2 / 2, y0 + self.h2 - 1,
            palette.bg_rgb, None
        );
        graph.line(
            x0 + self.w2 - 1 - x_offset, y0,
            x0 + self.w2 - 1 - x_offset, y0 + self.h2 - 1,
            palette.bg_rgb, None
        );
        let mut marks = vec![0; self.w2 as usize];
        let inner_x = inner_x + 1;
        let inner_y = inner_y + 1;
        let inner_w = inner_w - 2;
        let inner_h = inner_h - 2;
        for i in 0..palette.n {
            let c = palette.cam16[i];
            let x = ((c.J / 100. * (inner_w - 1) as f32) as i32).clip(0, inner_w - 1);
            let h = (f32::atan2(c.b, c.a) / (2. * PI)).cyclic_clip(1.);
            let y = ((h * (inner_h - 1) as f32) as i32).clip(0, inner_h - 1);
            graph.disc(
                inner_x + x - dd / 2, inner_y + inner_h - 1 - y - dd / 2,
                dd, palette.rgb[i]
            );
            if i == palette.bl {
                graph.circle(
                    inner_x + x - dd / 2 - 1, inner_y + inner_h - 1 - y - dd / 2 - 1,
                    dd + 1, palette.bg_rgb, None
                );
            }
            graph.put_pixel(
                inner_x + x, y0 + self.h2 + 1 + marks[(x + 1 + x_offset) as usize],
                palette.rgb[i]
            );
            marks[(x + 1 + x_offset) as usize] += 1;
        }
    }
}

pub struct UsefulMixesWidget {
    xn: i32,
    yn: i32,
    ww: i32,
    hh: i32
}
impl UsefulMixesWidget {
    pub fn new(xn: i32, yn: i32, ww: i32, hh: i32) -> Self {
        Self { xn, yn, ww, hh }
    }
}
impl Widget for UsefulMixesWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        let pairs = palette.useful_mixes((self.xn * self.yn) as usize);
        for xi in 0..self.xn {
            let x = x0 + (self.ww + 1) * xi;
            for yi in 0..self.yn {
                let y = y0 + (self.hh + 1) * yi;
                let i = (yi * self.xn + xi) as usize;
                if i < pairs.len() {
                    graph.dither(
                        x, y, self.ww, self.hh,
                        palette.rgb[pairs[i].0], palette.rgb[pairs[i].1]
                    );
                } else {
                    graph.frame(x, y, self.ww, self.hh, palette.bg_rgb);
                }
            }
        }
    }
}

pub struct LightnessChromaComponentsWidget {
    w: i32,
    h: i32
}
impl LightnessChromaComponentsWidget {
    pub fn new(w: i32, h: i32) -> Self {
        Self { w, h }
    }
}
impl Widget for LightnessChromaComponentsWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              font: &Font,
              x0: i32, y0: i32) {
        let hh = (self.h / palette.n as i32).clip(1, 6);
        let n = (self.h + 1) / (hh + 1);
        let w_empty = 4;
        let ww = (self.w - w_empty) / 2;
        let x1 = x0 + self.w - ww;
        graph.text("LI", x0, y0 - 1, TextAnchor::sw(), font, palette.fg_rgb);
        graph.text("CHR", x0 + self.w, y0 - 1, TextAnchor::se(), font, palette.fg_rgb);
        for i in 0..n {
            let y = y0 + (hh + 1) * i;
            if i < palette.n as i32 {
                let c = palette.cam16[i as usize];
                let J = (c.J / 100.).clip(0., 1.);
                let C = (c.C / 100.).clip(0., 1.);
                let l_J = ((J * ww as f32).round() as i32).clip(0, ww);
                let l_C = ((C * ww as f32).round() as i32).clip(0, ww);
                if l_J >= 1 {
                    graph.block(x0 + ww - l_J, y, l_J, hh, palette.rgb[i as usize]);
                }
                if ww - l_J - 1 >= 1 {
                    graph.frame(x0, y, ww - l_J - 1, hh, palette.bg_rgb);
                }
                if l_C >= 1 {
                    graph.block(x1, y, l_C, hh, palette.rgb[i as usize]);
                }
                if ww - l_C - 1 >= 1 {
                    graph.frame(x1 + l_C + 1, y, ww - l_C - 1, hh, palette.bg_rgb);
                }
            } else {
                graph.dither(
                    x0, y, ww, hh,
                    palette.bg_rgb, palette.bl_rgb
                );
                graph.dither(
                    x1, y, ww, hh,
                    palette.bg_rgb, palette.bl_rgb
                );
            }
        }
    }
}

pub struct MainPaletteWidget {
    w: i32,
    h: i32
}
impl MainPaletteWidget {
    pub fn new(w: i32, h: i32) -> Self {
        Self { w, h }
    }
}
impl Widget for MainPaletteWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        let ww = self.w / palette.n as i32;
        for i in 0..palette.n {
            let x = x0 + ww * i as i32;
            let i = palette.sorted[i];
            graph.block(x, y0, ww, self.h, palette.rgb[i]);
            if i == palette.bl {
                if ww >= 3 {
                    graph.frame(x, y0, ww, self.h, palette.bg_rgb);
                } else {
                    let y1 = y0 + self.h - 1;
                    graph.line(x, y0, x + ww - 1, y0, palette.bg_rgb, None);
                    graph.line(x, y1, x + ww - 1, y1, palette.bg_rgb, None);
                }
            }
        }
    }
}

pub struct NeutralisersWidget {
    w: i32,
    h1: i32,
    h2: i32
}
impl NeutralisersWidget {
    pub fn new(w: i32, h1: i32, h2: i32) -> Self {
        Self { w, h1, h2 }
    }
}
impl Widget for NeutralisersWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        let ww = self.w / palette.n as i32;
        let wx1 = if ww <= 12 { 1 } else { 2 };
        let wx2 = if ww <= 12 { 2 } else { 3 };
        for i in 0..palette.n {
            let x = x0 + ww * i as i32;
            let i = palette.sorted[i];
            let c = palette.cam16[i];
            let j = palette.neutraliser(c);
            let c_neu = palette.cam16[j];
            let a = (c.a + c_neu.a) / 2.;
            let b = (c.b + c_neu.b) / 2.;
            let r = f32::hypot(a, b);
            if r <= 10. && i != j {
                graph.block(x + wx1, y0, ww - 2 * wx1, self.h1, palette.rgb[j]);
                graph.dither(
                    x + wx2, y0 + self.h1,
                    ww - 2 * wx2, self.h2,
                    palette.rgb[j], palette.rgb[i]
                );
            }
        }
    }
}

pub struct RGB12BitWidget {}
impl Widget for RGB12BitWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              _cacher: &mut PlotCacher,
              palette: &Palette,
              ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        for g in 0..16 {
            let x = x0 + (g % 8) * 16;
            let y = y0 + (g / 8) * 16;
            for r in 0..16 {
                for b in 0..16 {
                    let c = CAM16UCS::of(
                        RGB255::new(r as u8 * 17, g as u8 * 17, b as u8 * 17).into(),
                        ill
                    );
                    let c = palette.nearest(c);
                    graph.put_pixel(x + r, y + b, c);
                }
            }
        }
    }
}

// TODO: a filled variation?
pub struct HueChromaPolarWidget {
    d: i32
}
impl HueChromaPolarWidget {
    pub fn new(d: i32) -> Self {
        Self { d }
    }
}
impl Widget for HueChromaPolarWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              cacher: &mut PlotCacher,
              palette: &Palette,
              ill: &CAT16Illuminant,
              font: &Font,
              x0: i32, y0: i32) {
        let r = self.d / 2;
        let cx = x0 + r;
        let cy = y0 + r;
        let cross_l = 5;
        graph.circle(x0, y0, self.d, palette.bg_rgb, None);
        graph.line(cx - cross_l, cy, cx + cross_l, cy, palette.bg_rgb, None);
        graph.line(cx, cy - cross_l, cx, cy + cross_l, palette.bg_rgb, None);
        for radius in [r / 4, r / 2, r * 3 / 4] {
            graph.circle(cx - radius, cy - radius, radius * 2 + 1, palette.bg_rgb, Some(3));
        }

        let boundary = cacher.get_cam16_boundary(ill);
        let boundary_n = boundary.len();
        for i in 0..boundary_n {
            let j = (i + 1) % boundary_n;
            let a_i = (i as f32 / boundary_n as f32) * 2. * PI;
            let C_i = boundary[i];
            let a_j = (j as f32 / boundary_n as f32) * 2. * PI;
            let C_j = boundary[j];
            let x_i = cx + (C_i * r as f32 * a_i.cos()).round() as i32;
            let y_i = cy - (C_i * r as f32 * a_i.sin()).round() as i32;
            let x_j = cx + (C_j * r as f32 * a_j.cos()).round() as i32;
            let y_j = cy - (C_j * r as f32 * a_j.sin()).round() as i32;
            graph.line(x_i, y_i, x_j, y_j, palette.fg_rgb, None);
        }

        let marks = [
            (255,   0, 0, "R"),
            (255, 255, 0, "Y"),
            (0, 255,   0, "G"),
            (0, 255, 255, "C"),
            (0,   0, 255, "B"),
            (255, 0, 255, "M")
        ];
        for (rr, gg, bb, text) in marks {
            let rgb = RGB255::new(rr, gg, bb);
            let xyz = CIEXYZ::from(rgb);
            let cam16 = CAM16UCS::of(xyz, ill);
            let h = f32::atan2(cam16.b, cam16.a);
            let C = cam16.C / 100.;
            let x = cx + ((C * r as f32 + 6.) * h.cos()).round() as i32;
            let y = cy - ((C * r as f32 + 6.) * h.sin()).round() as i32;
            graph.text(text, x, y, TextAnchor::c(), font, palette.fg_rgb);
        }

        let min_dd = if palette.n <= 24 { 4 } else { 2 };
        let max_dd = match palette.n {
            0..=64 => { 8 }
            0..=128 => { 6 }
            _ => { 4 }
        };
        for i in 0..palette.n {
            let c = palette.cam16[i];
            let h = f32::atan2(c.b, c.a);
            let mut C = c.C / 100.;
            if C <= 0.1 { C = 0.; }
            let dd = 2 + min_dd + (C * (max_dd - min_dd) as f32).round() as i32;
            let x = cx + (C * r as f32 * h.cos()).round() as i32;
            let y = cy - (C * r as f32 * h.sin()).round() as i32;
            graph.disc(x - dd / 2, y - dd / 2, dd, palette.rgb[i]);
            if i == palette.bl {
                graph.circle(
                    x - dd / 2 - 1, y - dd / 2 - 1, dd + 1,
                    palette.bg_rgb, None
                );
            }
        }
    }
}

pub struct HueLightnessPolarFilledWidget {
    C: f32,
    d: i32,
    inv: bool
}
impl HueLightnessPolarFilledWidget {
    pub fn new(C: f32, d: i32, inv: bool) -> Self {
        Self { C, d, inv }
    }
}
impl Widget for HueLightnessPolarFilledWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        graph.plot_polar(
            cacher, x0, y0, self.d, self.d,
            palette, &format!("HueLightness:d={}:inv={}:C={:.2}", self.d, self.inv, self.C),
            |r, a| { Some(CAM16UCS{
                J: if !self.inv { r * 100. } else { 100. * (1. - r) },
                a: self.C * f32::cos(a * 2. * PI),
                b: self.C * f32::sin(a * 2. * PI),
                C: self.C
            })}
        );
    }
}

pub struct HueLightnessPolarFilledGroupWidget {
    C_low: f32,
    C_high: f32,
    d_small: i32,
    d_big: i32
}
impl HueLightnessPolarFilledGroupWidget {
    pub fn new(C_low: f32, C_high: f32, d_small: i32, d_big: i32) -> Self {
        Self { C_low, C_high, d_small, d_big }
    }
}
impl Widget for HueLightnessPolarFilledGroupWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              cacher: &mut PlotCacher,
              palette: &Palette,
              ill: &CAT16Illuminant,
              font: &Font,
              x0: i32, y0: i32) {
        let d_cross = (self.d_big as f32 / f32::sqrt(2.)).round() as i32;
        graph.text(
            &format!("C: {}", self.C_low.round() as i32),
            x0, y0, TextAnchor::nw(),
            font, palette.fg_rgb
        );
        let c1 = HueLightnessPolarFilledWidget::new(self.C_low, self.d_big, true);
        c1.render(graph, cacher, palette, ill, font, x0, y0);
        graph.text(
            &format!("C: {}", self.C_low.round() as i32),
            x0 + self.d_big + self.d_small, y0 + self.d_big + d_cross, TextAnchor::se(),
            font, palette.fg_rgb
        );
        let c2 = HueLightnessPolarFilledWidget::new(self.C_low, self.d_big, false);
        c2.render(graph, cacher, palette, ill, font, x0 + d_cross, y0 + d_cross);
        graph.text(
            &format!("C: {}", self.C_high.round() as i32),
            x0 + self.d_big + self.d_small, y0 + self.d_small, TextAnchor::e(),
            font, palette.fg_rgb
        );
        let c3 = HueLightnessPolarFilledWidget::new(self.C_high, self.d_small, true);
        c3.render(graph, cacher, palette, ill, font, x0 + self.d_big, y0);
        graph.text(
            &format!("C: {}", self.C_high.round() as i32),
            x0, y0 + self.d_big, TextAnchor::sw(),
            font, palette.fg_rgb
        );
        let c4 = HueLightnessPolarFilledWidget::new(self.C_high, self.d_small, false);
        c4.render(graph, cacher, palette, ill, font, x0, y0 + self.d_big);
    }
}

pub struct ComplementariesWidget {
    a: f32,
    b: f32,
    w: i32,
    h: i32
}
impl ComplementariesWidget {
    pub fn new(a: f32, b: f32, w: i32, h: i32) -> Self {
        Self { a, b, w, h }
    }
}
impl Widget for ComplementariesWidget {
    fn render(&self,
              graph: &mut ImageGraph,
              cacher: &mut PlotCacher,
              palette: &Palette,
              _ill: &CAT16Illuminant,
              _font: &Font,
              x0: i32, y0: i32) {
        let key = format!(
            "Comp:w={}:h={}:a={}:b={}",
            self.w, self.h, self.a as i32, self.b as i32
        );
        graph.plot(
            cacher, x0, y0, self.w, self.h,
            palette, &key,
            |x, y| { Some(CAM16UCS{
                J: (x + y) / 2. * 100.,
                a: (y - x) * self.a,
                b: (y - x) * self.b,
                C: 0. // C isn't used so just zero
            })}
        );
    }
}
