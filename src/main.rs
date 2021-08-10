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

use clap::{Arg, App};

use crate::text::Font;
use crate::cache::PlotCacher;
use crate::analyse::*;
use crate::loader::*;

// TODO: WASM integration
// TODO: colour blindness widgets!
// TODO: image from URL loader
// TODO: daemon mode

fn main() {
    let matches = App::new("censor")
        .version("0.2.0")
        .about("Palette analysis tool.")
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
            eprintln!("No inputs were specified, aborting...");
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
    analyse(&colours, T, &mut cacher, &font, outfile);
}
