use clap::ArgMatches;

#[derive(Debug)]
pub struct Config<'tu> {
    to_update: Option<Vec<&'tu str>>,
    depth: u8,
    pub verbose: bool
}

impl<'tu> Config<'tu> {
    pub fn from_matches(m: &'tu ArgMatches) -> Self {
        Config {
            to_update: m.values_of("PKG"),
            depth: m.value_of("DEPTH").unwrap_or("1").parse().unwrap_or(1),
            verbose: m.is_present("verbose")
        }
    }
}
