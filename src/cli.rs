use clap::{
    arg_enum, crate_version, value_t, value_t_or_exit, App, AppSettings, Arg, ArgMatches,
    SubCommand,
};

arg_enum! {
    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum Format {
        List,
        Json,
    }
}

arg_enum! {
    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum Color {
        Auto,
        Never,
        Always
    }
}

/// Options from CLI arguments
#[derive(Debug)]
pub struct Options {
    pub format: Format,
    pub color: Color,
    pub features: Vec<String>,
    pub ignore: Vec<String>,
    pub exclude: Vec<String>,
    pub manifest_path: Option<String>,
    pub quiet: bool,
    pub verbose: u64,
    pub exit_code: i32,
    pub packages: Vec<String>,
    pub root: Option<String>,
    pub depth: Option<i32>,
    pub root_deps_only: bool,
    pub workspace: bool,
    pub aggressive: bool,
    pub offline: bool,
}

impl Options {
    pub fn all_features(&self) -> bool { self.features.is_empty() }

    pub fn no_default_features(&self) -> bool {
        !(self.features.is_empty() || self.features.contains(&"default".to_owned()))
    }

    pub fn locked(&self) -> bool { false }

    pub fn frozen(&self) -> bool { false }
}

impl<'a> From<&ArgMatches<'a>> for Options {
    fn from(m: &ArgMatches<'a>) -> Self {
        let mut opts = Options {
            format: value_t_or_exit!(m.value_of("format"), Format),
            color: value_t_or_exit!(m.value_of("color"), Color),
            features: m
                .values_of("features")
                .map(|vals| {
                    vals.flat_map(|x| x.split_ascii_whitespace().collect::<Vec<_>>())
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_else(Vec::new),
            ignore: m
                .values_of("ignore")
                .map(|vals| {
                    vals.flat_map(|x| x.split_ascii_whitespace().collect::<Vec<_>>())
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_else(Vec::new),
            exclude: m
                .values_of("exclude")
                .map(|vals| {
                    vals.flat_map(|x| x.split_ascii_whitespace().collect::<Vec<_>>())
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_else(Vec::new),
            manifest_path: m.value_of("manifest-path").map(ToOwned::to_owned),
            quiet: m.is_present("quiet"),
            verbose: m.occurrences_of("verbose"),
            exit_code: value_t!(m, "exit-code", i32).ok().unwrap_or(0),
            packages: m
                .values_of("packages")
                .map(|vals| {
                    vals.flat_map(|x| x.split_ascii_whitespace().collect::<Vec<_>>())
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(Vec::new),
            root: m.value_of("root").map(ToOwned::to_owned),
            depth: value_t!(m, "depth", i32).ok(),
            root_deps_only: m.is_present("root-deps-only"),
            workspace: m.is_present("workspace"),
            aggressive: m.is_present("aggressive"),
            offline: m.is_present("offline"),
        };

        if m.is_present("root-deps-only") {
            opts.depth = Some(1);
        }

        opts
    }
}

pub fn parse() -> Options {
    let matches = App::new("cargo-outdated")
        .bin_name("cargo")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(
            SubCommand::with_name("outdated")
                .setting(AppSettings::UnifiedHelpMessage)
                .about("Displays information about project dependency versions")
                .version(crate_version!())
                .arg(
                    Arg::with_name("aggressive")
                        .short("a")
                        .long("aggresssive")
                        .help("Ignores channels for latest updates"),
                )
                .arg(
                    Arg::with_name("quiet")
                        .short("q")
                        .long("quiet")
                        .help("Suppresses warnings"),
                )
                .arg(
                    Arg::with_name("root-deps-only")
                        .short("R")
                        .long("root-deps-only")
                        .help("Only check root dependencies (Equivalent to --depth=1)"),
                )
                .arg(
                    Arg::with_name("workspace")
                        .short("w")
                        .long("workspace")
                        .help("Checks updates for all workspace members rather than only the root package"),
                )
                .arg(
                    Arg::with_name("offline")
                        .short("o")
                        .long("offline")
                        .help("Run without accessing the network (useful for testing w/ local registries)"),
                )
                .arg(
                    Arg::with_name("format")
                        .long("format")
                        .default_value("list")
                        .case_insensitive(true)
                        .possible_values(&Format::variants())
                        .value_name("FORMAT")
                        .help("Output formatting"),
                )
                .arg(
                    Arg::with_name("ignore")
                        .short("i")
                        .long("ignore")
                        .help("Dependencies to not print in the output (comma separated or one per '--ignore' argument)")
                        .value_delimiter(",")
                        .number_of_values(1)
                        .multiple(true)
                        .value_name("DEPENDENCIES"),
                )
                .arg(
                    Arg::with_name("exclude")
                        .short("x")
                        .long("exclude")
                        .help("Dependencies to exclude from building (comma separated or one per '--exclude' argument)")
                        .value_delimiter(",")
                        .multiple(true)
                        .number_of_values(1)
                        .value_name("DEPENDENCIES"),
                )
                .arg(
                    Arg::with_name("verbose")
                        .short("v")
                        .long("verbose")
                        .multiple(true)
                        .help("Use verbose output")
                )
                .arg(
                    Arg::with_name("color")
                        .long("color")
                        .possible_values(&Color::variants())
                        .default_value("auto")
                        .value_name("COLOR")
                        .case_insensitive(true)
                        .help("Output coloring")
                )
                .arg(
                    Arg::with_name("depth")
                        .short("d")
                        .long("depth")
                        .value_name("NUM")
                        .help("How deep in the dependency chain to search (Defaults to all dependencies when omitted)")
                )
                .arg(
                    Arg::with_name("exit-code")
                        .long("exit-code")
                        .help("The exit code to return on new versions found")
                        .default_value("0")
                        .value_name("NUM"))
                .arg(
                    Arg::with_name("manifest-path")
                        .short("m")
                        .long("manifest-path")
                        .help("Path to the Cargo.toml file to use (Defaults to Cargo.toml in project root)")
                        .value_name("PATH"))
                .arg(
                    Arg::with_name("root")
                        .short("r")
                        .long("root")
                        .help("Package to treat as the root package")
                        .value_name("ROOT"))
                .arg(
                    Arg::with_name("packages")
                        .short("p")
                        .long("packages")
                        .help("Packages to inspect for updates (comma separated or one per '--packages' argument)")
                        .value_delimiter(",")
                        .number_of_values(1)
                        .multiple(true)
                        .value_name("PKGS"))
                .arg(
                    Arg::with_name("features")
                        .long("features")
                        .value_delimiter(",")
                        .help("Space-separated list of features")
                        .multiple(true)
                        .number_of_values(1)
                        .value_name("FEATURES"))
            )
        .get_matches();

    Options::from(matches.subcommand_matches("outdated").unwrap())
}
