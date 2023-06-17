use std::ffi::OsString;

use clap::{error::Result, ArgAction, Parser, Subcommand, ValueEnum};

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Default, strum::Display)]
#[strum(ascii_case_insensitive, serialize_all = "lowercase")]
pub enum Format {
    #[default]
    List,
    Json,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Default, strum::Display)]
#[strum(ascii_case_insensitive, serialize_all = "lowercase")]
pub enum Color {
    #[default]
    Auto,
    Never,
    Always,
}

#[derive(Parser, Debug)]
#[clap(bin_name = "cargo")]
struct Cargo {
    #[command(subcommand)]
    command: CargoCommand,
}

#[derive(Subcommand, Debug)]
enum CargoCommand {
    Outdated(Options),
}

/// Options from CLI arguments
#[derive(Parser, Debug, PartialEq, Default)]
#[command(version)]
#[command(about = "Displays information about project dependency versions")]
pub struct Options {
    /// Output formatting
    #[arg(long, value_enum, ignore_case = true, default_value_t = Default::default())]
    pub format: Format,
    /// Output coloring
    #[arg(long, value_enum, ignore_case = true, default_value_t = Default::default())]
    pub color: Color,
    /// Space-separated list of features
    #[arg(long, use_value_delimiter = true)]
    pub features: Vec<String>,
    /// Dependencies to not print in the output (comma separated or one per
    /// '--ignore' argument)
    #[arg(short, long, value_name = "DEPENDENCIES", use_value_delimiter = true)]
    pub ignore: Vec<String>,
    /// Dependencies to exclude from building (comma separated or one per
    /// '--exclude' argument)
    #[arg(
        short = 'x',
        long,
        value_name = "DEPENDENCIES",
        use_value_delimiter = true
    )]
    pub exclude: Vec<String>,
    /// Path to the Cargo.toml file to use (Default to Cargo.toml in project
    /// root)
    #[arg(short, long, value_name = "PATH")]
    pub manifest_path: Option<String>,
    /// Suppresses warnings
    #[arg(short, long)]
    pub quiet: bool,
    /// Use verbose output
    #[arg(short, long, action = ArgAction::Count)]
    pub verbose: u8,
    /// The exit code to return on new versions found
    #[arg(long, value_name = "NUM", default_value_t = Default::default())]
    pub exit_code: i32,
    /// Packages to inspect for updates (comma separated or one per --packages'
    /// argument)
    #[arg(short, long, value_name = "PKGS", use_value_delimiter = true)]
    pub packages: Vec<String>,
    /// Package to treat as the root package
    #[arg(short, long)]
    pub root: Option<String>,
    /// How deep in the dependency chain to search (Defaults to all
    /// dependencies)
    #[arg(short, long, value_name = "NUM")]
    pub depth: Option<i32>,
    /// Only check root dependencies (Equivalent to --depth=1)
    #[arg(short = 'R', long)]
    pub root_deps_only: bool,
    /// Checks updates for all workspace members rather than only the root
    /// package
    #[arg(short, long)]
    pub workspace: bool,
    /// Ignores channels for latest updates
    #[arg(short, long)]
    pub aggressive: bool,
    /// Ignore relative dependencies external to workspace and check root
    /// dependencies only
    #[arg(short = 'e', long = "ignore-external-rel")]
    pub workspace_only: bool,
    /// Run without accessing the network (useful for testing w/ local
    /// registries)
    #[arg(short, long)]
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

pub fn parse() -> Options {
    match try_parse_from(std::env::args_os()) {
        Ok(opts) => opts,
        Err(clap_err) => clap_err.exit(),
    }
}

fn split_elem_by_ascii_whitespace(slice: &[String]) -> Vec<String> {
    slice
        .iter()
        .flat_map(|x| x.split_ascii_whitespace())
        .map(ToOwned::to_owned)
        .collect()
}

fn try_parse_from(args: impl IntoIterator<Item = impl Into<OsString> + Clone>) -> Result<Options> {
    let CargoCommand::Outdated(mut opts) = Cargo::try_parse_from(args)?.command;

    opts.exclude = split_elem_by_ascii_whitespace(&opts.exclude);
    opts.features = split_elem_by_ascii_whitespace(&opts.features);
    opts.ignore = split_elem_by_ascii_whitespace(&opts.ignore);
    opts.packages = split_elem_by_ascii_whitespace(&opts.packages);

    if opts.root_deps_only {
        opts.depth = Some(1);
    }

    if opts.workspace_only {
        opts.depth = Some(1);
        opts.root_deps_only = true;
    }

    Ok(opts)
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions::assert_eq;

    fn options(args: &[&str]) -> Options { options_fail(args).unwrap() }

    fn options_fail(args: &[&str]) -> clap::error::Result<Options> {
        let mut argv = vec!["cargo", "outdated"];
        argv.extend(args);
        try_parse_from(argv)
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
            res.as_ref().unwrap_err().kind(),
            clap::error::ErrorKind::UnknownArgument,
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
            res.as_ref().unwrap_err().kind(),
            clap::error::ErrorKind::UnknownArgument,
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
            res.as_ref().unwrap_err().kind(),
            clap::error::ErrorKind::UnknownArgument,
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
            res.as_ref().unwrap_err().kind(),
            clap::error::ErrorKind::UnknownArgument,
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
            res.as_ref().unwrap_err().kind(),
            clap::error::ErrorKind::InvalidValue,
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
            res.as_ref().unwrap_err().kind(),
            clap::error::ErrorKind::InvalidValue,
        );
    }
}
