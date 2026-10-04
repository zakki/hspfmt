#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    message: String,
    line: Option<usize>,
    byte_offset: Option<usize>,
}

impl Error {
    pub(crate) fn new(line: Option<usize>, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line,
            byte_offset: None,
        }
    }
    /// Zero-based byte offset in the original input, when available.
    pub fn byte_offset(&self) -> Option<usize> {
        self.byte_offset
    }
    pub(crate) fn with_offset(mut self, offset: usize) -> Self {
        self.byte_offset = Some(offset);
        self
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    /// One-based line, or None for an error without a source location.
    pub fn line(&self) -> Option<usize> {
        self.line
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line) = self.line {
            write!(f, "line {}: {}", line, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for Error {}
