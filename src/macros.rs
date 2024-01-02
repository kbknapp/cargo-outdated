macro_rules! verbose {
    ($config:expr, $status:expr, $message:expr) => {
        $config
            .shell()
            .verbose(|sh| -> CargoResult<()> { sh.status($status, $message) })?
    };
}

#[cfg(feature = "debug")]
macro_rules! debug {
    ($config: expr, $message: expr) => (
        $config.shell().status_with_color("DEBUG", $message, &Default::default())?
    );
    ($config: expr, $($arg: tt)*) => (
        $config.shell().status_with_color("DEBUG", format!($($arg)*), &Default::default())?
    );
}

#[cfg(not(feature = "debug"))]
macro_rules! debug {
    ($config:expr, $message:expr) => {};
    ($config:expr, $($arg:tt)*) => {};
}
