#[cfg(not(target_arch = "wasm32"))]
use ureq;
use img_parts::{png::Png, jpeg::Jpeg, ImageICC};

#[cfg(not(target_arch = "wasm32"))]
use image::io::Reader as ImageReader;

use crate::colour::*;
use crate::metadata;

use std::collections::HashSet;
use std::iter::FromIterator;
#[cfg(not(target_arch = "wasm32"))]
use std::io::prelude::*;

#[derive(Clone, Copy, Debug)]
pub enum PaletteCheckError {
    TooFewColours(usize),
    TooManyColours(usize),
    Duplicates
}
impl std::fmt::Display for PaletteCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFewColours(n) => { write!(f, "Too few colours: {}", n) }
            Self::TooManyColours(n) => { write!(f, "Too many colours: {}", n) }
            Self::Duplicates => { write!(f, "Duplicated colours") }
        }
    }
}

pub fn check_palette(palette: &Vec<RGB255>) -> Result<(), PaletteCheckError> {
    let n = palette.len();
    if n < 2 {
        return Err(PaletteCheckError::TooFewColours(n));
    }
    if n > 256 {
        return Err(PaletteCheckError::TooManyColours(n));
    }
    let set: HashSet<RGB255> = HashSet::from_iter(palette.clone());
    let m = set.len();
    if m < n {
        return Err(PaletteCheckError::Duplicates);
    }
    return Ok(());
}

#[derive(Debug)]
pub enum LoadError {
    InvalidHexLength,
    NonHexCharacters,
#[cfg(not(target_arch = "wasm32"))]
    FileOpen(std::io::Error),
#[cfg(not(target_arch = "wasm32"))]
    FileRead(std::io::Error),
#[cfg(not(target_arch = "wasm32"))]
    NetworkError(ureq::Error),
#[cfg(not(target_arch = "wasm32"))]
    InvalidEncoding(std::io::Error),
#[cfg(not(target_arch = "wasm32"))]
    ImageEncoding(image::ImageError),
#[cfg(not(target_arch = "wasm32"))]
    NotFound
}
impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHexLength => { write!(f, "Invalid hex colour length") }
            Self::NonHexCharacters => { write!(f, "Invalid characters in hex colour") }
            Self::FileOpen(ref e) => { e.fmt(f) }
            Self::FileRead(ref e) => { e.fmt(f) }
            Self::NetworkError(ref e) => { e.fmt(f) }
            Self::InvalidEncoding(ref e) => { e.fmt(f) }
            Self::ImageEncoding(ref e) => { e.fmt(f) }
            Self::NotFound => { write!(f, "Palette not found") }
        }
    }
}

pub struct LoadedImage {
    pub data: Vec<Vec<Option<RGB255>>>,
    pub icc_profile: Option<img_parts::Bytes>
}
impl LoadedImage {
    pub fn new(data: Vec<Vec<Option<RGB255>>>) -> Self {
        Self { data, icc_profile: None }
    }
    pub fn with_icc_profile(self, profile: img_parts::Bytes) -> Self {
        Self {
            data: self.data,
            icc_profile: Some(profile)
        }
    }
}

pub fn load_image(filename: String) -> Result<LoadedImage, LoadError> {
    let image = ImageReader::open(&filename)
        .map_err(|e| LoadError::FileOpen(e))?
        .decode().map_err(|e| LoadError::ImageEncoding(e))?
        .to_rgba8();
    let w = image.width();
    let h = image.height();
    let mut data = vec![vec![None; w as usize]; h as usize];
    for y in 0..h {
        for x in 0..w {
            let c = image.get_pixel(x, y);
            let [r, g, b, a] = c.0;
            if a == 0xff {
                let c = RGB255::new(r, g, b);
                data[y as usize][x as usize] = Some(c);
            }
        }
    }

    let mut icc_profile = None;
    if filename.ends_with(".png") {
        if let Ok(data) = std::fs::read(&filename) {
            if let Ok(png) = Png::from_bytes(data.into()) {
                icc_profile = png.icc_profile();
            }
        }
    }
    if filename.ends_with(".jpg") || filename.ends_with("jpeg") {
        if let Ok(data) = std::fs::read(&filename) {
            if let Ok(jpeg) = Jpeg::from_bytes(data.into()) {
                icc_profile = jpeg.icc_profile();
            }
        }
    }
    
    let mut image = LoadedImage::new(data);
    if let Some(profile) = icc_profile {
        image = image.with_icc_profile(profile);
    }
    return Ok(image);
}

#[derive(Clone)]
pub struct LoadedPalette {
    pub colours: Vec<RGB255>,
    pub icc_profile: Option<img_parts::Bytes>
}
impl LoadedPalette {
    pub fn new(colours: Vec<RGB255>) -> Self {
        Self { colours, icc_profile: None }
    }
    pub fn with_icc_profile(self, profile: img_parts::Bytes) -> Self {
        Self {
            colours: self.colours,
            icc_profile: Some(profile)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_from_image(filename: String) -> Result<LoadedPalette, LoadError> {
    let image = ImageReader::open(&filename)
        .map_err(|e| LoadError::FileOpen(e))?
        .decode().map_err(|e| LoadError::ImageEncoding(e))?
        .to_rgba8();
    let w = image.width();
    let h = image.height();
    let mut colours = vec![];
    for y in 0..h {
        for x in 0..w {
            let c = image.get_pixel(x, y);
            let [r, g, b, a] = c.0;
            if a == 0xff {
                let c = RGB255::new(r, g, b);
                if !colours.contains(&c) {
                    colours.push(c);
                }
            }
        }
    }

    let mut icc_profile = None;
    if filename.ends_with(".png") {
        if let Ok(data) = std::fs::read(&filename) {
            if let Ok(png) = Png::from_bytes(data.into()) {
                icc_profile = png.icc_profile();
            }
        }
    }
    if filename.ends_with(".jpg") || filename.ends_with("jpeg") {
        if let Ok(data) = std::fs::read(&filename) {
            if let Ok(jpeg) = Jpeg::from_bytes(data.into()) {
                icc_profile = jpeg.icc_profile();
            }
        }
    }

    let mut palette = LoadedPalette::new(colours);
    if let Some(profile) = icc_profile {
        palette = palette.with_icc_profile(profile);
    }

    return Ok(palette);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_from_lospec(slug: String) -> Result<LoadedPalette, LoadError> {
    let url = format!("https://lospec.com/palette-list/{}.csv", slug);
    let csv = ureq::get(&url)
        .set("User-Agent", &format!("censor v{}", metadata::VERSION))
        .call().map_err(|e| LoadError::NetworkError(e))?
        .into_string().map_err(|e| LoadError::InvalidEncoding(e))?;
    if csv == "file not found" {
        return Err(LoadError::NotFound);
    }
    let colours = csv.split(',')
        .skip(2)
        .map(|s| parse_hex(s.into()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LoadedPalette::new(colours))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_from_file(filename: String) -> Result<LoadedPalette, LoadError> {
    let mut colours = vec![];
    let file = std::fs::File::open(filename).map_err(|e| LoadError::FileOpen(e))?;
    let reader = std::io::BufReader::new(file);
    for line in reader.lines() {
        let line = line.map_err(|e| LoadError::FileRead(e))?;
        let c = parse_hex(line)?;
        colours.push(c);
    }
    Ok(LoadedPalette::new(colours))
}

pub fn load_from_hex(data: &Vec<String>) -> Result<LoadedPalette, LoadError> {
    let colours = data.iter()
        .map(|s| parse_hex(s.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LoadedPalette::new(colours))
}

fn parse_hex(x: String) -> Result<RGB255, LoadError> {
    if x.len() < 6 || x.len() > 7 || (x.len() == 7 && !x.starts_with('#')) {
        return Err(LoadError::InvalidHexLength);
    }
    let mut iter = x.chars().skip_while(|&c| c == '#');
    let r = vec![iter.next().unwrap(), iter.next().unwrap()].into_iter().collect::<String>();
    let g = vec![iter.next().unwrap(), iter.next().unwrap()].into_iter().collect::<String>();
    let b = vec![iter.next().unwrap(), iter.next().unwrap()].into_iter().collect::<String>();
    let r = u8::from_str_radix(&r, 16).map_err(|_| LoadError::NonHexCharacters)?;
    let g = u8::from_str_radix(&g, 16).map_err(|_| LoadError::NonHexCharacters)?;
    let b = u8::from_str_radix(&b, 16).map_err(|_| LoadError::NonHexCharacters)?;
    return Ok(RGB255::new(r, g, b));
}
