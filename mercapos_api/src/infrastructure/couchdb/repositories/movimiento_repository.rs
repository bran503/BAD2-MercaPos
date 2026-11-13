use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::domain::entities::movimiento::{DireccionMovimiento, MotivoMovimiento, Movimiento};
use crate::domain::repositories::movimiento_repository::MovimientoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

use crate::infrastructure::couchdb::client::CouchDbClient;
use crate::infrastructure::couchdb::error::CouchDbError;

#[derive(Debug, Serialize, Deserialize)]
struct MovimientoDocument {
    #[serde(rename = "_id")]
    id: String,

    #[serde(rename = "_rev", skip_serializing_if = "Option::is_none")]
    rev: Option<String>,

    tipo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_version: Option<u32>,
    producto_id: String,
    puesto_id: String,
    cantidad: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    cantidad_mil: Option<i64>,
    motivo: String,
    operacion_id: Option<String>,
    usuario_id: Option<String>,
    dispositivo_id: String,
    fecha_local: String,
}

#[derive(Debug, Deserialize)]
struct CouchDbResponse {
    ok: bool,
}

#[derive(Clone)]
pub struct CouchDbMovimientoRepository {
    client: CouchDbClient,
}

impl CouchDbMovimientoRepository {
    pub fn new(client: CouchDbClient) -> Self {
        Self { client }
    }

    fn ruta_documento(&self, id: &str) -> Result<String, RepositoryError> {
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')
        {
            return Err(RepositoryError::Unexpected(
                "El ID del movimiento contiene caracteres no permitidos".to_string(),
            ));
        }

        Ok(format!("{}/{}", self.client.database(), id))
    }

    fn convertir_error(error: CouchDbError) -> RepositoryError {
        match error {
            CouchDbError::Server { status: 404, .. } => RepositoryError::NotFound,
            CouchDbError::Server { status: 409, .. } => RepositoryError::Conflict,
            CouchDbError::Http(error) => RepositoryError::ConnectionError(error.to_string()),
            otro => RepositoryError::Unexpected(otro.to_string()),
        }
    }

    fn convertir_motivo(motivo: &MotivoMovimiento) -> &'static str {
        match motivo {
            MotivoMovimiento::InventarioInicial => "inventario_inicial",
            MotivoMovimiento::Venta => "venta",
            MotivoMovimiento::Compra => "compra",
            MotivoMovimiento::Merma => "merma",
            MotivoMovimiento::Devolucion => "devolucion",
            MotivoMovimiento::Ajuste => "ajuste",
        }
    }

    fn documento_a_entidad(documento: MovimientoDocument) -> Result<Movimiento, RepositoryError> {
        let motivo = match documento.motivo.as_str() {
            "inventario_inicial" => MotivoMovimiento::InventarioInicial,
            "venta" => MotivoMovimiento::Venta,
            "compra" => MotivoMovimiento::Compra,
            "merma" => MotivoMovimiento::Merma,
            "devolucion" => MotivoMovimiento::Devolucion,
            "ajuste" => MotivoMovimiento::Ajuste,
            _ => {
                return Err(RepositoryError::Unexpected(
                    "Motivo de movimiento desconocido en CouchDB".to_string(),
                ));
            }
        };

        let signed = documento
            .cantidad_mil
            .map(|mil| mil as f64 / 1000.0)
            .unwrap_or(documento.cantidad);
        let direccion = if signed.is_sign_negative() {
            DireccionMovimiento::Salida
        } else {
            DireccionMovimiento::Entrada
        };

        if documento.tipo != "movimiento" && documento.tipo != "apertura_inventario" {
            return Err(RepositoryError::Unexpected(
                "El documento no es un movimiento".to_string(),
            ));
        }
        if documento.tipo == "apertura_inventario" && documento.schema_version != Some(2) {
            return Err(RepositoryError::Unexpected(
                "Versión de apertura incompatible".into(),
            ));
        }
        if !documento.id.starts_with("mov:") {
            return Err(RepositoryError::Unexpected(
                "ID de movimiento incompatible".into(),
            ));
        }

        Ok(Movimiento {
            id: documento.id,
            producto_id: documento.producto_id,
            puesto_id: documento.puesto_id,
            cantidad: signed.abs(),
            motivo,
            operacion_id: documento.operacion_id,
            usuario_id: documento.usuario_id,
            dispositivo_id: documento.dispositivo_id,
            fecha_local: documento.fecha_local,
            direccion,
        })
    }

    async fn buscar_documentos(&self, selector: Value) -> Result<Vec<Value>, RepositoryError> {
        self.client
            .find_all(selector)
            .await
            .map_err(Self::convertir_error)
    }

    fn proyectar_operacion(doc: Value) -> Result<Vec<Movimiento>, RepositoryError> {
        let field = |key: &str| -> Result<String, RepositoryError> {
            doc.get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| RepositoryError::Unexpected(format!("Operación canónica sin {key}")))
        };
        if doc.get("tipo").and_then(Value::as_str) != Some("operacion")
            || doc.get("schema_version").and_then(Value::as_u64) != Some(2)
        {
            return Err(RepositoryError::Unexpected(
                "Documento canónico incompatible".into(),
            ));
        }
        let id = field("_id")?;
        let puesto_id = field("puesto_id")?;
        let dispositivo_id = field("dispositivo_id")?;
        let fecha_local = field("fecha_local")?;
        let usuario_id = field("usuario_id")?;
        let motivo = match field("tipo_operacion")?.as_str() {
            "venta" => MotivoMovimiento::Venta,
            "compra" => MotivoMovimiento::Compra,
            "merma" => MotivoMovimiento::Merma,
            "devolucion" => MotivoMovimiento::Devolucion,
            "ajuste" => MotivoMovimiento::Ajuste,
            _ => {
                return Err(RepositoryError::Unexpected(
                    "Tipo de operación canónica inválido".into(),
                ));
            }
        };
        let effects = doc
            .get("impactos_inventario")
            .and_then(Value::as_array)
            .ok_or_else(|| RepositoryError::Unexpected("Operación canónica sin impactos".into()))?;
        effects
            .iter()
            .map(|effect| {
                let product = effect
                    .get("producto_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| RepositoryError::Unexpected("Impacto sin producto_id".into()))?;
                let line = effect
                    .get("renglon_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| RepositoryError::Unexpected("Impacto sin renglon_id".into()))?;
                let signed = if let Some(mil) = effect.get("cantidad_mil").and_then(Value::as_i64) {
                    mil as f64 / 1000.0
                } else {
                    effect
                        .get("cantidad")
                        .and_then(Value::as_f64)
                        .ok_or_else(|| RepositoryError::Unexpected("Impacto sin cantidad".into()))?
                };
                if !signed.is_finite() || signed == 0.0 {
                    return Err(RepositoryError::Unexpected(
                        "Cantidad canónica inválida".into(),
                    ));
                }
                Ok(Movimiento {
                    id: format!(
                        "mov:{}:{line}:{}",
                        id.trim_start_matches("operacion:"),
                        product.trim_start_matches("producto:")
                    ),
                    producto_id: product.to_owned(),
                    puesto_id: puesto_id.clone(),
                    cantidad: signed.abs(),
                    motivo: motivo.clone(),
                    operacion_id: Some(id.clone()),
                    usuario_id: Some(usuario_id.clone()),
                    dispositivo_id: dispositivo_id.clone(),
                    fecha_local: fecha_local.clone(),
                    direccion: if signed < 0.0 {
                        DireccionMovimiento::Salida
                    } else {
                        DireccionMovimiento::Entrada
                    },
                })
            })
            .collect()
    }

    async fn listar_proyectados(
        &self,
        puesto_id: Option<&str>,
        producto_id: Option<&str>,
    ) -> Result<Vec<Movimiento>, RepositoryError> {
        let mut legacy_selector = json!({"tipo":"movimiento"});
        if let Some(id) = puesto_id {
            legacy_selector["puesto_id"] = json!(id);
        }
        if let Some(id) = producto_id {
            legacy_selector["producto_id"] = json!(id);
        }
        let mut result = self
            .buscar_documentos(legacy_selector)
            .await?
            .into_iter()
            .map(|doc| {
                serde_json::from_value::<MovimientoDocument>(doc)
                    .map_err(|e| RepositoryError::Unexpected(e.to_string()))
                    .and_then(Self::documento_a_entidad)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut opening_selector = json!({"tipo":"apertura_inventario","schema_version":2});
        if let Some(id) = puesto_id {
            opening_selector["puesto_id"] = json!(id);
        }
        if let Some(id) = producto_id {
            opening_selector["producto_id"] = json!(id);
        }
        result.extend(
            self.buscar_documentos(opening_selector)
                .await?
                .into_iter()
                .map(|doc| {
                    serde_json::from_value::<MovimientoDocument>(doc)
                        .map_err(|e| RepositoryError::Unexpected(e.to_string()))
                        .and_then(Self::documento_a_entidad)
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        let mut operation_selector = json!({"tipo":"operacion","schema_version":2});
        if let Some(id) = puesto_id {
            operation_selector["puesto_id"] = json!(id);
        }
        for doc in self.buscar_documentos(operation_selector).await? {
            result.extend(
                Self::proyectar_operacion(doc)?
                    .into_iter()
                    .filter(|m| producto_id.is_none_or(|id| m.producto_id == id)),
            );
        }
        Ok(result)
    }
}

impl From<&Movimiento> for MovimientoDocument {
    fn from(movimiento: &Movimiento) -> Self {
        Self {
            id: movimiento.id.clone(),
            rev: None,
            tipo: if movimiento.motivo == MotivoMovimiento::InventarioInicial {
                "apertura_inventario".to_string()
            } else {
                "movimiento".to_string()
            },
            schema_version: if movimiento.motivo == MotivoMovimiento::InventarioInicial {
                Some(2)
            } else {
                None
            },
            producto_id: movimiento.producto_id.clone(),
            puesto_id: movimiento.puesto_id.clone(),
            cantidad: if matches!(movimiento.direccion, DireccionMovimiento::Salida) {
                -movimiento.cantidad.abs()
            } else {
                movimiento.cantidad.abs()
            },
            cantidad_mil: None,
            motivo: CouchDbMovimientoRepository::convertir_motivo(&movimiento.motivo).to_string(),
            operacion_id: movimiento.operacion_id.clone(),
            usuario_id: movimiento.usuario_id.clone(),
            dispositivo_id: movimiento.dispositivo_id.clone(),
            fecha_local: movimiento.fecha_local.clone(),
        }
    }
}

#[async_trait]
impl MovimientoRepository for CouchDbMovimientoRepository {
    async fn crear(&self, movimiento: Movimiento) -> Result<(), RepositoryError> {
        let ruta = self.ruta_documento(&movimiento.id)?;

        let mut documento = MovimientoDocument::from(&movimiento);
        if movimiento.motivo == MotivoMovimiento::InventarioInicial {
            documento.cantidad_mil = Some(
                crate::domain::money::parse_scaled(
                    &movimiento.cantidad.to_string(),
                    3,
                    1_000_000_000,
                )
                .map_err(RepositoryError::Unexpected)?,
            );
        }

        let respuesta: CouchDbResponse = match self.client.put(&ruta, &documento).await {
            Ok(respuesta) => respuesta,

            Err(CouchDbError::Server { status: 409, .. }) => {
                return Err(RepositoryError::AlreadyExists);
            }

            Err(error) => return Err(Self::convertir_error(error)),
        };

        if !respuesta.ok {
            return Err(RepositoryError::Unexpected(
                "CouchDB no confirmó la creación del movimiento".to_string(),
            ));
        }

        Ok(())
    }

    async fn listar_por_producto(
        &self,
        producto_id: &str,
    ) -> Result<Vec<Movimiento>, RepositoryError> {
        self.listar_proyectados(None, Some(producto_id)).await
    }

    async fn listar_por_puesto(&self, puesto_id: &str) -> Result<Vec<Movimiento>, RepositoryError> {
        self.listar_proyectados(Some(puesto_id), None).await
    }
}
