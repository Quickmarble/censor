use json;

use crate::graph::{GraphPixel, PixelWriter};

const DATA: &str = include_str!("../assets/font.json");
const OK_GLYPH_DATA: [[i32; 7]; 7] = [
    [0, 0, 0, 0, 0, 1, 0],
    [0, 0, 0, 0, 1, 1, 0],
    [0, 0, 0, 0, 1, 1, 0],
    [1, 0, 0, 1, 1, 0, 0],
    [1, 1, 0, 1, 1, 0, 0],
    [0, 1, 1, 1, 0, 0, 0],
    [0, 0, 1, 1, 0, 0, 0]
];
const WARN_GLYPH_DATA: [[i32; 7]; 7] = [
    [0, 0, 0, 1, 0, 0, 0],
    [0, 0, 1, 1, 1, 0, 0],
    [0, 0, 1, 1, 1, 0, 0],
    [0, 1, 1, 1, 1, 1, 0],
    [0, 1, 1, 1, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1]
];
const ALERT_GLYPH_DATA: [[i32; 7]; 7] = [
    [1, 1, 0, 0, 0, 1, 1],
    [1, 1, 1, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 0],
    [0, 0, 1, 1, 1, 0, 0],
    [0, 1, 1, 1, 1, 1, 0],
    [1, 1, 1, 0, 1, 1, 1],
    [1, 1, 0, 0, 0, 1, 1]
];

#[derive(Clone)]
pub struct Font {
    data: json::JsonValue,
    pub ok: json::JsonValue,
    pub warn: json::JsonValue,
    pub alert: json::JsonValue
}
impl Font {
    pub fn new() -> Self {
        let parsed: Result<json::JsonValue, json::JsonError> = json::parse(DATA);
        let data = parsed.unwrap();
        let ok = Self::convert_glyph(&OK_GLYPH_DATA);
        let warn = Self::convert_glyph(&WARN_GLYPH_DATA);
        let alert = Self::convert_glyph(&ALERT_GLYPH_DATA);
        Self { data, ok, warn, alert }
    }
    fn convert_glyph(data: &[[i32; 7]; 7]) -> json::JsonValue {
        let mut rows = json::JsonValue::Array(vec![]);
        for y in 0..7 {
            let mut row = json::JsonValue::Array(vec![]);
            for x in 0..7 {
                row.push(data[y][x]).unwrap();
            }
            rows.push(row).unwrap();
        }
        return rows;
    }
    pub fn get_glyph(&self, c: char) -> &json::JsonValue {
        let k = &format!("{}", c);
        if self.data["special"].has_key(k) {
            return &self.data["special"][k]["data"];
        }
        if self.data["data"].has_key(k) {
            return &self.data["data"][k];
        }
        return &self.data["data"]["?"];
    }
    pub fn render_glyph<T: GraphPixel, W: PixelWriter<T>>
            (&self, w: &mut W, x0: i32, y0: i32, glyph: &json::JsonValue, c: T) {
        let mut dx: i32;
        let mut dy: i32 = 0;
        for row in glyph.members() {
            dx = 0;
            for v in row.members() {
                let x = x0 + dx;
                let y = y0 + dy;
                let v = v.as_i32().unwrap();
                if v == 1 {
                    w.put_pixel(x, y, c);
                }
                dx += 1;
            }
            dy += 1;
        }
    }
    pub fn render_string<T: GraphPixel, W: PixelWriter<T>>
            (&self, w: &mut W, x0: i32, y0: i32, s: &str, c: T) {
        let mut x = x0;
        for ch in s.chars() {
            let k = &format!("{}", ch);
            if self.data["special"].has_key(k) {
                let desc = &self.data["special"][k];
                let x_kern = desc["x_kern"].as_i32().unwrap_or(0);
                let y_kern = desc["y_kern"].as_i32().unwrap_or(0);
                let glyph = &desc["data"];
                self.render_glyph(w, x + x_kern, y0 - y_kern, glyph, c);
            } else {
                let glyph = self.get_glyph(ch);
                self.render_glyph(w, x, y0, glyph, c);
            }
            x += 1 + self.char_width(ch);
        }
    }
    pub fn char_width(&self, c: char) -> i32 {
        if self.data["special"].has_key(&format!("{}", c)) {
            let desc = &self.data["special"][&format!("{}", c)];
            let w = desc["data"][0].len() as i32;
            return w;
        } else {
            let w: i32 = self.data["w"].as_i32().unwrap();
            return w;
        }
    }
    pub fn char_height(&self, c: char) -> i32 {
        let h_std: i32 = self.data["h"].as_i32().unwrap();
        if self.data["special"].has_key(&format!("{}", c)) {
            let desc = &self.data["special"][&format!("{}", c)];
            if desc.has_key("y_kern") {
                let y_kern = desc["y_kern"].as_i32().unwrap();
                let h = h_std + y_kern;
                return h;
            }
        }
        return h_std;
    }
    pub fn str_width(&self, s: &str) -> i32 {
        let n = s.len() as i32;
        let mut w = i32::max(n - 1, 0);
        for c in s.chars() {
            w += self.char_width(c);
        }
        return w;
    }
    pub fn str_height(&self, s: &str) -> i32 {
        let mut h = 0;
        for c in s.chars() {
            h = i32::max(h, self.char_height(c));
        }
        return h;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HorizontalTextAnchor {
    Left, Center, Right
}
impl HorizontalTextAnchor {
    pub fn align(self, w: i32) -> i32 {
        match self {
            HorizontalTextAnchor::Left => { 0 }
            HorizontalTextAnchor::Center => { -w / 2 }
            HorizontalTextAnchor::Right => { -w }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VerticalTextAnchor {
    Top, Center, Bottom
}
impl VerticalTextAnchor {
    pub fn align(self, h: i32) -> i32 {
        match self {
            VerticalTextAnchor::Top => { 0 }
            VerticalTextAnchor::Center => { -h / 2 }
            VerticalTextAnchor::Bottom => { -h }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TextAnchor {
    pub horizontal: HorizontalTextAnchor,
    pub vertical: VerticalTextAnchor
}
#[allow(dead_code)]
impl TextAnchor {
    pub fn align(self, w: i32, h: i32) -> (i32, i32) {
        (self.horizontal.align(w), self.vertical.align(h))
    }
    pub fn nw() -> Self {
        Self {
            horizontal: HorizontalTextAnchor::Left,
            vertical: VerticalTextAnchor::Top
        }
    }
    pub fn n() -> Self {
        Self {
            horizontal: HorizontalTextAnchor::Center,
            vertical: VerticalTextAnchor::Top
        }
    }
    pub fn ne() -> Self {
        Self {
            horizontal: HorizontalTextAnchor::Right,
            vertical: VerticalTextAnchor::Top
        }
    }
    pub fn w() -> Self {
        Self {
            horizontal: HorizontalTextAnchor::Left,
            vertical: VerticalTextAnchor::Center
        }
    }
    pub fn c() -> Self {
        Self {
            horizontal: HorizontalTextAnchor::Center,
            vertical: VerticalTextAnchor::Center
        }
    }
    pub fn e() -> Self {
        Self {
            horizontal: HorizontalTextAnchor::Right,
            vertical: VerticalTextAnchor::Center
        }
    }
    pub fn sw() -> Self {
        Self {
            horizontal: HorizontalTextAnchor::Left,
            vertical: VerticalTextAnchor::Bottom
        }
    }
    pub fn s() -> Self {
        Self {
            horizontal: HorizontalTextAnchor::Center,
            vertical: VerticalTextAnchor::Bottom
        }
    }
    pub fn se() -> Self {
        Self {
            horizontal: HorizontalTextAnchor::Right,
            vertical: VerticalTextAnchor::Bottom
        }
    }
}
