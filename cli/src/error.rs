use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use csv::ErrorKind;
use crate::error::ErrorType::{CsvRead, IO};
use crate::ErrorType::{CsvOther, CsvWrite};

#[derive(Debug)]
pub enum ErrorType {
    CliParseError,
    IO,
    CsvRead,
    CsvWrite,
    CsvOther,
}

pub struct CliError {
    message: String,
    error_type: ErrorType,
}

impl CliError {
    pub fn new<T>(error_type: ErrorType, message: T) -> Self
        where T: ToString
    {
        CliError {
            error_type,
            message: message.to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[allow(dead_code)]
    pub fn error_type(&self) -> &ErrorType {
        &self.error_type
    }
}

impl Debug for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "App error type: {:?}, Message: {}", self.error_type, self.message)
    }
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for CliError {}

impl From<csv::Error> for CliError {
    fn from(e: csv::Error) -> Self {
        match e.kind() {
            ErrorKind::Io(e) => CliError::new(IO, e.to_string()),
            ErrorKind::Serialize(e) => CliError::new(CsvWrite, e),
            ErrorKind::Deserialize { .. } => CliError::new(CsvRead, e.to_string()),
            _ => CliError::new(CsvOther, e.to_string())
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::new(IO, e.to_string())
    }
}