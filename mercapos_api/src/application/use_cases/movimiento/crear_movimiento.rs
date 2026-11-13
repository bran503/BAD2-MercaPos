use std::sync::Arc;

use crate::domain::date::valid_rfc3339;
use crate::domain::entities::movimiento::{DireccionMovimiento, MotivoMovimiento, Movimiento};
use crate::domain::repositories::movimiento_repository::MovimientoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

#[derive(Debug)]
pub enum CrearMovimientoError {
    DatosInvalidos(String),
    Repositorio(RepositoryError),
}

impl From<RepositoryError> for CrearMovimientoError {
    fn from(error: RepositoryError) -> Self {
        Self::Repositorio(error)
    }
}

pub struct CrearMovimiento {
    repositorio: Arc<dyn MovimientoRepository>,
}

impl CrearMovimiento {
    pub fn new(repositorio: Arc<dyn MovimientoRepository>) -> Self {
        Self { repositorio }
    }

    pub async fn ejecutar(&self, movimiento: Movimiento) -> Result<(), CrearMovimientoError> {
        if movimiento.id.trim().is_empty() {
            return Err(CrearMovimientoError::DatosInvalidos(
                "El ID es obligatorio".to_string(),
            ));
        }
        if !movimiento.id.starts_with("mov:") {
            return Err(CrearMovimientoError::DatosInvalidos(
                "ID de apertura inválido".into(),
            ));
        }
        if movimiento.motivo == MotivoMovimiento::InventarioInicial
            && movimiento
                .usuario_id
                .as_deref()
                .is_none_or(|id| !id.starts_with("usuario:"))
        {
            return Err(CrearMovimientoError::DatosInvalidos(
                "usuario_id es obligatorio para abrir inventario".into(),
            ));
        }
        if movimiento.producto_id.trim().is_empty() {
            return Err(CrearMovimientoError::DatosInvalidos(
                "El producto es obligatorio".to_string(),
            ));
        }
        if movimiento.puesto_id.trim().is_empty() {
            return Err(CrearMovimientoError::DatosInvalidos(
                "El puesto es obligatorio".to_string(),
            ));
        }
        if movimiento.dispositivo_id.trim().is_empty() {
            return Err(CrearMovimientoError::DatosInvalidos(
                "El dispositivo es obligatorio".to_string(),
            ));
        }
        if !valid_rfc3339(&movimiento.fecha_local) {
            return Err(CrearMovimientoError::DatosInvalidos(
                "fecha_local debe ser RFC 3339 con zona".into(),
            ));
        }
        if !movimiento.cantidad.is_finite() || movimiento.cantidad <= 0.0 {
            return Err(CrearMovimientoError::DatosInvalidos(
                "La cantidad debe ser mayor que cero".to_string(),
            ));
        }
        crate::domain::money::parse_scaled(&movimiento.cantidad.to_string(), 3, 1_000_000_000)
            .map_err(CrearMovimientoError::DatosInvalidos)?;
        let direccion_valida = matches!(
            (&movimiento.motivo, &movimiento.direccion),
            (
                MotivoMovimiento::InventarioInicial,
                DireccionMovimiento::Entrada
            ) | (MotivoMovimiento::Venta, DireccionMovimiento::Salida)
                | (MotivoMovimiento::Compra, DireccionMovimiento::Entrada)
                | (MotivoMovimiento::Merma, DireccionMovimiento::Salida)
                | (MotivoMovimiento::Devolucion, _)
                | (MotivoMovimiento::Ajuste, _)
        );

        if !direccion_valida {
            return Err(CrearMovimientoError::DatosInvalidos(
                "La dirección no corresponde al motivo del movimiento".to_string(),
            ));
        }

        self.repositorio.crear(movimiento).await?;
        Ok(())
    }
}
