macro_rules! verbose {
    ($config: expr, $status: expr, $message: expr) => (
        $config
            .shell()
            .verbose(
                |sh| -> CargoResult<()> { sh.status($status, $message) },
            )?
    )
}

#[cfg(feature = "debug")]
macro_rules! debug {
    ($config: expr, $message: expr) => (
        $config.shell().say($message, ::term::color::WHITE)?
    );
    ($config: expr, $($arg: tt)*) => (
        $config.shell().say(format!($($arg)*), ::term::color::WHITE)?
    );
}

#[cfg(not(feature = "debug"))]
macro_rules! debug {
    ($config: expr, $message: expr) => ();
    ($config: expr, $($arg: tt)*) => ();
}
