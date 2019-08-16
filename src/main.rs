#![deny(bare_trait_objects, anonymous_parameters, elided_lifetimes_in_paths)]

use cargo;
use env_logger;

#[macro_use]
mod macros;
mod cargo_ops;
use crate::cargo_ops::{ElaborateWorkspace, TempProject};

use cargo::core::maybe_allow_nightly_features;
use cargo::core::shell::Verbosity;
use cargo::core::Workspace;
use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::util::{CargoResult, CliError, Config};
use docopt::Docopt;

pub const USAGE: &str = "
Displays information about project dependency versions

USAGE:
    cargo outdated [options]

Options:
    -a, --aggressive            Ignores channels for latest updates
    -h, --help                  Prints help information
        --format FORMAT         Output formatting [default: list]
                                [values: list, json]
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
    -m, --manifest-path FILE    An absolute path to the Cargo.toml file to use
                                (Defaults to Cargo.toml in project root)
    -p, --packages PKGS         Packages to inspect for updates
    -r, --root ROOT             Package to treat as the root package
";

/// Options from CLI arguments
#[derive(serde_derive::Deserialize, Debug)]
pub struct Options {
    flag_format: Option<String>,
    flag_color: Option<String>,
    flag_features: Vec<String>,
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
}

impl Options {
    fn all_features(&self) -> bool {
        self.flag_features.is_empty()
    }

    fn no_default_features(&self) -> bool {
        !(self.flag_features.is_empty() || self.flag_features.contains(&"default".to_owned()))
    }

    fn locked(&self) -> bool {
        false
    }

    fn frozen(&self) -> bool {
        false
    }
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

pub fn execute(options: Options, config: &mut Config) -> CargoResult<i32> {
    config.configure(
        options.flag_verbose,
        None,
        &options.flag_color,
        options.frozen(),
        options.locked(),
        false,
        &None,
        &[],
    )?;
    debug!(config, format!("options: {:?}", options));

    // Needed to allow nightly features
    maybe_allow_nightly_features();

    verbose!(config, "Parsing...", "current workspace");
    // the Cargo.toml that we are actually working on
    let curr_manifest = if let Some(ref manifest_path) = options.flag_manifest_path {
        manifest_path.into()
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
    let ela_compat =
        ElaborateWorkspace::from_workspace(compat_workspace.as_ref().unwrap(), &options)?;

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
    let ela_latest =
        ElaborateWorkspace::from_workspace(latest_workspace.as_ref().unwrap(), &options)?;

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
                sum += ela_curr.print_json(&options, member.package_id(), sum > 0)?;
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
            ela_curr.print_json(&options, root, false)?;
        } else {
            println!("Error, did not specify list or json output formatting");
            std::process::exit(2);
        }

        Ok(count)
    }
}
