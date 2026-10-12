//! `%` compiler directives (ref 4.7, p. 4-43): `%include`, `%if`, ...
//! Stub: passes the source through unchanged.

use std::path::PathBuf;

use crate::diag::Diagnostic;
use crate::source::{FileId, SourceMap};

/// Preprocessor options.
#[derive(Debug, Default)]
pub struct Options {
    /// `%include` search directories (`-I`), in order.
    pub include_dirs: Vec<PathBuf>,
}

/// Run the preprocessor over `file`.  Stub: returns the text unchanged.
pub fn preprocess(
    sm: &SourceMap,
    file: FileId,
    _opts: &Options,
) -> Result<String, Vec<Diagnostic>> {
    Ok(sm.file(file).text.clone())
}
