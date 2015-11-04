# cargo-outdated

[![Join the chat at https://gitter.im/kbknapp/cargo-outdated](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/kbknapp/cargo-outdated?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)
Linux: [![Build Status](https://travis-ci.org/kbknapp/cargo-outdated.svg?branch=master)](https://travis-ci.org/kbknapp/cargo-outdated)

A cargo subcommand for displaying when Rust dependencies are out of date

## About

`cargo-outdated` is a very early proof-of-concept for displaying when dependencies have newer versions available.

## Demo

Once installed (see below) running `cargo outdated` in a project directory looks like the following:

```
$ cargo outdated
Checking for SemVer compatible updates...Done
Checking for the latest updates...Done
The following dependencies have newer versions available:

    Name                 Project Ver  SemVer Compat  Latest Ver
    regex->regex-syntax     0.2.1        0.2.2         0.2.2
    regex->memchr           0.1.5        0.1.6         0.1.6
    clap                    1.2.3        1.2.5         1.4.7
    tabwriter               0.1.23       0.1.24        0.1.24
    clippy                  0.0.11       0.0.22        0.0.22
    clap->ansi_term         0.6.3          --          0.7.0
    regex->aho-corasick     0.3.0        0.3.4         0.4.0
    ansi_term               0.6.3          --          0.7.0
```

## Installing

`cargo-outdated` can be installed with `cargo install`

```
$ cargo install cargo-outdated
```

This may require a nightly version of `cargo` if you get an error about the `install` command not being found. You may also compile and install the traditional way by followin the instructions below.

## Compiling

Follow these instructions to compile `cargo-outdated`, then skip down to Installation.

 1. Ensure you have current version of `cargo` and [Rust](https://www.rust-lang.org) installed
 2. Clone the project `$ git clone https://github.com/kbknapp/cargo-outdated && cd cargo-outdated`
 3. Build the project `$ cargo build --release`
 4. Once complete, the binary will be located at `target/release/cargo-outdated`

## Installation and Usage

All you need to do is place `cargo-outdated` somewhere in your `$PATH`. Then run `cargo outdated` anywhere in your project directory. For full details see below.

### Linux / OS X

You have two options, place `cargo-outdated` into a directory that is already located in your `$PATH` variable (To see which directories those are, open a terminal and type `echo "${PATH//:/\n}"`, the quotation marks are important), or you can add a custom directory to your `$PATH`

**Option 1**
If you have write permission to a directory listed in your `$PATH` or you have root permission (or via `sudo`), simply copy the `cargo-outdated` to that directory `# sudo cp cargo-outdated /usr/local/bin`

**Option 2**
If you do not have root, `sudo`, or write permission to any directory already in `$PATH` you can create a directory inside your home directory, and add that. Many people use `$HOME/.bin` to keep it hidden (and not clutter your home directory), or `$HOME/bin` if you want it to be always visible. Here is an example to make the directory, add it to `$PATH`, and copy `cargo-outdated` there.

Simply change `bin` to whatever you'd like to name the directory, and `.bashrc` to whatever your shell startup file is (usually `.bashrc`, `.bash_profile`, or `.zshrc`)

```sh
$ mkdir ~/bin
$ echo "export PATH=$PATH:$HOME/bin" >> ~/.bashrc
$ cp cargo-outdated ~/bin
$ source ~/.bashrc
```

### Windows

On Windows 7/8 you can add directory to the `PATH` variable by opening a command line as an administrator and running

```sh
C:\> setx path "%path%;C:\path\to\cargo-outdated\binary"
```

Otherwise, ensure you have the `cargo-outdated` binary in the directory which you operating in the command line from, because Windows automatically adds your current directory to PATH (i.e. if you open a command line to `C:\my_project\` to use `cargo-outdated` ensure `cargo-outdated.exe` is inside that directory as well).


### Options

There are a few options for using `cargo-outdated` which should be somewhat self explanitory.

```
USAGE:
    cargo outdated [FLAGS] [OPTIONS]

FLAGS:
    -h, --help              Prints help information
    -R, --root-deps-only    Only check root dependencies (Equivilant to --depth=1)
    -V, --version           Prints version information
    -v, --verbose           Print verbose output

OPTIONS:
    -d, --depth <DEPTH>       How deep in the dependency chain to search
                              (Defaults to all dependencies when omitted)
    -p, --package <PKG>...    Package to inspect for updates
```

## License

`cargo-outdated` is released under the terms of either the MIT or Apache 2.0 license. See the LICENSE-MIT or LICENSE-APACHE file for the details.

## Dependencies Tree
![cargo-outdated dependencies](cargo-outdated.png)