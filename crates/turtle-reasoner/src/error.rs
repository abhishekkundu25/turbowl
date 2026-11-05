//! Errors of the interface (ticket 02 fills the variants' detail).

use std::fmt;

/// An input or output failure. Reasoning itself does not fail: data problems are diagnostics.
#[derive(Debug)]
pub enum Error {
    /// Reading or writing a file failed.
    Io {
        path: Option<std::path::PathBuf>,
        source: std::io::Error,
    },
    /// A document is not valid in its format.
    Parse {
        path: Option<std::path::PathBuf>,
        line: u64,
        column: u64,
        message: String,
    },
    /// A file's extension names no supported format.
    UnknownFormat { path: std::path::PathBuf },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let at = |p: &Option<std::path::PathBuf>| p.as_ref().map(|p| format!("{}: ", p.display())).unwrap_or_default();
        match self {
            Error::Io { path, source } => write!(f, "{}{source}", at(path)),
            Error::Parse {
                path,
                line,
                column,
                message,
            } => write!(f, "{}{line}:{column}: {message}", at(path)),
            Error::UnknownFormat { path } => write!(
                f,
                "{}: unknown format (use .ttl, .nt, .n3, .rdf or .owl)",
                path.display()
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
