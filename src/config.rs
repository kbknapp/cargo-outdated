use clap::ArgMatches;

use fmt::Format;

#[derive(Debug)]
pub struct Config<'tu> {
    pub to_update: Option<Vec<&'tu str>>,
    pub depth: i32,
    pub verbose: bool
}

impl<'tu> Config<'tu> {
    pub fn from_matches(m: &'tu ArgMatches) -> Self {
        let depth = match m.value_of("DEPTH") {
            Some(d_str) => {
                match d_str.parse::<u8>() {
                    Ok(num) => num as i32,
                    Err(..)  => {
                        wlnerr!("{} Couldn't parse '{}' as a valid depth (Valid depths are 1-255)", Format::Error("error:"), d_str);
                        ::std::process::exit(1);
                    }
                }
            },
            None => if m.is_present("root-deps-only") { 1 } else { 0 }
        };

        Config {
            to_update: m.values_of("PKG"),
            depth: depth,
            verbose: m.is_present("verbose")
        }
    }
}
