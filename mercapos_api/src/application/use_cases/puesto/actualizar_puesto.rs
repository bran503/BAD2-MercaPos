use std::sync::Arc;

use crate::domain::entities::puesto::Puesto;
use crate::domain::repositories::puesto_repository::PuestoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

#[derive(Debug)]
pub enum ActualizarPuestoError {
    DatosInvalidos(String),
    Repositorio(RepositoryError),
}

impl From<RepositoryError> for ActualizarPuestoError {
    fn from(error: RepositoryError) -> Self {
        Self::Repositorio(error)
    }
}

pub struct ActualizarPuesto {
    repositorio: Arc<dyn PuestoRepository>,
}

impl ActualizarPuesto {
    pub fn new(repositorio: Arc<dyn PuestoRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, puesto: Puesto) -> Result<(), ActualizarPuestoError> {
        if puesto.id.trim().is_empty() {
            return Err(ActualizarPuestoError::DatosInvalidos(
                "El ID es obligatorio".to_string(),
            ));
        }
        if puesto.nombre.trim().is_empty() {
            return Err(ActualizarPuestoError::DatosInvalidos(
                "El nombre es obligatorio".to_string(),
            ));
        }
        if puesto.mercado.trim().is_empty() {
            return Err(ActualizarPuestoError::DatosInvalidos(
                "El mercado es obligatorio".to_string(),
            ));
        }
        if puesto.ubicacion.trim().is_empty() {
            return Err(ActualizarPuestoError::DatosInvalidos(
                "La ubicación es obligatoria".to_string(),
            ));
        }
        if puesto.propietario_usuario_id.trim().is_empty() {
            return Err(ActualizarPuestoError::DatosInvalidos(
                "El propietario es obligatorio".to_string(),
            ));
        }

        self.repositorio.actualizar(puesto).await?;
        Ok(())
    }
}
