use escape_string;

use crate::text::Font;
use crate::cache::PlotCacher;
use crate::analyse::*;
use crate::loader::*;

use std::io::{BufRead, Write};
use std::net::{TcpListener, TcpStream};

pub fn run(port: u16, verbose: bool) -> std::io::Result<()> {
    eprintln!("Starting daemon on port {}", port);
    let listener = TcpListener::bind(&format!("127.0.0.1:{}", port))?;
    
    let font = Font::new();
    let mut cacher = PlotCacher::new();
    let T = 5500.;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                process(stream, T, &font, &mut cacher, verbose);
            }
            Err(e) => {
                eprintln!("Daemon error: {:?}", e);
            }
        }
    }
    Ok(())
}

fn process(mut stream: TcpStream, T: f32, font: &Font, cacher: &mut PlotCacher, verbose: bool) {
    let reader = match stream.try_clone() {
        Ok(reader) => { reader }
        Err(e) => {
            eprintln!("Couldn't clone TcpStream: {:?}", e);
            return;
        }
    };
    let mut reader = std::io::BufReader::new(reader);
    let mut buf = String::new();
    match reader.read_line(&mut buf) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Couldn't read command: {:?}", e);
            return;
        }
    }
    let buf = buf.split('\n').next().unwrap_or("");
    let split = escape_string::split(buf);
    if let None = split {
        eprintln!("Errors in input.");
        let _ = stream.write("ERR\n".as_bytes());
        return;
    }
    let cmd: Vec<String> = split.unwrap().into_iter()
        .map(|x| x.into_owned())
        .collect();
    if cmd.len() == 0 {
        eprintln!("Empty input.");
        let _ = stream.write("ERR\n".as_bytes());
        return;
    }
    match cmd[0].as_str() {
        "analyse" => {
            if cmd.len() != 3 {
                eprintln!("Invalid number of arguments: {}.", buf);
                let _ = stream.write("ERR\n".as_bytes());
                return;
            }
            let outfile: String = cmd[2].clone();
            let src: Vec<&str> = cmd[1].split("://").collect();
            if src.len() != 2 {
                eprintln!("Invalid source: {}", cmd[1]);
                let _ = stream.write("ERR\n".as_bytes());
                return;
            }
            let src_type = src[0];
            let src_data = src[1];
            let result = match src_type {
                "hex" => {
                    let hex_list = src_data.split(',')
                        .map(|s| String::from(s))
                        .collect::<Vec<_>>();
                    load_from_hex(&hex_list)
                }
                "img" => {
                    load_from_image(src_data.into())
                }
                "file" => {
                    load_from_file(src_data.into())
                }
                _ => {
                    eprintln!("Unknown source type: {}", src_type);
                    let _ = stream.write("ERR\n".as_bytes());
                    return;
                }
            };
            let colours = match result {
                Ok(x) => { x }
                Err(e) => {
                    eprintln!("Error while getting palette: {:?}", e);
                    let _ = stream.write("ERR\n".as_bytes());
                    return;
                }
            };
            match check_palette(&colours) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error while validating palette: {:?}", e);
                    let _ = stream.write("ERR\n".as_bytes());
                    return;
                }
            }
            analyse(&colours, T, cacher, font, outfile, verbose);
            let _ = stream.write("OK\n".as_bytes());
            return;
        }
        _ => {
            eprintln!("Unknown command: {}", cmd[0]);
            let _ = stream.write("ERR\n".as_bytes());
            return;
        }
    }
}
