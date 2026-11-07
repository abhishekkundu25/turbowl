//! Errors of the interface.

use std::fmt;

/// An input or output failure. Reasoning itself does not fail: data problems are diagnostics.
#[derive(Debug)]
pub enum Error {
    /// Reading or writing a file failed.
    Io {
        /// The file; `None` for a string input or a caller's writer.
        path: Option<std::path::PathBuf>,
        source: std::io::Error,
    },
    /// A document is not valid in its format.
    Parse {
        /// The file; `None` for a string input.
        path: Option<std::path::PathBuf>,
        /// The 1-based line of the error; 0 when the error is not at a
        /// position in the text (an invalid base IRI).
        line: u64,
        /// The 1-based column, in characters; 0 with line 0.
        column: u64,
        /// The parser's message.
        message: String,
    },
    /// A file's extension names no supported format.
    UnknownFormat { path: std::path::PathBuf },
    /// A snapshot is refused ([`Reasoner::from_snapshot_bytes`](crate::Reasoner::from_snapshot_bytes)):
    /// it is not a snapshot, it is of another format version or another
    /// TurbOWL version, or it is truncated, damaged or malformed.
    Snapshot {
        /// The file; `None` for bytes given.
        path: Option<std::path::PathBuf>,
        /// Why, as one sentence: "the snapshot is damaged: its checksum
        /// does not match its contents".
        message: String,
    },
    /// The base or the switches changed since the last run, so there is no
    /// closure to explain until [`Reasoner::reason`](crate::Reasoner::reason)
    /// runs ([`Reasoner::explainer`](crate::Reasoner::explainer)).
    NotReasoned,
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
            } if *line == 0 => write!(f, "{}{message}", at(path)),
            Error::Parse {
                path,
                line,
                column,
                message,
            } => write!(f, "{}{line}:{column}: {message}", at(path)),
            Error::Snapshot { path, message } => write!(f, "{}{message}", at(path)),
            Error::NotReasoned => write!(f, "the reasoner changed since its last run: reason() first"),
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
