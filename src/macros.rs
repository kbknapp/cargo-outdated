macro_rules! verbose {
    ($context:expr, $status:expr, $message:expr) => {
        $context
            .shell()
            .verbose(|sh| -> CargoResult<()> { sh.status($status, $message) })?
    };
}

#[cfg(feature = "debug")]
macro_rules! debug {
    ($context: expr, $message: expr) => (
        $context.shell().status_with_color("DEBUG", $message, &Default::default())?
    );
    ($context: expr, $($arg: tt)*) => (
        $context.shell().status_with_color("DEBUG", format!($($arg)*), &Default::default())?
    );
}

#[cfg(not(feature = "debug"))]
macro_rules! debug {
    ($context:expr, $message:expr) => {};
    ($context:expr, $($arg:tt)*) => {};
}
