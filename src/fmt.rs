use std::fmt;

#[cfg(all(feature = "color", not(target_os = "windows")))]
use ansi_term::Colour::{Green, Red, Yellow};
#[cfg(all(feature = "color", not(target_os = "windows")))]
use ansi_term::ANSIString;

#[allow(dead_code)]
pub enum Format<T> {
    Error(T),
    Warning(T),
    Good(T),
}

#[cfg(all(feature = "color", not(target_os = "windows")))]
impl<T: AsRef<str>> Format<T> {
    fn format(&self) -> ANSIString {
        match *self {
            Format::Error(ref e) => Red.bold().paint(e.as_ref()),
            Format::Warning(ref e) => Yellow.paint(e.as_ref()),
            Format::Good(ref e) => Green.paint(e.as_ref()),
        }
    }

}

#[cfg(all(feature = "color", not(target_os = "windows")))]
impl<T: AsRef<str>> fmt::Display for Format<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.format())
    }
}

#[cfg(any(not(feature = "color"), target_os = "windows"))]
impl<T: fmt::Display> Format<T> {
    fn format(&self) -> &T {
        match *self {
            Format::Error(ref e) => e,
            Format::Warning(ref e) => e,
            Format::Good(ref e) => e,
        }
    }
}

#[cfg(any(not(feature = "color"), target_os = "windows"))]
impl<T: fmt::Display> fmt::Display for Format<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.format())
    }
}
