#![allow(non_snake_case)]

mod util;
mod colour;
mod palette;
mod text;
mod cache;
mod graph;
mod widget;
mod analyse;
mod loader;
#[cfg(not(target_arch = "wasm32"))]
mod daemon;
mod web;
mod metadata;
mod dither;

#[cfg(target_arch = "wasm32")]
use stdweb;

use image::RgbImage;
use img_parts::{png::Png, ImageICC};
use text_io::scan;

use crate::colour::*;
use crate::palette::*;
use crate::text::Font;
use crate::cache::*;
use crate::analyse::*;
use crate::loader::*;
use crate::dither::*;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::rc::Rc;

// TODO: WASM integration
// TODO: colour blindness widgets!
// TODO: image from URL loader

#[cfg(target_arch = "wasm32")]
fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    stdweb::initialize();
    let hex_list = match web::read_storage("input") {
        Some(hex_list) => { hex_list }
        None => {
            eprintln!("Analyser: couldn't read `input` from session storage");
            return;
        }
    };
    let hex_list: Vec<String> = hex_list.split(',')
        .map(|s| String::from(s))
        .collect();
    let result = load_from_hex(&hex_list);
    let palette = match result {
        Ok(x) => { x }
        Err(e) => {
            eprintln!("Error while getting palette: {}", e);
            return;
        }
    };
    match check_palette(&palette.colours) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error while validating palette: {}", e);
            return;
        }
    }

    let font = Font::new();
    let mut cacher = BigCacher::init(false);
    let T = 5500.;
    let cache = cacher.at(T);

    let grey_ui = false;

    analyse(&palette, T, cache, &font, grey_ui, "output".into(), true);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let app = metadata::cmd_parser();
    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("analyse") {
        main_analyse(matches);
        return;
    }
    if let Some(matches) = matches.subcommand_matches("daemon") {
        main_daemon(matches);
        return;
    }
    if let Some(matches) = matches.subcommand_matches("compute") {
        main_compute(matches);
        return;
    }
    if let Some(matches) = matches.subcommand_matches("dither") {
        main_dither(matches);
        return;
    }
    eprintln!("Usage information:");
    eprintln!("\tcensor --help");
    std::process::exit(1);
}

fn palette_from_cmd<'a>(matches: &clap::ArgMatches<'a>, verbose: bool)
            -> LoadedPalette {
    let list_provided = matches.value_of("colours").is_some();
    let file_provided = matches.value_of("hexfile").is_some();
    let slug_provided = matches.value_of("lospec").is_some();
    let image_provided = matches.value_of("imagefile").is_some();

    let result;

    match (list_provided, file_provided, slug_provided, image_provided) {
        (true, false, false, false) => {
            let hex_list = matches.value_of("colours").unwrap();
            let hex_list = hex_list.split(',')
                .map(|s| String::from(s))
                .collect::<Vec<_>>();
            result = load_from_hex(&hex_list);
        }
        (false, true, false, false) => {
            let filename = matches.value_of("hexfile").unwrap();
            result = load_from_file(filename.into());
        }
        (false, false, true, false) => {
            let slug = matches.value_of("lospec").unwrap();
            if verbose { eprintln!("Downloading palette..."); }
            result = load_from_lospec(slug.into());
        }
        (false, false, false, true) => {
            let filename = matches.value_of("imagefile").unwrap();
            result = load_from_image(filename.into());
        }
        _ => {
            eprintln!("Impossible happened! Blame the `clap` library. Report this error.");
            std::process::exit(1);
        }
    }
    let palette = match result {
        Ok(x) => { x }
        Err(e) => {
            eprintln!("Error while getting palette: {}", e);
            std::process::exit(1);
        }
    };
    return palette;
}

fn main_analyse<'a>(matches: &clap::ArgMatches<'a>) {
    let verbose = matches.is_present("verbose");
    let grey_ui = matches.is_present("grey_ui");
    let multithreaded = matches.is_present("multithreaded");

    let mut outfile: String = matches.value_of("outfile").unwrap_or("plot.png").into();
    if !outfile.ends_with(".png") {
        outfile = format!("{}.png", outfile);
    }

    let font = Font::new();
    let mut cacher = BigCacher::init(verbose);
    let T: f32;
    if let Some(D) = matches.value_of("D") {
        match D {
            "50" => { T = 5000.00 }
            "55" => { T = 5500.00 }
            "65" => { T = 6503.51 }
            _ => {
                eprintln!("Invalid illuminant preset: D{}", D);
                std::process::exit(1);
            }
        }
    } else {
        T = match str::parse(matches.value_of("T").unwrap_or("5500")) {
            Ok(x) => { x }
            Err(e) => {
                eprintln!("Error parsing temperature: {}", e);
                std::process::exit(1);
            }
        };
    }
    let ill = CAT16Illuminant::new(CIExy::from_T(T));

    let palette = palette_from_cmd(matches, verbose);

    match check_palette(&palette.colours) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error while validating palette: {}", e);
            std::process::exit(1);
        }
    }

    if !multithreaded {
        let cache_provider = SinglethreadedCacheProvider::new(T, &ill, &mut cacher);
        let cache = Rc::new(RwLock::new(cache_provider));
        analyse_singlethreaded(&palette, T, cache, Rc::new(font), grey_ui, outfile, verbose);
    } else {
        let mut cache_hoster = CacheHoster::new(&mut cacher);
        let (cp_req_send, cp_req_recv) = crossbeam_channel::bounded(0);
        let (cp_send, cp_recv) = crossbeam_channel::bounded(0);
        let handle = std::thread::spawn(move || {
            analyse_multithreaded(
                &palette, T, cp_req_send, cp_recv,
                Arc::new(font), grey_ui, outfile, verbose
            );
        });
        loop {
            match cp_req_recv.recv() {
                Ok(()) => {
                    let (recv, send) = cache_hoster.register();
                    let cache_provider = MultithreadedCacheProvider::new(T, ill, send, recv);
                    cp_send.send(cache_provider).unwrap();
                }
                Err(_) => { break; }
            }
        }
        cache_hoster.process();
        handle.join().unwrap();
    }

    if let Err(e) = cacher.save() {
        if verbose {
            eprintln!("Error saving cache: {}", e);
        }
    }
}

fn main_daemon<'a>(matches: &clap::ArgMatches<'a>) {
    let verbose = matches.is_present("verbose");

    let port_str = matches.value_of("port").unwrap();
    let port = match u16::from_str_radix(port_str, 10) {
        Ok(port) => { port }
        Err(e) => {
            eprintln!("Error parsing daemon port: {}", e);
            std::process::exit(1);
        }
    };
    match daemon::run(port, verbose) {
        Ok(()) => { std::process::exit(0); }
        Err(e) => {
            eprintln!("Daemon error: {}", e);
            std::process::exit(1);
        }
    }
}

fn main_compute<'a>(matches: &clap::ArgMatches<'a>) {
    let T: f32;
    if let Some(D) = matches.value_of("D") {
        match D {
            "50" => { T = 5000.00 }
            "55" => { T = 5500.00 }
            "65" => { T = 6503.51 }
            _ => {
                eprintln!("Invalid illuminant preset: D{}", D);
                std::process::exit(1);
            }
        }
    } else {
        T = match str::parse(matches.value_of("T").unwrap_or("5500")) {
            Ok(x) => { x }
            Err(e) => {
                eprintln!("Error parsing temperature: {}", e);
                std::process::exit(1);
            }
        };
    }
    let ill = CAT16Illuminant::new(CIExy::from_T(T));

    let palette = palette_from_cmd(matches, false);
    let palette = Palette::new(palette.colours.clone(), &ill, false);

    let metrics = ["iss", "acyclic"];

    let mut enabled = HashMap::<&str, bool>::new();
    for metric in metrics {
        enabled.insert(metric, matches.is_present(metric));
    }
    if matches.is_present("all") {
        for metric in metrics {
            enabled.insert(metric, true);
        }
    }

    for metric in metrics {
        if enabled[metric] {
            let v: String;
            match metric {
                "iss" => {
                    let iss = palette.internal_similarity();
                    v = format!("{:.2}", iss);
                }
                "acyclic" => {
                    let acyclic = palette.is_acyclic();
                    v = format!("{}", acyclic);
                }
                _ => { continue; }
            };
            println!("{},{}", metric, v);
        }
    }
}

fn main_dither<'a>(matches: &clap::ArgMatches<'a>) {
    let verbose = matches.is_present("verbose");

    let T: f32;
    if let Some(D) = matches.value_of("D") {
        match D {
            "50" => { T = 5000.00 }
            "55" => { T = 5500.00 }
            "65" => { T = 6503.51 }
            _ => {
                eprintln!("Invalid illuminant preset: D{}", D);
                std::process::exit(1);
            }
        }
    } else {
        T = match str::parse(matches.value_of("T").unwrap_or("5500")) {
            Ok(x) => { x }
            Err(e) => {
                eprintln!("Error parsing temperature: {}", e);
                std::process::exit(1);
            }
        };
    }
    let ill = CAT16Illuminant::new(CIExy::from_T(T));

    let mut outfile: String = matches.value_of("outfile").unwrap_or("plot.png").into();
    if !outfile.ends_with(".png") {
        outfile = format!("{}.png", outfile);
    }

    let palette = palette_from_cmd(matches, verbose);
    let palette = Palette::new(palette.colours.clone(), &ill, false);

    let image_filename = matches.value_of("imageinput").unwrap();
    let image = match load_image(image_filename.into()) {
        Ok(x) => { x }
        Err(e) => {
            eprintln!("Error loading input image: {}", e);
            std::process::exit(1);
        }
    };
    let h = image.data.len() as u32;
    let w = image.data[0].len() as u32;

    if verbose { eprintln!("Converting the image into CAM16UCS...") }
    let icc_profile = image.icc_profile;
    let image_cam16: Vec<Vec<Option<CAM16UCS>>> = image.data.iter().map(
        |row| row.iter().map(
            |opt| opt.map(
                |rgb| CAM16UCS::of(CIEXYZ::from(rgb), &ill)
            )
        ).collect()
    ).collect();
    let plot = PlotData::new(image_cam16);

    let nodither_provided = matches.is_present("nodither");
    let bayer_provided = matches.is_present("bayer");
    let whitenoise_provided = matches.is_present("whitenoise");
    let bluenoise_provided = matches.is_present("bluenoise");

    let method = match () {
        () if nodither_provided => { DitheringMethod::None }
        () if bayer_provided => {
            let n = match str::parse(matches.value_of("bayer").unwrap()) {
                Ok(x) => { x }
                Err(e) => {
                    eprintln!("Could not parse Bayer matrix size: {}", e);
                    std::process::exit(1);
                }
            };
            DitheringMethod::Bayer(n)
        }
        () if whitenoise_provided => {
            let wxh = matches.value_of("whitenoise").unwrap();
            let w: usize;
            let h: usize;
            scan!(wxh.bytes() => "{}x{}", w, h);
            DitheringMethod::WhiteNoise(w, h)
        }
        () if bluenoise_provided => {
            let wxh = matches.value_of("bluenoise").unwrap();
            let w: usize;
            let h: usize;
            scan!(wxh.bytes() => "{}x{}", w, h);
            DitheringMethod::BlueNoise(w, h)
        }
        () => { DitheringMethod::default() }
    };

    let dithered = Ditherer::dither(plot, &palette, method, verbose);

    let mut image = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            match dithered.data[y as usize][x as usize] {
                Some(rgb) => {
                    image.put_pixel(x, y, rgb.into());
                }
                None => {}
            }
        }
    }
    if let Err(e) = image.save(&outfile) {
        eprintln!("Error saving output image: {}", e);
        std::process::exit(1);
    }

    if let Some(ref icc_profile) = icc_profile {
        let data = match std::fs::read(&outfile) {
            Ok(x) => { x }
            Err(_) => { return; }
        };
        let mut png = match Png::from_bytes(data.into()) {
            Ok(x) => { x }
            Err(_) => { return; }
        };
        png.set_icc_profile(Some(icc_profile.clone()));
        let file = match std::fs::File::create(&outfile) {
            Ok(x) => { x }
            Err(_) => { return; }
        };
        let _ = png.encoder().write_to(file);
    }
}
