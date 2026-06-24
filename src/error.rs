use std::error::Error;
use std::fmt;

#[derive(Clone, Debug)]
pub enum AvroError {
    InvalidASTDataType(String),
    FailedParsing(String),
    MissingName(String),
    UndefinedReference(String),
    Io(String),
    CircularImport(String),
}

impl fmt::Display for AvroError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AvroError::InvalidASTDataType(message) => write!(f, "{}", message),
            AvroError::FailedParsing(message) => write!(f, "{}", message),
            AvroError::MissingName(message) => write!(f, "{}", message),
            AvroError::UndefinedReference(message) => write!(f, "{}", message),
            AvroError::Io(message) => write!(f, "IO error: {}", message),
            AvroError::CircularImport(path) => {
                write!(f, "Circular import detected: {}", path)
            }
        }
    }
}

impl Error for AvroError {}
