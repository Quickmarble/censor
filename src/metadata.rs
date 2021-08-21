use const_format::formatcp;

use clap::{Arg, App, SubCommand, ArgGroup};

pub const VERSION_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
pub const VERSION_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
pub const VERSION_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");
pub const VERSION: &str = formatcp!("{}.{}.{}", VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH);

pub fn cmd_parser<'a, 'b>() -> App<'a, 'b> {
    let (input_group, input_args) = palette_input_args();
    let (interp_groups, interp_args) = interpretation_args();
    let (metrics_group, metrics_args) = metrics_args();
    let (repr_groups, repr_args) = representation_args();
    let verbose = verbose_arg();

    let daemon = SubCommand::with_name("daemon")
        .about("Starts in daemon mode.")
        .arg(verbose.clone())
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .help("The port exposed by the daemon")
                .takes_value(true)
                .required(true)
        );
    let analyse = SubCommand::with_name("analyse")
        .about("Produces a plot with palette analysis.")
        .arg(verbose.clone())
        .group(input_group.clone())
        .args(input_args.as_slice())
        .groups(interp_groups.as_slice())
        .args(interp_args.as_slice())
        .groups(repr_groups.as_slice())
        .args(repr_args.as_slice())
        .arg(
            Arg::with_name("outfile")
                .short("o")
                .long("out")
                .value_name("FILE")
                .help("Sets output image file; default: plot.png")
                .takes_value(true)
        );
    let compute = SubCommand::with_name("compute")
        .about("Computes palette metrics.")
        .group(input_group.clone())
        .args(input_args.as_slice())
        .groups(interp_groups.as_slice())
        .args(interp_args.as_slice())
        .group(metrics_group.clone())
        .args(metrics_args.as_slice());

    let app = App::new("censor")
        .version(VERSION)
        .about("Palette analysis tool.")
        .subcommand(daemon)
        .subcommand(analyse)
        .subcommand(compute);

    return app;
}

pub fn daemon_parser<'a, 'b>() -> App<'a, 'b> {
    let (input_group, input_args) = palette_input_args();
    let (interp_groups, interp_args) = interpretation_args();
    let (metrics_group, metrics_args) = metrics_args();
    let (repr_groups, repr_args) = representation_args();

    let analyse = SubCommand::with_name("analyse")
        .about("Produces a plot with palette analysis.")
        .group(input_group.clone())
        .args(input_args.as_slice())
        .groups(interp_groups.as_slice())
        .args(interp_args.as_slice())
        .groups(repr_groups.as_slice())
        .args(repr_args.as_slice())
        .arg(
            Arg::with_name("outfile")
                .short("o")
                .long("out")
                .value_name("FILE")
                .help("Sets output image file")
                .takes_value(true)
                .required(true)
        );
    let compute = SubCommand::with_name("compute")
        .about("Computes palette metrics.")
        .group(input_group.clone())
        .args(input_args.as_slice())
        .groups(interp_groups.as_slice())
        .args(interp_args.as_slice())
        .group(metrics_group.clone())
        .args(metrics_args.as_slice());

    let app = App::new("censor")
        .version(VERSION)
        .about("Palette analysis daemon.")
        .subcommand(analyse)
        .subcommand(compute);

    return app;
}

fn palette_input_args<'a, 'b>() -> (ArgGroup<'a>, Vec<Arg<'a, 'b>>) {
    let group = ArgGroup::with_name("input")
        .multiple(false)
        .required(true)
        .args(&["colours", "hexfile", "imagefile", "lospec"]);
    let args = vec![
        Arg::with_name("colours")
            .short("c")
            .long("colours")
            .value_name("LIST")
            .help("Sets input colours to the specified list of comma-separated hex values")
            .takes_value(true),
        Arg::with_name("hexfile")
            .short("f")
            .long("hexfile")
            .value_name("FILE")
            .help("Reads input colours from the specified file with newline-separated hex values")
            .takes_value(true),
        Arg::with_name("imagefile")
            .short("i")
            .long("image")
            .value_name("FILE")
            .help("Reads input colours from the specified image")
            .takes_value(true),
        Arg::with_name("lospec")
            .short("l")
            .long("lospec")
            .value_name("SLUG")
            .help("Loads input colours from https://lospec.com/palette-list/SLUG")
            .takes_value(true)
    ];
    return (group, args);
}

fn interpretation_args<'a, 'b>() -> (Vec<ArgGroup<'a>>, Vec<Arg<'a, 'b>>) {
    let groups = vec![
        ArgGroup::with_name("illuminant")
            .multiple(false)
            .required(false)
            .args(&["T", "D"])
    ];
    let args = vec![
        Arg::with_name("T")
            .short("T")
            .value_name("TEMP")
            .help("Use TEMP Kelvins to define the white point for the daylight illuminant. Default: 5500")
            .takes_value(true),
        Arg::with_name("D")
            .short("D")
            .value_name("NUM")
            .help("Use a predefined white point for the daylight illuminant. Supported values: 50, 55, 65")
            .takes_value(true)
    ];
    return (groups, args);
}

fn metrics_args<'a, 'b>() -> (ArgGroup<'a>, Vec<Arg<'a, 'b>>) {
    let group = ArgGroup::with_name("metrics")
        .multiple(true)
        .required(true)
        .args(&["all", "iss", "acyclic"]);
    let args = vec![
        Arg::with_name("all")
            .short("a")
            .long("all")
            .help("Computes all the metrics"),
        Arg::with_name("iss")
            .long("iss")
            .help("Computes internal similarity score"),
        Arg::with_name("acyclic")
            .long("acyclic")
            .help("Checks is a palette is acyclic")
    ];
    return (group, args);
}

fn representation_args<'a, 'b>() -> (Vec<ArgGroup<'a>>, Vec<Arg<'a, 'b>>) {
    let groups = vec![];
    let args = vec![
        Arg::with_name("grey_ui")
            .short("g")
            .long("grey")
            .help("Uses black, grey and white for UI instead of choosing palette colours")
    ];
    return (groups, args);
}

fn verbose_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("verbose")
        .short("v")
        .long("verbose")
        .help("Prints debugging output")
}
