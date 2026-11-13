use async_trait::async_trait;
use serde_json::{Value, json};

use crate::domain::entities::operacion::{
    Operacion, Pago, RenglonOperacion, TipoDevolucion, TipoOperacion,
};
use crate::domain::repositories::{
    operacion_repository::OperacionRepository, repository_error::RepositoryError,
};
use crate::infrastructure::couchdb::{client::CouchDbClient, error::CouchDbError};

#[derive(Clone)]
pub struct CouchDbOperacionRepository {
    client: CouchDbClient,
}

impl CouchDbOperacionRepository {
    pub fn new(client: CouchDbClient) -> Self {
        Self { client }
    }

    fn path(&self, id: &str) -> Result<String, RepositoryError> {
        if !id.starts_with("operacion:")
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
        {
            return Err(RepositoryError::Unexpected(
                "El ID de la operación contiene caracteres no permitidos".into(),
            ));
        }
        Ok(format!("{}/{}", self.client.database(), id))
    }

    fn map_error(e: CouchDbError) -> RepositoryError {
        match e {
            CouchDbError::Server { status: 404, .. } => RepositoryError::NotFound,
            CouchDbError::Server { status: 409, .. } => RepositoryError::Conflict,
            CouchDbError::Http(e) => RepositoryError::ConnectionError(e.to_string()),
            e => RepositoryError::Unexpected(e.to_string()),
        }
    }

    fn to_doc(op: &Operacion) -> Result<Value, RepositoryError> {
        let impactos = op
            .renglones
            .iter()
            .enumerate()
            .map(|(index, r)| {
                if op.destino_devolucion.as_deref() == Some("no_vendible") {
                    return Ok(None);
                }
                let magnitude = crate::domain::money::parse_scaled(
                    &r.cantidad.abs().to_string(),
                    3,
                    1_000_000_000,
                )
                .map_err(RepositoryError::Unexpected)?;
                let negative = matches!(
                    op.tipo_operacion,
                    TipoOperacion::Venta | TipoOperacion::Merma
                ) || op.tipo_devolucion == Some(TipoDevolucion::Proveedor)
                    || (matches!(op.tipo_operacion, TipoOperacion::Ajuste) && r.cantidad < 0.0);
                let signed_mil = if negative { -magnitude } else { magnitude };
                Ok(Some(
                    json!({"renglon_id":format!("r{}", index + 1),"producto_id":r.producto_id,
                "cantidad":signed_mil as f64 / 1000.0,"cantidad_mil":signed_mil}),
                ))
            })
            .collect::<Result<Vec<_>, RepositoryError>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let renglones = op
            .renglones
            .iter()
            .map(|r| {
                let mut linea = serde_json::Map::new();
                linea.insert("producto_id".into(), json!(r.producto_id));
                linea.insert("cantidad".into(), json!(r.cantidad));
                if let Some(nombre) = &r.nombre_producto {
                    linea.insert("nombre_producto".into(), json!(nombre));
                }
                if let Some(unidad) = &r.unidad {
                    linea.insert("unidad".into(), json!(unidad));
                }
                if let Some(precio) = r.precio_unitario {
                    linea.insert("precio_unitario".into(), json!(precio));
                }
                if let Some(costo) = r.costo_unitario {
                    linea.insert("costo_unitario".into(), json!(costo));
                }
                Value::Object(linea)
            })
            .collect::<Vec<_>>();
        let mut doc = serde_json::Map::new();
        doc.insert("_id".into(), json!(op.id));
        doc.insert("schema_version".into(), json!(2));
        doc.insert("tipo".into(), json!("operacion"));
        doc.insert(
            "tipo_operacion".into(),
            json!(Self::tipo(&op.tipo_operacion)),
        );
        doc.insert("puesto_id".into(), json!(op.puesto_id));
        doc.insert("usuario_id".into(), json!(op.usuario_id));
        doc.insert("dispositivo_id".into(), json!(op.dispositivo_id));
        doc.insert("fecha_local".into(), json!(op.fecha_local));
        doc.insert(
            "recibido_en_epoch_ms".into(),
            json!(op.recibido_en_epoch_ms),
        );
        doc.insert("renglones".into(), Value::Array(renglones));
        doc.insert("impactos_inventario".into(), Value::Array(impactos));
        doc.insert("moneda".into(), json!("USD"));
        if let Some(total) = op.total {
            doc.insert("total".into(), json!(total));
        }
        if let Some(total_centavos) = op.total_centavos {
            doc.insert("total_centavos".into(), json!(total_centavos));
        }
        if let Some(motivo) = &op.motivo {
            doc.insert("motivo".into(), json!(motivo));
        }
        if let Some(origen) = &op.operacion_origen_id {
            doc.insert("operacion_origen_id".into(), json!(origen));
        }
        if let Some(conteo) = &op.conteo_id {
            doc.insert("conteo_id".into(), json!(conteo));
        }
        if let Some(tipo) = &op.tipo_devolucion {
            doc.insert(
                "tipo_devolucion".into(),
                json!(match tipo {
                    TipoDevolucion::Cliente => "cliente",
                    TipoDevolucion::Proveedor => "proveedor",
                }),
            );
        }
        if let Some(destino) = &op.destino_devolucion {
            doc.insert("destino_devolucion".into(), json!(destino));
        }
        if let Some(pago) = &op.pago {
            doc.insert(
                "pago".into(),
                json!({
                    "metodo": pago.metodo,
                    "modo": pago.modo,
                    "estado": pago.estado,
                    "autorizacion": pago.autorizacion,
                    "fecha": pago.fecha,
                }),
            );
        }
        Ok(Value::Object(doc))
    }
    fn tipo(t: &TipoOperacion) -> &'static str {
        match t {
            TipoOperacion::Venta => "venta",
            TipoOperacion::Compra => "compra",
            TipoOperacion::Merma => "merma",
            TipoOperacion::Devolucion => "devolucion",
            TipoOperacion::Ajuste => "ajuste",
        }
    }
    fn from_doc(v: Value) -> Result<Operacion, RepositoryError> {
        if v.get("tipo").and_then(Value::as_str) != Some("operacion") {
            return Err(RepositoryError::Unexpected(
                "El documento no es una operación".into(),
            ));
        }
        if !v
            .get("_id")
            .and_then(Value::as_str)
            .is_some_and(|id| id.starts_with("operacion:"))
        {
            return Err(RepositoryError::Unexpected(
                "ID de operación incompatible".into(),
            ));
        }
        let strf = |k: &str| {
            v.get(k)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| {
                    RepositoryError::Unexpected(format!("Documento operación sin campo {k}"))
                })
        };
        let tipo_operacion = match strf("tipo_operacion")?.as_str() {
            "venta" => TipoOperacion::Venta,
            "compra" => TipoOperacion::Compra,
            "merma" => TipoOperacion::Merma,
            "devolucion" => TipoOperacion::Devolucion,
            "ajuste" => TipoOperacion::Ajuste,
            _ => {
                return Err(RepositoryError::Unexpected(
                    "Tipo de operación desconocido en CouchDB".into(),
                ));
            }
        };
        let tipo_devolucion = match v.get("tipo_devolucion").and_then(Value::as_str) {
            Some("cliente") => Some(TipoDevolucion::Cliente),
            Some("proveedor") => Some(TipoDevolucion::Proveedor),
            None => None,
            _ => {
                return Err(RepositoryError::Unexpected(
                    "Tipo de devolución desconocido en CouchDB".into(),
                ));
            }
        };
        let renglones = v
            .get("renglones")
            .and_then(Value::as_array)
            .ok_or_else(|| RepositoryError::Unexpected("Documento operación sin renglones".into()))?
            .iter()
            .map(|r| {
                Ok(RenglonOperacion {
                    producto_id: r
                        .get("producto_id")
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            RepositoryError::Unexpected("Renglón sin producto_id".into())
                        })?
                        .to_owned(),
                    cantidad: r.get("cantidad").and_then(Value::as_f64).ok_or_else(|| {
                        RepositoryError::Unexpected("Renglón sin cantidad válida".into())
                    })?,
                    precio_unitario: r.get("precio_unitario").and_then(Value::as_f64),
                    costo_unitario: r.get("costo_unitario").and_then(Value::as_f64),
                    nombre_producto: r
                        .get("nombre_producto")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    unidad: r.get("unidad").and_then(Value::as_str).map(str::to_owned),
                })
            })
            .collect::<Result<Vec<_>, RepositoryError>>()?;
        Ok(Operacion {
            id: strf("_id")?,
            tipo_operacion,
            puesto_id: strf("puesto_id")?,
            usuario_id: strf("usuario_id")?,
            dispositivo_id: strf("dispositivo_id")?,
            fecha_local: strf("fecha_local")?,
            recibido_en_epoch_ms: v.get("recibido_en_epoch_ms").and_then(Value::as_u64),
            renglones,
            total: v.get("total").and_then(Value::as_f64),
            total_centavos: match v.get("total_centavos").and_then(Value::as_i64) {
                Some(cents) => Some(cents),
                None => v.get("total").and_then(Value::as_number).and_then(|n| {
                    crate::domain::money::parse_scaled(&n.to_string(), 2, 10_000_000_000).ok()
                }),
            },
            motivo: v.get("motivo").and_then(Value::as_str).map(str::to_owned),
            operacion_origen_id: v
                .get("operacion_origen_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            conteo_id: v
                .get("conteo_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            tipo_devolucion,
            destino_devolucion: v
                .get("destino_devolucion")
                .and_then(Value::as_str)
                .map(str::to_owned),
            pago: v.get("pago").and_then(Value::as_object).and_then(|p| {
                Some(Pago {
                    metodo: p.get("metodo")?.as_str()?.to_owned(),
                    modo: p.get("modo")?.as_str()?.to_owned(),
                    estado: p.get("estado")?.as_str()?.to_owned(),
                    autorizacion: p.get("autorizacion")?.as_str()?.to_owned(),
                    fecha: p.get("fecha")?.as_str()?.to_owned(),
                })
            }),
        })
    }
}

#[derive(serde::Deserialize)]
struct WriteResponse {
    ok: bool,
}

#[async_trait]
impl OperacionRepository for CouchDbOperacionRepository {
    async fn crear(&self, op: Operacion) -> Result<(), RepositoryError> {
        let path = self.path(&op.id)?;
        let doc = Self::to_doc(&op)?;
        let response: WriteResponse = match self.client.put(&path, &doc).await {
            Ok(v) => v,
            Err(CouchDbError::Server { status: 409, .. }) => {
                return Err(RepositoryError::AlreadyExists);
            }
            Err(e) => return Err(Self::map_error(e)),
        };
        if response.ok {
            Ok(())
        } else {
            Err(RepositoryError::Unexpected(
                "CouchDB no confirmó la operación".into(),
            ))
        }
    }
    async fn obtener_por_id(&self, id: &str) -> Result<Option<Operacion>, RepositoryError> {
        let path = self.path(id)?;
        match self.client.get::<Value>(&path).await {
            Ok(v) => Ok(Some(Self::from_doc(v)?)),
            Err(CouchDbError::Server { status: 404, .. }) => Ok(None),
            Err(e) => Err(Self::map_error(e)),
        }
    }
    async fn listar_por_puesto(&self, puesto_id: &str) -> Result<Vec<Operacion>, RepositoryError> {
        self.client
            .find_all(json!({"tipo":"operacion","puesto_id":puesto_id}))
            .await
            .map_err(Self::map_error)?
            .into_iter()
            .map(Self::from_doc)
            .collect()
    }
}
