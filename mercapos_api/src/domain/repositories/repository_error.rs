#[derive(Debug)]
pub enum RepositoryError {
    NotFound,
    AlreadyExists,
    Conflict,
    ConnectionError(String),
    Unexpected(String),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("Documento no encontrado"),
            Self::AlreadyExists => formatter.write_str("El documento ya existe"),
            Self::Conflict => formatter.write_str("Conflicto de revisión"),
            Self::ConnectionError(message) | Self::Unexpected(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for RepositoryError {}
