use std::{collections::HashMap, sync::Arc};

use crate::domain::date::valid_rfc3339;
use crate::domain::entities::movimiento::DireccionMovimiento;
use crate::domain::entities::operacion::{Operacion, TipoDevolucion, TipoOperacion};
use crate::domain::money::{parse_scaled, subtotal_centavos};
use crate::domain::repositories::movimiento_repository::MovimientoRepository;
use crate::domain::repositories::operacion_repository::OperacionRepository;
use crate::domain::repositories::producto_repository::ProductoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

#[derive(Debug)]
pub enum CrearOperacionError {
    DatosInvalidos(String),
    StockInsuficiente(String),
    Repositorio(RepositoryError),
}

impl From<RepositoryError> for CrearOperacionError {
    fn from(error: RepositoryError) -> Self {
        Self::Repositorio(error)
    }
}

pub struct CrearOperacion {
    repositorio: Arc<dyn OperacionRepository>,
    movimiento_repositorio: Arc<dyn MovimientoRepository>,
    producto_repositorio: Arc<dyn ProductoRepository>,
    permitir_stock_negativo: bool,
}

impl CrearOperacion {
    pub fn new(
        repositorio: Arc<dyn OperacionRepository>,
        movimiento_repositorio: Arc<dyn MovimientoRepository>,
        producto_repositorio: Arc<dyn ProductoRepository>,
    ) -> Self {
        Self {
            repositorio,
            movimiento_repositorio,
            producto_repositorio,
            permitir_stock_negativo: false,
        }
    }

    pub fn permitir_stock_negativo(mut self, permitir: bool) -> Self {
        self.permitir_stock_negativo = permitir;
        self
    }

    pub async fn ejecutar(&self, operacion: Operacion) -> Result<(), CrearOperacionError> {
        if operacion.id.trim().is_empty() {
            return Err(CrearOperacionError::DatosInvalidos(
                "El ID es obligatorio".to_string(),
            ));
        }

        if operacion.puesto_id.trim().is_empty() {
            return Err(CrearOperacionError::DatosInvalidos(
                "El puesto es obligatorio".to_string(),
            ));
        }

        if operacion.usuario_id.trim().is_empty() {
            return Err(CrearOperacionError::DatosInvalidos(
                "El usuario es obligatorio".to_string(),
            ));
        }

        if operacion.dispositivo_id.trim().is_empty() {
            return Err(CrearOperacionError::DatosInvalidos(
                "El dispositivo es obligatorio".to_string(),
            ));
        }

        if !valid_rfc3339(&operacion.fecha_local) {
            return Err(CrearOperacionError::DatosInvalidos(
                "fecha_local debe ser RFC 3339 con zona horaria".to_string(),
            ));
        }

        if operacion.renglones.is_empty() {
            return Err(CrearOperacionError::DatosInvalidos(
                "Debe haber al menos un renglón".to_string(),
            ));
        }

        match (&operacion.tipo_operacion, &operacion.tipo_devolucion) {
            (TipoOperacion::Devolucion, None) => {
                return Err(CrearOperacionError::DatosInvalidos(
                    "Debe especificar si la devolución es de un cliente o proveedor".to_string(),
                ));
            }

            (TipoOperacion::Devolucion, Some(_)) => {}

            (_, Some(_)) => {
                return Err(CrearOperacionError::DatosInvalidos(
                    "El tipo de devolución solo aplica a operaciones de devolución".to_string(),
                ));
            }

            (_, None) => {}
        }
        match (
            &operacion.tipo_operacion,
            operacion.destino_devolucion.as_deref(),
        ) {
            (TipoOperacion::Devolucion, None | Some("vendible")) => {}
            (TipoOperacion::Devolucion, Some("no_vendible"))
                if operacion.tipo_devolucion == Some(TipoDevolucion::Cliente) => {}
            (TipoOperacion::Devolucion, _) => {
                return Err(CrearOperacionError::DatosInvalidos(
                    "Destino de devolución inválido".into(),
                ));
            }
            (_, Some(_)) => {
                return Err(CrearOperacionError::DatosInvalidos(
                    "destino_devolucion solo aplica a devoluciones".into(),
                ));
            }
            (_, None) => {}
        }
        // Validar cada renglón
        for renglon in &operacion.renglones {
            if renglon.producto_id.trim().is_empty() {
                return Err(CrearOperacionError::DatosInvalidos(
                    "El producto del renglón es obligatorio".to_string(),
                ));
            }

            let cantidad_invalida = !renglon.cantidad.is_finite()
                || renglon.cantidad == 0.0
                || (!matches!(operacion.tipo_operacion, TipoOperacion::Ajuste)
                    && renglon.cantidad < 0.0);
            if cantidad_invalida {
                return Err(CrearOperacionError::DatosInvalidos(
                    "La cantidad debe ser distinta de cero; solo un ajuste puede usar cantidad negativa".to_string(),
                ));
            }

            if let Some(precio) = renglon.precio_unitario
                && (!precio.is_finite() || precio < 0.0)
            {
                return Err(CrearOperacionError::DatosInvalidos(
                    "El precio unitario no puede ser negativo ni inválido".to_string(),
                ));
            }

            if let Some(costo) = renglon.costo_unitario
                && (!costo.is_finite() || costo < 0.0)
            {
                return Err(CrearOperacionError::DatosInvalidos(
                    "El costo unitario no puede ser negativo ni inválido".to_string(),
                ));
            }
        }

        match operacion.tipo_operacion {
            TipoOperacion::Venta => {
                if operacion.total.is_none()
                    || operacion
                        .renglones
                        .iter()
                        .any(|r| r.precio_unitario.is_none())
                {
                    return Err(CrearOperacionError::DatosInvalidos(
                        "Una venta requiere total y precio_unitario en cada renglón".to_string(),
                    ));
                }
            }
            TipoOperacion::Compra => {
                if operacion.total.is_none()
                    || operacion
                        .renglones
                        .iter()
                        .any(|r| r.costo_unitario.is_none())
                {
                    return Err(CrearOperacionError::DatosInvalidos(
                        "Una compra requiere total y costo_unitario en cada renglón".to_string(),
                    ));
                }
            }
            TipoOperacion::Merma => {
                if operacion
                    .motivo
                    .as_deref()
                    .is_none_or(|m| m.trim().is_empty())
                {
                    return Err(CrearOperacionError::DatosInvalidos(
                        "El motivo es obligatorio para registrar una merma".to_string(),
                    ));
                }
            }
            TipoOperacion::Devolucion => {
                if operacion.total.is_none()
                    || operacion.renglones.iter().any(|r| {
                        if operacion.tipo_devolucion == Some(TipoDevolucion::Proveedor) {
                            r.costo_unitario.is_none()
                        } else {
                            r.precio_unitario.is_none()
                        }
                    })
                {
                    return Err(CrearOperacionError::DatosInvalidos(
                        "Una devolución requiere total y precio_unitario en cada renglón"
                            .to_string(),
                    ));
                }
                if operacion
                    .operacion_origen_id
                    .as_deref()
                    .is_none_or(|id| id.trim().is_empty())
                {
                    return Err(CrearOperacionError::DatosInvalidos(
                        "operacion_origen_id es obligatorio para una devolución".to_string(),
                    ));
                }
            }
            TipoOperacion::Ajuste => {}
        }

        if let Some(total) = operacion.total
            && (!total.is_finite() || total < 0.0)
        {
            return Err(CrearOperacionError::DatosInvalidos(
                "El total no puede ser negativo ni inválido".to_string(),
            ));
        }
        if matches!(
            operacion.tipo_operacion,
            TipoOperacion::Venta | TipoOperacion::Compra | TipoOperacion::Devolucion
        ) {
            let mut cents = 0_i64;
            for line in &operacion.renglones {
                let quantity = parse_scaled(&line.cantidad.to_string(), 3, 1_000_000_000)
                    .map_err(CrearOperacionError::DatosInvalidos)?;
                let unit = match operacion.tipo_operacion {
                    TipoOperacion::Compra => line.costo_unitario,
                    TipoOperacion::Devolucion
                        if operacion.tipo_devolucion == Some(TipoDevolucion::Proveedor) =>
                    {
                        line.costo_unitario
                    }
                    _ => line.precio_unitario,
                }
                .ok_or_else(|| {
                    CrearOperacionError::DatosInvalidos("Falta precio o costo unitario".into())
                })?;
                let price = parse_scaled(&unit.to_string(), 4, 10_000_000_000)
                    .map_err(CrearOperacionError::DatosInvalidos)?;
                let line_cents = subtotal_centavos(quantity, price)
                    .map_err(CrearOperacionError::DatosInvalidos)?;
                cents = cents.checked_add(line_cents).ok_or_else(|| {
                    CrearOperacionError::DatosInvalidos("Importe fuera de rango".into())
                })?;
            }
            if operacion.total_centavos != Some(cents) {
                return Err(CrearOperacionError::DatosInvalidos(format!(
                    "Total calculado {cents} centavos, enviado {:?}",
                    operacion.total_centavos
                )));
            }
            if operacion.total.is_some_and(|total| {
                parse_scaled(&total.to_string(), 2, 10_000_000_000).ok() != Some(cents)
            }) {
                return Err(CrearOperacionError::DatosInvalidos(
                    "El total no coincide con los renglones".into(),
                ));
            }
        }
        if matches!(operacion.tipo_operacion, TipoOperacion::Ajuste) {
            if operacion
                .motivo
                .as_deref()
                .is_none_or(|m| m.trim().is_empty())
            {
                return Err(CrearOperacionError::DatosInvalidos(
                    "El motivo es obligatorio para un ajuste".to_string(),
                ));
            }
            if operacion
                .conteo_id
                .as_deref()
                .is_some_and(|id| id.trim().is_empty())
            {
                return Err(CrearOperacionError::DatosInvalidos(
                    "El conteo_id no puede estar vacío".to_string(),
                ));
            }
        }

        if let Some(existing) = self.repositorio.obtener_por_id(&operacion.id).await? {
            if !existing.misma_solicitud(&operacion) {
                return Err(CrearOperacionError::Repositorio(
                    RepositoryError::AlreadyExists,
                ));
            }
            // A version 2 operation contains every stock effect in one document.
            // Legacy operations are only acknowledged when all their old movement
            // documents already exist. Missing effects require audited migration.
            let movements = self
                .movimiento_repositorio
                .listar_por_puesto(&operacion.puesto_id)
                .await?;
            let mut effects = movements
                .into_iter()
                .filter(|m| m.operacion_id.as_deref() == Some(operacion.id.as_str()))
                .collect::<Vec<_>>();
            let expected_count = if operacion.destino_devolucion.as_deref() == Some("no_vendible") {
                0
            } else {
                operacion.renglones.len()
            };
            if effects.len() != expected_count {
                return Err(CrearOperacionError::Repositorio(RepositoryError::Conflict));
            }
            for line in operacion.renglones.iter().take(expected_count) {
                let outgoing =
                    matches!(
                        operacion.tipo_operacion,
                        TipoOperacion::Venta | TipoOperacion::Merma
                    ) || matches!(operacion.tipo_devolucion, Some(TipoDevolucion::Proveedor))
                        || (matches!(operacion.tipo_operacion, TipoOperacion::Ajuste)
                            && line.cantidad < 0.0);
                let expected = if outgoing {
                    DireccionMovimiento::Salida
                } else {
                    DireccionMovimiento::Entrada
                };
                let index = effects
                    .iter()
                    .position(|m| {
                        m.producto_id == line.producto_id
                            && m.cantidad == line.cantidad.abs()
                            && m.direccion == expected
                    })
                    .ok_or(CrearOperacionError::Repositorio(RepositoryError::Conflict))?;
                effects.swap_remove(index);
            }
            return Ok(());
        }

        for line in &operacion.renglones {
            match self
                .producto_repositorio
                .obtener_por_id(&line.producto_id)
                .await?
            {
                Some(producto) if producto.activo && producto.puesto_id == operacion.puesto_id => {}
                Some(_) => {
                    return Err(CrearOperacionError::DatosInvalidos(format!(
                        "El producto {} está inactivo",
                        line.producto_id
                    )));
                }
                None => {
                    return Err(CrearOperacionError::DatosInvalidos(format!(
                        "El producto {} no existe",
                        line.producto_id
                    )));
                }
            }
        }

        // Apply outgoing stock checks to the current movement ledger. Totals are
        // grouped per product so duplicate lines cannot bypass the validation.
        let mut outgoing: HashMap<String, i64> = HashMap::new();
        for line in &operacion.renglones {
            let quantity = match operacion.tipo_operacion {
                TipoOperacion::Venta | TipoOperacion::Merma => line.cantidad.abs(),
                TipoOperacion::Devolucion
                    if matches!(operacion.tipo_devolucion, Some(TipoDevolucion::Proveedor)) =>
                {
                    line.cantidad.abs()
                }
                TipoOperacion::Ajuste if line.cantidad.is_sign_negative() => line.cantidad.abs(),
                _ => 0.0,
            };
            if quantity > 0.0 {
                let scaled = parse_scaled(&quantity.to_string(), 3, 1_000_000_000)
                    .map_err(CrearOperacionError::DatosInvalidos)?;
                let prior = outgoing.get(&line.producto_id).copied().unwrap_or_default();
                let total = prior.checked_add(scaled).ok_or_else(|| {
                    CrearOperacionError::DatosInvalidos("Cantidad fuera de rango".into())
                })?;
                outgoing.insert(line.producto_id.clone(), total);
            }
        }
        if !self.permitir_stock_negativo && !outgoing.is_empty() {
            let movements = self
                .movimiento_repositorio
                .listar_por_puesto(&operacion.puesto_id)
                .await?;
            let mut available: HashMap<String, i64> = HashMap::new();
            for movement in movements {
                let magnitude = parse_scaled(&movement.cantidad.to_string(), 3, i64::MAX)
                    .map_err(CrearOperacionError::DatosInvalidos)?;
                let signed = if matches!(movement.direccion, DireccionMovimiento::Salida) {
                    -magnitude
                } else {
                    magnitude
                };
                let prior = available
                    .get(&movement.producto_id)
                    .copied()
                    .unwrap_or_default();
                let total = prior.checked_add(signed).ok_or_else(|| {
                    CrearOperacionError::DatosInvalidos("Existencia fuera de rango".into())
                })?;
                available.insert(movement.producto_id, total);
            }
            for (product_id, quantity) in outgoing {
                let stock = available.get(&product_id).copied().unwrap_or_default();
                if stock < quantity {
                    return Err(CrearOperacionError::StockInsuficiente(format!(
                        "Existencia insuficiente para {product_id}: disponible {} milésimas, solicitada {quantity} milésimas",
                        stock
                    )));
                }
            }
        }

        self.repositorio.crear(operacion).await.map_err(Into::into)
    }
}
