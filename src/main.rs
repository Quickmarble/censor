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
use clap::{Arg, App, SubCommand, ArgGroup};

#[cfg(target_arch = "wasm32")]
use stdweb;

use crate::colour::*;
use crate::palette::*;
use crate::text::Font;
use crate::cache::PlotCacher;
use crate::analyse::*;
use crate::loader::*;

use std::collections::HashMap;

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

    let grey_ui = false;

    analyse(&colours, T, &mut cacher, &font, grey_ui, "output".into(), true);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let app = App::new("censor")
        .version(metadata::VERSION)
        .about("Palette analysis tool.")
        .subcommand(SubCommand::with_name("daemon")
            .about("Starts in daemon mode.")
            .arg(
                Arg::with_name("verbose")
                    .short("v")
                    .long("verbose")
                    .help("Prints debugging output")
            )
            .arg(
                Arg::with_name("port")
                    .short("p")
                    .long("port")
                    .value_name("PORT")
                    .help("The port exposed by the daemon")
                    .takes_value(true)
                    .required(true)
            )
        )
        .subcommand(SubCommand::with_name("analyse")
            .about("Produces a plot with palette analysis.")
            .arg(
                Arg::with_name("verbose")
                    .short("v")
                    .long("verbose")
                    .help("Prints debugging output")
            )
            .arg(
                Arg::with_name("grey_ui")
                    .short("g")
                    .long("grey")
                    .help("Uses black, grey and white for UI instead of choosing palette colours")
            )
            .group(ArgGroup::with_name("input")
                .multiple(false)
                .required(true)
                .args(&["colours", "hexfile", "imagefile", "lospec"])
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
        )
        .subcommand(SubCommand::with_name("compute")
            .about("Computes palette metrics.")
            .group(ArgGroup::with_name("input")
                .multiple(false)
                .required(true)
                .args(&["colours", "hexfile", "imagefile", "lospec"])
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
            .group(ArgGroup::with_name("metrics")
                .multiple(true)
                .required(true)
                .args(&["all", "iss", "acyclic"])
            )
            .arg(
                Arg::with_name("all")
                    .short("a")
                    .long("all")
                    .help("Computes all the metrics")
            )
            .arg(
                Arg::with_name("iss")
                    .long("iss")
                    .help("Computes internal similarity score")
            )
            .arg(
                Arg::with_name("acyclic")
                    .long("acyclic")
                    .help("Checks is a palette is acyclic")
            )
        );
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
    eprintln!("Usage information:");
    eprintln!("\tcensor --help");
    std::process::exit(1);
}

fn palette_from_cmd<'a>(matches: &clap::ArgMatches<'a>, verbose: bool) -> Vec<RGB255> {
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
    let colours = match result {
        Ok(x) => { x }
        Err(e) => {
            eprintln!("Error while getting palette: {:?}", e);
            std::process::exit(1);
        }
    };
    return colours;
}

fn main_analyse<'a>(matches: &clap::ArgMatches<'a>) {
    let verbose = matches.is_present("verbose");
    let grey_ui = matches.is_present("grey_ui");

    let mut outfile: String = matches.value_of("outfile").unwrap_or("plot.png").into();
    if !outfile.ends_with(".png") {
        outfile = format!("{}.png", outfile);
    }

    let font = Font::new();
    let mut cacher = PlotCacher::new();
    let T = 5500.;

    let colours = palette_from_cmd(matches, verbose);

    match check_palette(&colours) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error while validating palette: {:?}", e);
            std::process::exit(1);
        }
    }

    analyse(&colours, T, &mut cacher, &font, grey_ui, outfile, verbose);
}

fn main_daemon<'a>(matches: &clap::ArgMatches<'a>) {
    let verbose = matches.is_present("verbose");

    let port_str = matches.value_of("port").unwrap();
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

fn main_compute<'a>(matches: &clap::ArgMatches<'a>) {
    let T = 5500.;
    let ill = CAT16Illuminant::new(CIExy::from_T(T));

    let colours = palette_from_cmd(matches, false);
    let palette = Palette::new(colours.clone(), &ill, false);

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
