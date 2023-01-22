# How to Contribute

Contributions are always welcome! Please use the following guidelines when contributing to `cargo-outdated`

## Justfile

We use [`just`](https://github.com/Casey/just) as a command
runner which should simplify running the same tests on your
changes locally that will be run in CI.

After installing `just` (`cargo install`, via Github Release
binaries, etc.) you can run `just help` to see a list of valid
targets. Most importantly `just ci` should run the entire CI
suit against your changes (except only for your native OS and
architecture).

`just lint` is a good recipe to run while developing to run the
linting and formatting checks prior to trying to run the entire
test suite.

At the time of this writing the recipes look like this:

```
$ just help
Available recipes:
    bench $RUSTFLAGS='-Ctarget-cpu=native' # Run benchmarks
    ci                  # Run all the checks required for CI to pass
    clean
    debug TEST
    default
    fmt                 # Format the code
    fmt-check           # Check the formatting of the code but don't actually format it
    help                # Get a list of recipes you can run
    lint                # Lint the code
    run-test TEST
    run-tests
    setup               # Install required tools for development
    spell-check         # Check for typos
    test TEST_RUNNER='cargo nextest run' # Run the test suite
    update-contributors

```

## Commit Subjects

As you make your commit messages; please note that we use a [conventional](https://github.com/ajoslin/conventional-changelog/blob/master/CONVENTIONS.md) changelog format so we can update my changelog using [clog](https://github.com/clog-tool/clog-cli)

* Format your commit subject line using the following format: `TYPE(COMPONENT): MESSAGE` where `TYPE` is one of the following:
  * `feat` - A new feature
  * `imp` - An improvement to an existing feature
  * `perf` - A performance improvement
  * `docs` - Changes to documentation only
  * `tests` - Changes to the testing framework or tests only
  * `fix` - A bug fix
  * `refactor` - Code functionality doesn't change, but underlying structure may
  * `style` - Stylistic changes only, no functionality changes
  * `wip` - A work in progress commit (Should typically be `git rebase`'ed away)
  * `chore` - Catch all or things that have to do with the build system, etc
* The `COMPONENT` is optional, and may be a single file, directory, or logical component. Can be omitted if commit applies globally

