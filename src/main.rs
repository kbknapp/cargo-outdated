//! cargo-outdated
//! A subcommand for cargo that checks if your dependencies are up-to-date

#![deny(bare_trait_objects, anonymous_parameters, elided_lifetimes_in_paths)]

#[macro_use]
mod macros;
mod cargo_ops;
mod cli;
mod error;

use std::collections::HashSet;

use cargo::{
    core::{shell::Verbosity, Workspace},
    util::{
        important_paths::find_root_manifest_for_wd,
        network::http::{http_handle, needs_custom_http_transport},
        CargoResult, CliError, GlobalContext,
    },
};

use crate::{
    cargo_ops::{ElaborateWorkspace, TempProject},
    cli::{Format, Options},
    error::OutdatedError,
};

fn main() {
    env_logger::init();
    let options = cli::parse();

    let mut gctx = match GlobalContext::default() {
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
    if let Ok(true) = needs_custom_http_transport(&gctx) {
        if let Ok(handle) = http_handle(&gctx) {
            unsafe {
                git2_curl::register(handle);
            }
        }
    }

    let exit_code = options.exit_code;
    let result = execute(options, &mut gctx);
    match result {
        Err(e) => {
            gctx.shell().set_verbosity(Verbosity::Normal);
            let cli_error = CliError::new(e, 1);
            cargo::exit_with_error(cli_error, &mut gctx.shell())
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
pub fn execute(options: Options, gctx: &mut GlobalContext) -> CargoResult<i32> {
    // Check if $CARGO_HOME is set before capturing the config environment
    // if it is, set it in the configure options
    let cargo_home_path = std::env::var_os("CARGO_HOME").map(std::path::PathBuf::from);

    // enabling nightly features
    gctx.nightly_features_allowed = true;

    gctx.configure(
        options.verbose.into(),
        options.quiet,
        Some(&options.color.to_string().to_ascii_lowercase()),
        options.frozen(),
        options.locked(),
        options.offline,
        &cargo_home_path,
        &[],
        &[],
    )?;
    debug!(gctx, format!("options: {options:?}"));

    verbose!(gctx, "Parsing...", "current workspace");
    // the Cargo.toml that we are actually working on
    let mut manifest_abspath: std::path::PathBuf;
    let curr_manifest = if let Some(ref manifest_path) = options.manifest_path {
        manifest_abspath = manifest_path.into();
        if manifest_abspath.is_relative() {
            verbose!(gctx, "Resolving...", "absolute path of manifest");
            manifest_abspath = std::env::current_dir()?.join(manifest_path);
        }
        manifest_abspath
    } else {
        find_root_manifest_for_wd(gctx.cwd())?
    };
    let curr_workspace = Workspace::new(&curr_manifest, gctx)?;
    verbose!(gctx, "Resolving...", "current workspace");
    if options.verbose == 0 {
        gctx.shell().set_verbosity(Verbosity::Quiet);
    }
    let ela_curr = ElaborateWorkspace::from_workspace(&curr_workspace, &options)?;
    if options.verbose > 0 {
        gctx.shell().set_verbosity(Verbosity::Verbose);
    } else {
        gctx.shell().set_verbosity(Verbosity::Normal);
    }

    verbose!(gctx, "Parsing...", "compat workspace");
    let mut skipped = HashSet::new();
    let compat_proj =
        TempProject::from_workspace(&ela_curr, &curr_manifest.to_string_lossy(), &options)?;
    compat_proj.write_manifest_semver(
        curr_workspace.root(),
        compat_proj.temp_dir.path(),
        &ela_curr,
        &mut skipped,
    )?;
    verbose!(gctx, "Updating...", "compat workspace");
    compat_proj.cargo_update()?;
    verbose!(gctx, "Resolving...", "compat workspace");
    let compat_workspace = compat_proj.workspace.borrow();
    let ela_compat = ElaborateWorkspace::from_workspace(
        compat_workspace
            .as_ref()
            .ok_or(OutdatedError::CannotElaborateWorkspace)?,
        &options,
    )?;

    verbose!(gctx, "Parsing...", "latest workspace");
    let latest_proj =
        TempProject::from_workspace(&ela_curr, &curr_manifest.to_string_lossy(), &options)?;
    latest_proj.write_manifest_latest(
        curr_workspace.root(),
        compat_proj.temp_dir.path(),
        &ela_curr,
        &mut skipped,
    )?;
    verbose!(gctx, "Updating...", "latest workspace");
    latest_proj.cargo_update()?;
    verbose!(gctx, "Resolving...", "latest workspace");
    let latest_workspace = latest_proj.workspace.borrow();
    let ela_latest = ElaborateWorkspace::from_workspace(
        latest_workspace
            .as_ref()
            .ok_or(OutdatedError::CannotElaborateWorkspace)?,
        &options,
    )?;

    if ela_curr.workspace_mode {
        let mut sum = 0;
        match options.format {
            Format::List => verbose!(gctx, "Printing...", "Package status in list format"),
            Format::Json => verbose!(gctx, "Printing...", "Package status in json format"),
        }

        for member in ela_curr.workspace.members() {
            ela_curr.resolve_status(
                &ela_compat,
                &ela_latest,
                &options,
                gctx,
                member.package_id(),
                &skipped,
            )?;
            match options.format {
                Format::List => {
                    sum += ela_curr.print_list(&options, member.package_id(), sum > 0, &skipped)?;
                }
                Format::Json => {
                    sum += ela_curr.print_json(&options, member.package_id(), &skipped)?;
                }
            }
        }
        if sum == 0 && matches!(options.format, Format::List) {
            println!("All dependencies are up to date, yay!");
        }
        Ok(sum)
    } else {
        verbose!(gctx, "Resolving...", "package status");
        let root = ela_curr.determine_root(&options)?;
        ela_curr.resolve_status(&ela_compat, &ela_latest, &options, gctx, root, &skipped)?;
        verbose!(gctx, "Printing...", "list format");
        let mut count = 0;

        match options.format {
            Format::List => {
                count = ela_curr.print_list(&options, root, false, &skipped)?;
            }
            Format::Json => {
                ela_curr.print_json(&options, root, &skipped)?;
            }
        }

        Ok(count)
    }
}
