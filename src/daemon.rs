use escape_string;

use crate::text::Font;
use crate::cache::*;
use crate::analyse::*;
use crate::loader::*;
use crate::colour::*;
use crate::palette::*;
use crate::metadata;

use std::io::{BufRead, Write};
use std::net::{TcpListener, TcpStream};
use std::collections::HashMap;

pub fn run(port: u16, verbose: bool) -> std::io::Result<()> {
    let listener = TcpListener::bind(&format!("127.0.0.1:{}", port))?;
    let addr = listener.local_addr()?;
    eprintln!("Started daemon on port {}", addr.port());

    let parser = metadata::daemon_parser();
    
    let font = Font::new();
    let mut cacher = BigCacher::init(true);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                process(stream, parser.clone(), &font, &mut cacher, verbose);
            }
            Err(e) => {
                eprintln!("Daemon error: {}", e);
            }
        }
    }
    Ok(())
}

fn abort(stream: &mut TcpStream, reason: String) {
    eprintln!("Command processing failed: {}", reason);
    let answer = format!("ERR\n{}", reason);
    let _ = stream.write(answer.as_bytes());
}

fn process<'a, 'b>(mut stream: TcpStream, parser: clap::App<'a, 'b>,
            font: &Font, cacher: &mut BigCacher, verbose: bool) {
    let reader = match stream.try_clone() {
        Ok(reader) => { reader }
        Err(e) => {
            eprintln!("Couldn't clone TcpStream: {}", e);
            return;
        }
    };
    let mut reader = std::io::BufReader::new(reader);
    let mut buf = String::new();
    match reader.read_line(&mut buf) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Couldn't read command: {}", e);
            return;
        }
    }
    let buf = buf.split('\n').next().unwrap_or("");
    let cmd = format!("censor {}", buf);
    let cmd_split = match escape_string::split(&cmd) {
        Some(x) => {
            x.into_iter().map(|y| y.into_owned()).collect::<Vec<String>>()
        }
        None => {
            return abort(&mut stream, "Error splitting the command".into());
        }
    };

    let matches = match parser.get_matches_from_safe(cmd_split) {
        Ok(x) => { x }
        Err(_) => {
            return abort(&mut stream, "Invalid command".into());
        }
    };

    if let Some(matches) = matches.subcommand_matches("analyse") {
        daemon_analyse(&mut stream, matches, font, cacher, verbose);
        return;
    }
    if let Some(matches) = matches.subcommand_matches("compute") {
        daemon_compute(&mut stream, matches);
        return;
    }

    return abort(&mut stream, "Invalid command".into());
}

fn palette_from_cmd<'a>(matches: &clap::ArgMatches<'a>, verbose: bool)
            -> Result<LoadedPalette, String> {
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
            return Err("Impossible happened! Blame the `clap` library. Report this error.".into());
        }
    }
    let palette = match result {
        Ok(x) => { x }
        Err(e) => {
            return Err(format!("Error while getting palette: {:?}", e));
        }
    };
    return Ok(palette);
}

fn daemon_analyse<'a>(stream: &mut TcpStream, matches: &clap::ArgMatches<'a>,
            font: &Font, cacher: &mut BigCacher, verbose: bool) {
    let grey_ui = matches.is_present("grey_ui");

    let mut outfile: String = matches.value_of("outfile").unwrap().into();
    if !outfile.ends_with(".png") {
        outfile = format!("{}.png", outfile);
    }

    let T: f32;
    if let Some(D) = matches.value_of("D") {
        match D {
            "50" => { T = 5000.00 }
            "55" => { T = 5500.00 }
            "65" => { T = 6503.51 }
            _ => {
                return abort(stream, format!("Invalid illuminant preset: D{}", D));
            }
        }
    } else {
        T = match str::parse(matches.value_of("T").unwrap_or("5500")) {
            Ok(x) => { x }
            Err(e) => {
                return abort(stream, format!("Error parsing temperature: {}", e));
            }
        };
    }

    let cache = cacher.at(T);

    let palette = match palette_from_cmd(matches, verbose) {
        Ok(x) => { x }
        Err(e) => { return abort(stream, e); }
    };

    match check_palette(&palette.colours) {
        Ok(_) => {}
        Err(e) => {
            return abort(stream, format!("Error while validating palette: {:?}", e));
        }
    }

    analyse(&palette, T, cache, &font, grey_ui, outfile, verbose);
    let _ = stream.write("OK\n".as_bytes());

    if let Err(e) = cacher.save() {
        if verbose {
            eprintln!("Error saving cache: {}", e);
        }
    }
}

fn daemon_compute<'a>(stream: &mut TcpStream, matches: &clap::ArgMatches<'a>) {
    let T: f32;
    if let Some(D) = matches.value_of("D") {
        match D {
            "50" => { T = 5000.00 }
            "55" => { T = 5500.00 }
            "65" => { T = 6503.51 }
            _ => {
                return abort(stream, format!("Invalid illuminant preset: D{}", D));
            }
        }
    } else {
        T = match str::parse(matches.value_of("T").unwrap_or("5500")) {
            Ok(x) => { x }
            Err(e) => {
                return abort(stream, format!("Error parsing temperature: {}", e));
            }
        };
    }
    let ill = CAT16Illuminant::new(CIExy::from_T(T));

    let palette = match palette_from_cmd(matches, false) {
        Ok(x) => { x }
        Err(e) => { return abort(stream, e); }
    };
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
            let _ = stream.write(format!("{},{}\n", metric, v).as_bytes());
        }
    }
}
