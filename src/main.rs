//! cargo-outdated
//! A subcommand for cargo that checks if your dependencies are up-to-date

#![deny(bare_trait_objects, anonymous_parameters, elided_lifetimes_in_paths)]

#[macro_use]
mod macros;
mod cargo_ops;
mod error;

use crate::{
    cargo_ops::{ElaborateWorkspace, TempProject},
    error::OutdatedError,
};

use cargo::core::shell::Verbosity;
use cargo::core::Workspace;
use cargo::ops::needs_custom_http_transport;
use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::util::{CargoResult, CliError, Config};
use docopt::Docopt;

/// usage message for --help
pub const USAGE: &str = "
Displays information about project dependency versions

USAGE:
    cargo outdated [options]

Options:
    -a, --aggressive            Ignores channels for latest updates
    -h, --help                  Prints help information
        --format FORMAT         Output formatting [default: list]
                                [values: list, json]
    -i, --ignore DEPENDENCIES   Comma separated list of dependencies to not print in the output
    -x, --exclude DEPENDENCIES  Comma separated list of dependencies to exclude from building
    -q, --quiet                 Suppresses warnings
    -R, --root-deps-only        Only check root dependencies (Equivalent to --depth=1)
    -V, --version               Prints version information
    -v, --verbose ...           Use verbose output
    -w, --workspace             Checks updates for all workspace members rather than
                                only the root package
        --color COLOR           Coloring: auto, always, never [default: auto]
                                [values: auto, always, never]
    -d, --depth NUM             How deep in the dependency chain to search
                                (Defaults to all dependencies when omitted)
        --exit-code NUM         The exit code to return on new versions found [default: 0]
        --features FEATURES     Space-separated list of features
    -m, --manifest-path FILE    Path to the Cargo.toml file to use
                                (Defaults to Cargo.toml in project root)
    -p, --packages PKGS         Packages to inspect for updates
    -r, --root ROOT             Package to treat as the root package
    -o, --offline               Run without accessing the network (useful for testing w/ local registries)
";

/// Options from CLI arguments
#[derive(serde_derive::Deserialize, Debug, PartialEq, Default)]
pub struct Options {
    flag_format: Option<String>,
    flag_color: Option<String>,
    flag_features: Vec<String>,
    flag_ignore: Vec<String>,
    flag_exclude: Vec<String>,
    flag_manifest_path: Option<String>,
    flag_quiet: bool,
    flag_verbose: u32,
    flag_exit_code: i32,
    flag_packages: Vec<String>,
    flag_root: Option<String>,
    flag_depth: Option<i32>,
    flag_root_deps_only: bool,
    flag_workspace: bool,
    flag_aggressive: bool,
    flag_offline: bool,
}

impl Options {
    fn all_features(&self) -> bool { self.flag_features.is_empty() }

    fn no_default_features(&self) -> bool {
        !(self.flag_features.is_empty() || self.flag_features.contains(&"default".to_owned()))
    }

    fn locked(&self) -> bool { false }

    fn frozen(&self) -> bool { false }
}

fn main() {
    env_logger::init();
    let options = {
        let mut options: Options = Docopt::new(USAGE)
            .and_then(|d| {
                d.version(Some(
                    concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION")).to_owned(),
                ))
                .deserialize()
            })
            .unwrap_or_else(|e| e.exit());
        fn flat_split(arg: &[String]) -> Vec<String> {
            arg.iter()
                .flat_map(|s| s.split_whitespace())
                .flat_map(|s| s.split(','))
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect()
        }
        options.flag_features = flat_split(&options.flag_features);
        options.flag_ignore = flat_split(&options.flag_ignore);
        options.flag_exclude = flat_split(&options.flag_exclude);
        options.flag_packages = flat_split(&options.flag_packages);
        if options.flag_root_deps_only {
            options.flag_depth = Some(1);
        }
        options
    };

    let mut config = match Config::default() {
        Ok(cfg) => cfg,
        Err(e) => {
            let mut shell = cargo::core::Shell::new();
            cargo::exit_with_error(e.into(), &mut shell)
        }
    };

    // Only use a custom transport if any HTTP options are specified,
    // such as proxies or custom certificate authorities. The custom
    // transport, however, is not as well battle-tested.
    // See cargo-outdated issue #197 and
    // https://github.com/rust-lang/cargo/blob/master/src/bin/cargo/main.rs#L181
    // fn init_git_transports()
    if let Ok(true) = needs_custom_http_transport(&config) {
        if let Ok(handle) = cargo::ops::http_handle(&config) {
            unsafe {
                git2_curl::register(handle);
            }
        }
    }

    let exit_code = options.flag_exit_code;
    let result = execute(options, &mut config);
    match result {
        Err(e) => {
            config.shell().set_verbosity(Verbosity::Normal);
            let cli_error = CliError::new(e, 1);
            cargo::exit_with_error(cli_error, &mut *config.shell())
        }
        Ok(i) => {
            if i > 0 {
                std::process::exit(exit_code);
            } else {
                std::process::exit(0);
            }
        }
    }
}

/// executes the cargo-outdate command with the cargo configuration and options
pub fn execute(options: Options, config: &mut Config) -> CargoResult<i32> {
    // Check if $CARGO_HOME is set before capturing the config environment
    // if it is, set it in the configure options
    let cargo_home_path = std::env::var_os("CARGO_HOME").map(std::path::PathBuf::from);

    // enabling nightly features
    config.nightly_features_allowed = true;

    config.configure(
        options.flag_verbose,
        options.flag_quiet,
        options.flag_color.as_deref(),
        options.frozen(),
        options.locked(),
        options.flag_offline,
        &cargo_home_path,
        &[],
        &[],
    )?;
    debug!(config, format!("options: {:?}", options));

    verbose!(config, "Parsing...", "current workspace");
    // the Cargo.toml that we are actually working on
    let mut manifest_abspath: std::path::PathBuf;
    let curr_manifest = if let Some(ref manifest_path) = options.flag_manifest_path {
        manifest_abspath = manifest_path.into();
        if manifest_abspath.is_relative() {
            verbose!(config, "Resolving...", "absolute path of manifest");
            manifest_abspath = std::env::current_dir()?.join(manifest_path);
        }
        manifest_abspath
    } else {
        find_root_manifest_for_wd(config.cwd())?
    };
    let curr_workspace = Workspace::new(&curr_manifest, config)?;
    verbose!(config, "Resolving...", "current workspace");
    if options.flag_verbose == 0 {
        config.shell().set_verbosity(Verbosity::Quiet);
    }
    let ela_curr = ElaborateWorkspace::from_workspace(&curr_workspace, &options)?;
    if options.flag_verbose > 0 {
        config.shell().set_verbosity(Verbosity::Verbose);
    } else {
        config.shell().set_verbosity(Verbosity::Normal);
    }

    verbose!(config, "Parsing...", "compat workspace");
    let compat_proj =
        TempProject::from_workspace(&ela_curr, &curr_manifest.to_string_lossy(), &options)?;
    compat_proj.write_manifest_semver(
        curr_workspace.root(),
        compat_proj.temp_dir.path(),
        &ela_curr,
    )?;
    verbose!(config, "Updating...", "compat workspace");
    compat_proj.cargo_update()?;
    verbose!(config, "Resolving...", "compat workspace");
    let compat_workspace = compat_proj.workspace.borrow();
    let ela_compat = ElaborateWorkspace::from_workspace(
        compat_workspace
            .as_ref()
            .ok_or(OutdatedError::CannotElaborateWorkspace)?,
        &options,
    )?;

    verbose!(config, "Parsing...", "latest workspace");
    let latest_proj =
        TempProject::from_workspace(&ela_curr, &curr_manifest.to_string_lossy(), &options)?;
    latest_proj.write_manifest_latest(
        curr_workspace.root(),
        compat_proj.temp_dir.path(),
        &ela_curr,
    )?;
    verbose!(config, "Updating...", "latest workspace");
    latest_proj.cargo_update()?;
    verbose!(config, "Resolving...", "latest workspace");
    let latest_workspace = latest_proj.workspace.borrow();
    let ela_latest = ElaborateWorkspace::from_workspace(
        latest_workspace
            .as_ref()
            .ok_or(OutdatedError::CannotElaborateWorkspace)?,
        &options,
    )?;

    if ela_curr.workspace_mode {
        let mut sum = 0;
        if options.flag_format == Some("list".to_string()) {
            verbose!(config, "Printing...", "Package status in list format");
        } else if options.flag_format == Some("json".to_string()) {
            verbose!(config, "Printing...", "Package status in json format");
        }

        for member in ela_curr.workspace.members() {
            ela_curr.resolve_status(
                &ela_compat,
                &ela_latest,
                &options,
                config,
                member.package_id(),
            )?;
            if options.flag_format == Some("list".to_string()) {
                sum += ela_curr.print_list(&options, member.package_id(), sum > 0)?;
            } else if options.flag_format == Some("json".to_string()) {
                sum += ela_curr.print_json(&options, member.package_id())?;
            }
        }
        if sum == 0 {
            println!("All dependencies are up to date, yay!");
        }
        Ok(sum)
    } else {
        verbose!(config, "Resolving...", "package status");
        let root = ela_curr.determine_root(&options)?;
        ela_curr.resolve_status(&ela_compat, &ela_latest, &options, config, root)?;
        verbose!(config, "Printing...", "list format");
        let mut count = 0;

        if options.flag_format == Some("list".to_string()) {
            count = ela_curr.print_list(&options, root, false)?;
        } else if options.flag_format == Some("json".to_string()) {
            ela_curr.print_json(&options, root)?;
        } else {
            println!("Error, did not specify list or json output formatting");
            std::process::exit(2);
        }

        Ok(count)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn options(args: &[&str]) -> Options {
        let mut argv = vec!["cargo", "outdated"];
        if !args.is_empty() {
            argv.extend(args);
        }
        let mut options: Options = Docopt::new(USAGE)
            .and_then(|d| {
                d.version(Some(
                    concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION")).to_owned(),
                ))
                .argv(argv)
                .deserialize()
            })
            .unwrap_or_else(|e| e.exit());
        fn flat_split(arg: &[String]) -> Vec<String> {
            arg.iter()
                .flat_map(|s| s.split_whitespace())
                .flat_map(|s| s.split(','))
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect()
        }
        options.flag_features = flat_split(&options.flag_features);
        options.flag_ignore = flat_split(&options.flag_ignore);
        options.flag_exclude = flat_split(&options.flag_exclude);
        options.flag_packages = flat_split(&options.flag_packages);
        if options.flag_root_deps_only {
            options.flag_depth = Some(1);
        }
        options
    }

    #[test]
    fn default() {
        let opts = options(&[]);
        assert_eq!(
            Options {
                flag_format: Some("list".into()),
                flag_color: Some("auto".into()),
                ..Options::default()
            },
            opts
        )
    }

    #[test]
    fn root_only() {
        let opts = options(&["--root-deps-only"]);
        assert_eq!(
            Options {
                flag_format: Some("list".into()),
                flag_color: Some("auto".into()),
                flag_depth: Some(1),
                flag_root_deps_only: true,
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
        // Not supported
        //let opts4 = options("--features one --features two --features three");
        //let opts5 = options("--features one --features two,three");
        let correct = Options {
            flag_format: Some("list".into()),
            flag_color: Some("auto".into()),
            flag_features: vec!["one".into(), "two".into(), "three".into()],
            ..Options::default()
        };

        assert_eq!(correct, opts1);
        assert_eq!(correct, opts2);
        assert_eq!(correct, opts3);
    }

    #[test]
    fn exclude() {
        let opts1 = options(&["--exclude=one,two,three"]);
        let opts2 = options(&["--exclude", "one,two,three"]);
        let opts3 = options(&["--exclude", "one two three"]);
        // Not supported
        //let opts4 = options("--exclude one two three");
        //let opts5 = options("--exclude one --exclude two --exclude three");
        //let opts6 = options("--exclude one --exclude two,three");
        let correct = Options {
            flag_format: Some("list".into()),
            flag_color: Some("auto".into()),
            flag_exclude: vec!["one".into(), "two".into(), "three".into()],
            ..Options::default()
        };

        assert_eq!(correct, opts1);
        assert_eq!(correct, opts2);
        assert_eq!(correct, opts3);
    }

    #[test]
    fn ignore() {
        let opts1 = options(&["--ignore=one,two,three"]);
        let opts2 = options(&["--ignore", "one,two,three"]);
        let opts3 = options(&["--ignore", "one two three"]);
        // Not supported
        //let opts4 = options("--ignore one two three");
        //let opts5 = options("--ignore one --ignore two --ignore three");
        //let opts6 = options("--ignore one --ignore two,three");
        let correct = Options {
            flag_format: Some("list".into()),
            flag_color: Some("auto".into()),
            flag_ignore: vec!["one".into(), "two".into(), "three".into()],
            ..Options::default()
        };

        assert_eq!(correct, opts1);
        assert_eq!(correct, opts2);
        assert_eq!(correct, opts3);
    }

    #[test]
    fn verbose() {
        let opts1 = options(&["--verbose", "--verbose", "--verbose"]);
        let correct = Options {
            flag_format: Some("list".into()),
            flag_color: Some("auto".into()),
            flag_verbose: 3,
            ..Options::default()
        };

        assert_eq!(correct, opts1);
    }

    #[test]
    fn packages() {
        let opts1 = options(&["--packages", "one,two"]);
        let opts2 = options(&["--packages", "one two"]);
        // Not Supported
        //let opts3 = options(&["--packages","one","--packages","two"]);
        //let opts4 = options(&["--packages", "one", "two"]);
        let correct = Options {
            flag_format: Some("list".into()),
            flag_color: Some("auto".into()),
            flag_packages: vec!["one".into(), "two".into()],
            ..Options::default()
        };

        assert_eq!(correct, opts1);
        assert_eq!(correct, opts2);
    }
}
