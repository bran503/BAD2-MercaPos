use std::sync::Arc;

use crate::domain::date::valid_rfc3339;
use crate::domain::entities::conteo_fisico::ConteoFisico;
use crate::domain::money::parse_scaled;
use crate::domain::repositories::conteo_repository::ConteoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

#[derive(Debug)]
pub enum CrearConteoError {
    DatosInvalidos(String),
    Repositorio(RepositoryError),
}

impl From<RepositoryError> for CrearConteoError {
    fn from(error: RepositoryError) -> Self {
        Self::Repositorio(error)
    }
}

pub struct CrearConteo {
    repositorio: Arc<dyn ConteoRepository>,
}

impl CrearConteo {
    pub fn new(repositorio: Arc<dyn ConteoRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, conteo: ConteoFisico) -> Result<(), CrearConteoError> {
        if conteo.id.trim().is_empty() {
            return Err(CrearConteoError::DatosInvalidos(
                "El ID es obligatorio".to_string(),
            ));
        }
        if conteo.puesto_id.trim().is_empty() {
            return Err(CrearConteoError::DatosInvalidos(
                "El puesto es obligatorio".to_string(),
            ));
        }
        if conteo.usuario_id.trim().is_empty() {
            return Err(CrearConteoError::DatosInvalidos(
                "El usuario es obligatorio".to_string(),
            ));
        }
        if conteo.dispositivo_id.trim().is_empty() {
            return Err(CrearConteoError::DatosInvalidos(
                "El dispositivo es obligatorio".to_string(),
            ));
        }
        if !valid_rfc3339(&conteo.fecha_local) {
            return Err(CrearConteoError::DatosInvalidos(
                "fecha_local debe ser RFC 3339 con zona".into(),
            ));
        }
        if conteo.renglones.is_empty() {
            return Err(CrearConteoError::DatosInvalidos(
                "Debe haber al menos un renglón".to_string(),
            ));
        }
        if conteo
            .periodo_estudio
            .as_deref()
            .is_none_or(|p| p.trim().is_empty())
            || !matches!(conteo.etapa.as_deref(), Some("antes" | "despues"))
            || !matches!(conteo.origen_registro.as_deref(), Some("cuaderno" | "pos"))
        {
            return Err(CrearConteoError::DatosInvalidos(
                "Período, etapa u origen inválido".into(),
            ));
        }
        if conteo.origen_registro.as_deref() == Some("cuaderno")
            && conteo
                .evidencia
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(CrearConteoError::DatosInvalidos(
                "Un conteo de cuaderno requiere evidencia".into(),
            ));
        }
        if conteo.origen_registro.as_deref() == Some("pos") && conteo.corte_seq.is_none() {
            return Err(CrearConteoError::DatosInvalidos(
                "El conteo POS requiere corte de servidor".into(),
            ));
        }

        for renglon in &conteo.renglones {
            if renglon.producto_id.trim().is_empty() {
                return Err(CrearConteoError::DatosInvalidos(
                    "El producto de cada renglón es obligatorio".to_string(),
                ));
            }
            if !renglon.cantidad_sistema.is_finite()
                || !renglon.cantidad_contada.is_finite()
                || renglon.cantidad_contada < 0.0
                || !renglon.diferencia.is_finite()
            {
                return Err(CrearConteoError::DatosInvalidos(
                    "Las cantidades del conteo deben ser números finitos y no negativos"
                        .to_string(),
                ));
            }
            let system = parse_scaled(&renglon.cantidad_sistema.to_string(), 3, i64::MAX)
                .map_err(CrearConteoError::DatosInvalidos)?;
            let counted = parse_scaled(&renglon.cantidad_contada.to_string(), 3, i64::MAX)
                .map_err(CrearConteoError::DatosInvalidos)?;
            let difference = parse_scaled(&renglon.diferencia.to_string(), 3, i64::MAX)
                .map_err(CrearConteoError::DatosInvalidos)?;
            if counted.checked_sub(system) != Some(difference) {
                return Err(CrearConteoError::DatosInvalidos(
                    "La diferencia debe ser cantidad_contada - cantidad_sistema".to_string(),
                ));
            }
        }

        self.repositorio.crear(conteo).await?;
        Ok(())
    }
}
