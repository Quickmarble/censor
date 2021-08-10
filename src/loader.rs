use ureq;
use image::io::Reader as ImageReader;

use crate::colour::*;

use std::collections::HashSet;
use std::iter::FromIterator;
use std::io::prelude::*;

#[derive(Clone, Copy, Debug)]
pub enum PaletteCheckError {
    TooFewColours(usize),
    TooManyColours(usize),
    Duplicates
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
    FileOpen(std::io::Error),
    FileRead(std::io::Error),
    NetworkError(ureq::Error),
    InvalidEncoding(std::io::Error),
    ImageEncoding(image::ImageError),
    NotFound
}

pub fn load_from_image(filename: String) -> Result<Vec<RGB255>, LoadError> {
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
    return Ok(colours);
}

pub fn load_from_lospec(slug: String) -> Result<Vec<RGB255>, LoadError> {
    let url = format!("https://lospec.com/palette-list/{}.csv", slug);
    let csv = ureq::get(&url)
        .set("User-Agent", "censor v0.2.0")
        .call().map_err(|e| LoadError::NetworkError(e))?
        .into_string().map_err(|e| LoadError::InvalidEncoding(e))?;
    if csv == "file not found" {
        return Err(LoadError::NotFound);
    }
    let colours = csv.split(',')
        .skip(2)
        .map(|s| parse_hex(s.into()))
        .collect::<Result<Vec<_>, _>>();
    return colours;
}

pub fn load_from_file(filename: String) -> Result<Vec<RGB255>, LoadError> {
    let mut colours = vec![];
    let file = std::fs::File::open(filename).map_err(|e| LoadError::FileOpen(e))?;
    let reader = std::io::BufReader::new(file);
    for line in reader.lines() {
        let c = parse_hex(line.map_err(|e| LoadError::FileRead(e))?.clone())?;
        colours.push(c);
    }
    return Ok(colours);
}

pub fn load_from_hex(data: &Vec<String>) -> Result<Vec<RGB255>, LoadError> {
    data.iter()
        .map(|s| parse_hex(s.clone()))
        .collect::<Result<Vec<_>, _>>()
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
