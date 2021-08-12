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

#[cfg(not(target_arch = "wasm32"))]
use clap::{Arg, App};

#[cfg(target_arch = "wasm32")]
use stdweb;

use crate::text::Font;
use crate::cache::PlotCacher;
use crate::analyse::*;
use crate::loader::*;

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
    let colours = match result {
        Ok(x) => { x }
        Err(e) => {
            eprintln!("Error while getting palette: {:?}", e);
            return;
        }
    };
    match check_palette(&colours) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error while validating palette: {:?}", e);
            return;
        }
    }

    let font = Font::new();
    let mut cacher = PlotCacher::new();
    let T = 5500.;

    analyse(&colours, T, &mut cacher, &font, "output".into(), true);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let matches = App::new("censor")
        .version(metadata::VERSION)
        .about("Palette analysis tool.")
        .arg(
            Arg::with_name("daemon")
                .short("d")
                .long("daemon")
                .value_name("PORT")
                .help("Starts in daemon mode on TCP port PORT")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Prints debugging output")
        )
        .arg(
            Arg::with_name("colours")
                .short("c")
                .long("colours")
                .value_name("LIST")
                .help("Sets input colours to the specified list of comma-separated hex values")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("hexfile")
                .short("f")
                .long("hexfile")
                .value_name("FILE")
                .help("Reads input colours from the specified file with newline-separated hex values")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("imagefile")
                .short("i")
                .long("image")
                .value_name("FILE")
                .help("Reads input colours from the specified image")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("lospec")
                .short("l")
                .long("lospec")
                .value_name("SLUG")
                .help("Loads input colours from https://lospec.com/palette-list/SLUG")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("outfile")
                .short("o")
                .long("out")
                .value_name("FILE")
                .help("Sets output image file; default: plot.png")
                .takes_value(true)
        )
        .get_matches();

    let list_provided = matches.value_of("colours").is_some();
    let file_provided = matches.value_of("hexfile").is_some();
    let slug_provided = matches.value_of("lospec").is_some();
    let image_provided = matches.value_of("imagefile").is_some();

    let verbose = matches.is_present("verbose");

    let daemon = matches.value_of("daemon").is_some();
    if daemon {
        if list_provided || file_provided || slug_provided || image_provided {
            eprintln!("Daemon mode conflicts with input sources.");
            std::process::exit(1);
        }
        let port_str = matches.value_of("daemon").unwrap();
        let port = match u16::from_str_radix(port_str, 10) {
            Ok(port) => { port }
            Err(e) => {
                eprintln!("Error parsing daemon port: {:?}", e);
                std::process::exit(1);
            }
        };
        match daemon::run(port, verbose) {
            Ok(()) => { std::process::exit(0); }
            Err(e) => {
                eprintln!("Daemon error: {:?}", e);
                std::process::exit(1);
            }
        }
    }

    let mut outfile: String = matches.value_of("outfile").unwrap_or("plot.png").into();
    if !outfile.ends_with(".png") {
        outfile = format!("{}.png", outfile);
    }

    let font = Font::new();
    let mut cacher = PlotCacher::new();
    let T = 5500.;

    let result;

    match (list_provided, file_provided, slug_provided, image_provided) {
        (false, false, false, false) => {
            eprintln!("{}", matches.usage());
            std::process::exit(1);
        }
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
            eprintln!("Two or more conflicting inputs were specified, aborting...");
            std::process::exit(1);
        }
    }
    let colours = match result {
        Ok(x) => { x }
        Err(e) => {
            eprintln!("Error while getting palette: {:?}", e);
            std::process::exit(1);
        }
    };
    match check_palette(&colours) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error while validating palette: {:?}", e);
            std::process::exit(1);
        }
    }
    analyse(&colours, T, &mut cacher, &font, outfile, verbose);
}
