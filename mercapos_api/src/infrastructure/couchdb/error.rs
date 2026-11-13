use std::fmt;

#[derive(Debug)]
pub enum CouchDbError {
    Http(reqwest::Error),
    Configuration(String),
    Server { status: u16, message: String },
}

impl fmt::Display for CouchDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(error) => {
                write!(f, "Error HTTP: {}", error)
            }

            Self::Configuration(message) => {
                write!(f, "Error de configuración: {}", message)
            }

            Self::Server { status, message } => {
                write!(f, "CouchDB respondió {}: {}", status, message)
            }
        }
    }
}

impl std::error::Error for CouchDbError {}

impl From<reqwest::Error> for CouchDbError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}
