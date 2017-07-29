macro_rules! wlnerr(
    ($($arg:tt)*) => ({
        use std::io::{Write, stderr};
        writeln!(&mut stderr(), $($arg)*).ok();
    })
);

#[allow(unused_macros)]
macro_rules! werr(
    ($($arg:tt)*) => ({
        use std::io::{Write, stderr};
        write!(&mut stderr(), $($arg)*).ok();
    })
);

macro_rules! verbose(
    ($cfg:ident, $($arg:tt)*) => ({
        if $cfg.verbose {
            use std::io::{Write, stdout};
            write!(&mut stdout(), $($arg)*).ok();
        }
    })
);

macro_rules! verboseln(
    ($cfg:ident, $($arg:tt)*) => ({
        if $cfg.verbose {
            use std::io::{Write, stdout};
            writeln!(&mut stdout(), $($arg)*).ok();
        }
    })
);

#[cfg(feature = "debug")]
macro_rules! debugln {
    ($fmt:expr) => (println!(concat!("*DEBUG:cargo-outdated:", $fmt)));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("*DEBUG:cargo-outdated:",$fmt), $($arg)*));
}

#[cfg(feature = "debug")]
macro_rules! debug {
    ($fmt:expr) => (print!(concat!("*DEBUG:cargo-outdated:", $fmt)));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("*DEBUG:cargo-outdated:",$fmt), $($arg)*));
}

#[cfg(not(feature = "debug"))]
macro_rules! debugln {
    ($fmt:expr) => ();
    ($fmt:expr, $($arg:tt)*) => ();
}

#[allow(unused_macros)]
#[cfg(not(feature = "debug"))]
macro_rules! debug {
    ($fmt:expr) => ();
    ($fmt:expr, $($arg:tt)*) => ();
}
