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

impl Default for Format {
    fn default() -> Self {
        Format::List
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

impl Default for Color {
    fn default() -> Self {
        Color::Auto
    }
}

/// Options from CLI arguments
#[derive(Debug, PartialEq, Default)]
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
    pub workspace_only: bool,
    pub offline: bool,
}

impl Options {
    pub fn all_features(&self) -> bool {
        self.features.is_empty()
    }

    pub fn no_default_features(&self) -> bool {
        !(self.features.is_empty() || self.features.contains(&"default".to_owned()))
    }

    pub fn locked(&self) -> bool {
        false
    }

    pub fn frozen(&self) -> bool {
        false
    }
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
            workspace_only: m.is_present("ignore-external-rel"),
            workspace: m.is_present("workspace"),
            aggressive: m.is_present("aggressive"),
            offline: m.is_present("offline"),
        };

        if m.is_present("root-deps-only") {
            opts.depth = Some(1);
        }

        if m.is_present("ignore-external-rel") {
            opts.depth = Some(1);
            opts.root_deps_only = true;
        }

        opts
    }
}

fn build() -> App<'static, 'static> {
    App::new("cargo-outdated")
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
                        .long("aggressive")
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
                    Arg::with_name("ignore-external-rel")
                        .short("e")
                        .long("ignore-external-rel")
                        .help("Ignore relative dependencies external to workspace and check root dependencies only."),
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
}

pub fn parse() -> Options {
    let matches = build().get_matches();

    Options::from(matches.subcommand_matches("outdated").unwrap())
}

#[cfg(test)]
mod test {
    use super::*;

    fn options(args: &[&str]) -> Options {
        let mut argv = vec!["cargo", "outdated"];
        argv.extend(args);
        let m = build().get_matches_from(argv);
        Options::from(m.subcommand_matches("outdated").unwrap())
    }

    fn options_fail(args: &[&str]) -> clap::Result<ArgMatches<'static>> {
        let mut argv = vec!["cargo", "outdated"];
        argv.extend(args);
        build().get_matches_from_safe(argv)
    }

    #[test]
    fn default() {
        let opts = options(&[]);
        assert_eq!(Options::default(), opts)
    }

    #[test]
    fn root_only() {
        let opts = options(&["--root-deps-only"]);
        assert_eq!(
            Options {
                depth: Some(1),
                root_deps_only: true,
                ..Options::default()
            },
            opts
        )
    }

    #[test]
    fn workspace_only() {
        let opts = options(&["--ignore-external-rel"]);
        assert_eq!(
            Options {
                workspace_only: true,
                depth: Some(1),
                root_deps_only: true,
                ..Options::default()
            },
            opts
        )
    }

    #[test]
    fn features() {
        let opts1 = options(&["--features=one,two,three"]);
        let opts2 = options(&["--features", "one,two,three"]);
        let opts3 = options(&["--features", "one two three"]);
        let opts4 = options(&[
            "--features",
            "one",
            "--features",
            "two",
            "--features",
            "three",
        ]);
        let opts5 = options(&["--features", "one", "--features", "two,three"]);

        let correct = Options {
            features: vec!["one".into(), "two".into(), "three".into()],
            ..Options::default()
        };

        assert_eq!(correct, opts1);
        assert_eq!(correct, opts2);
        assert_eq!(correct, opts3);
        assert_eq!(correct, opts4);
        assert_eq!(correct, opts5);
    }

    #[test]
    fn features_fail() {
        let res = options_fail(&["--features", "one", "two"]);
        assert!(res.is_err());
        assert_eq!(
            res.as_ref().unwrap_err().kind,
            clap::ErrorKind::UnknownArgument,
            "{:?}",
            res.as_ref().unwrap_err().kind
        );
    }

    #[test]
    fn exclude() {
        let opts1 = options(&["--exclude=one,two,three"]);
        let opts2 = options(&["--exclude", "one,two,three"]);
        let opts3 = options(&["--exclude", "one two three"]);
        let opts4 = options(&["--exclude", "one", "--exclude", "two", "--exclude", "three"]);
        let opts5 = options(&["--exclude", "one", "--exclude", "two,three"]);
        let correct = Options {
            exclude: vec!["one".into(), "two".into(), "three".into()],
            ..Options::default()
        };

        assert_eq!(correct, opts1);
        assert_eq!(correct, opts2);
        assert_eq!(correct, opts3);
        assert_eq!(correct, opts4);
        assert_eq!(correct, opts5);
    }

    #[test]
    fn exclude_fail() {
        let res = options_fail(&["--exclude", "one", "two"]);
        assert!(res.is_err());
        assert_eq!(
            res.as_ref().unwrap_err().kind,
            clap::ErrorKind::UnknownArgument,
            "{:?}",
            res.as_ref().unwrap_err().kind
        );
    }

    #[test]
    fn ignore() {
        let opts1 = options(&["--ignore=one,two,three"]);
        let opts2 = options(&["--ignore", "one,two,three"]);
        let opts3 = options(&["--ignore", "one two three"]);
        let opts4 = options(&["--ignore", "one", "--ignore", "two", "--ignore", "three"]);
        let opts5 = options(&["--ignore", "one", "--ignore", "two,three"]);
        let correct = Options {
            ignore: vec!["one".into(), "two".into(), "three".into()],
            ..Options::default()
        };

        assert_eq!(correct, opts1);
        assert_eq!(correct, opts2);
        assert_eq!(correct, opts3);
        assert_eq!(correct, opts4);
        assert_eq!(correct, opts5);
    }

    #[test]
    fn ignore_fail() {
        let res = options_fail(&["--ignore", "one", "two"]);
        assert!(res.is_err());
        assert_eq!(
            res.as_ref().unwrap_err().kind,
            clap::ErrorKind::UnknownArgument,
            "{:?}",
            res.as_ref().unwrap_err().kind
        );
    }

    #[test]
    fn verbose() {
        let opts1 = options(&["--verbose", "--verbose", "--verbose"]);
        let correct = Options {
            verbose: 3,
            ..Options::default()
        };

        assert_eq!(correct, opts1);
    }

    #[test]
    fn packages() {
        let opts1 = options(&["--packages", "one,two"]);
        let opts2 = options(&["--packages", "one two"]);
        let opts3 = options(&["--packages", "one", "--packages", "two"]);
        let correct = Options {
            packages: vec!["one".into(), "two".into()],
            ..Options::default()
        };

        assert_eq!(correct, opts1);
        assert_eq!(correct, opts2);
        assert_eq!(correct, opts3);
    }

    #[test]
    fn packages_fail() {
        let res = options_fail(&["--packages", "one", "two"]);
        assert!(res.is_err());
        assert_eq!(
            res.as_ref().unwrap_err().kind,
            clap::ErrorKind::UnknownArgument,
            "{:?}",
            res.as_ref().unwrap_err().kind
        );
    }

    #[test]
    fn format_case() {
        let opts1 = options(&["--format", "JsOn"]);
        let correct = Options {
            format: Format::Json,
            ..Options::default()
        };

        assert_eq!(correct, opts1);
    }

    #[test]
    fn format_unknown() {
        let res = options_fail(&["--format", "foobar"]);
        assert!(res.is_err());
        assert_eq!(
            res.as_ref().unwrap_err().kind,
            clap::ErrorKind::InvalidValue,
            "{:?}",
            res.as_ref().unwrap_err().kind
        );
    }

    #[test]
    fn color_case() {
        let opts1 = options(&["--color", "NeVeR"]);
        let correct = Options {
            color: Color::Never,
            ..Options::default()
        };

        assert_eq!(correct, opts1);
    }

    #[test]
    fn color_unknown() {
        let res = options_fail(&["--color", "foobar"]);
        assert!(res.is_err());
        assert_eq!(
            res.as_ref().unwrap_err().kind,
            clap::ErrorKind::InvalidValue,
            "{:?}",
            res.as_ref().unwrap_err().kind
        );
    }
}
