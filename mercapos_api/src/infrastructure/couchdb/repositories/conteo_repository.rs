use crate::domain::entities::conteo_fisico::{ConteoFisico, RenglonConteo};
use crate::domain::repositories::{
    conteo_repository::ConteoRepository, repository_error::RepositoryError,
};
use crate::infrastructure::couchdb::{client::CouchDbClient, error::CouchDbError};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone)]
pub struct CouchDbConteoRepository {
    client: CouchDbClient,
}
#[derive(Deserialize)]
struct WriteResponse {
    ok: bool,
}
impl CouchDbConteoRepository {
    pub fn new(client: CouchDbClient) -> Self {
        Self { client }
    }
    fn path(&self, id: &str) -> Result<String, RepositoryError> {
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
        {
            return Err(RepositoryError::Unexpected("ID de conteo inválido".into()));
        }
        Ok(format!("{}/{}", self.client.database(), id))
    }
    fn err(e: CouchDbError) -> RepositoryError {
        match e {
            CouchDbError::Server { status: 409, .. } => RepositoryError::AlreadyExists,
            CouchDbError::Server { status: 404, .. } => RepositoryError::NotFound,
            CouchDbError::Http(e) => RepositoryError::ConnectionError(e.to_string()),
            e => RepositoryError::Unexpected(e.to_string()),
        }
    }
    fn from_doc(v: Value) -> Result<ConteoFisico, RepositoryError> {
        if v.get("tipo").and_then(Value::as_str) != Some("conteo_fisico")
            || !v
                .get("_id")
                .and_then(Value::as_str)
                .is_some_and(|id| id.starts_with("conteo:"))
        {
            return Err(RepositoryError::Unexpected(
                "Documento de conteo incompatible".into(),
            ));
        }
        let s = |k: &str| {
            v.get(k)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| RepositoryError::Unexpected(format!("Conteo sin campo {k}")))
        };
        let rows = v
            .get("renglones")
            .and_then(Value::as_array)
            .ok_or_else(|| RepositoryError::Unexpected("Conteo sin renglones".into()))?
            .iter()
            .map(|r| {
                Ok(RenglonConteo {
                    producto_id: r["producto_id"]
                        .as_str()
                        .ok_or_else(|| RepositoryError::Unexpected("Renglón sin producto".into()))?
                        .to_owned(),
                    cantidad_sistema: r["cantidad_sistema"].as_f64().ok_or_else(|| {
                        RepositoryError::Unexpected("cantidad_sistema inválida".into())
                    })?,
                    cantidad_contada: r["cantidad_contada"].as_f64().ok_or_else(|| {
                        RepositoryError::Unexpected("cantidad_contada inválida".into())
                    })?,
                    diferencia: r["diferencia"]
                        .as_f64()
                        .ok_or_else(|| RepositoryError::Unexpected("diferencia inválida".into()))?,
                })
            })
            .collect::<Result<Vec<_>, RepositoryError>>()?;
        Ok(ConteoFisico {
            id: s("_id")?,
            puesto_id: s("puesto_id")?,
            usuario_id: s("usuario_id")?,
            dispositivo_id: s("dispositivo_id")?,
            fecha_local: s("fecha_local")?,
            periodo_estudio: v
                .get("periodo_estudio")
                .and_then(Value::as_str)
                .map(str::to_owned),
            etapa: v.get("etapa").and_then(Value::as_str).map(str::to_owned),
            origen_registro: v
                .get("origen_registro")
                .and_then(Value::as_str)
                .map(str::to_owned),
            evidencia: v
                .get("evidencia")
                .and_then(Value::as_str)
                .map(str::to_owned),
            corte_seq: v
                .get("corte_seq")
                .and_then(Value::as_str)
                .map(str::to_owned),
            renglones: rows,
        })
    }
}
#[async_trait]
impl ConteoRepository for CouchDbConteoRepository {
    async fn crear(&self, c: ConteoFisico) -> Result<(), RepositoryError> {
        if !c.id.starts_with("conteo:") {
            return Err(RepositoryError::Conflict);
        }
        let mut body = serde_json::Map::new();
        body.insert("_id".into(), json!(c.id));
        body.insert("tipo".into(), json!("conteo_fisico"));
        body.insert("schema_version".into(), json!(2));
        body.insert("puesto_id".into(), json!(c.puesto_id));
        body.insert("usuario_id".into(), json!(c.usuario_id));
        body.insert("dispositivo_id".into(), json!(c.dispositivo_id));
        body.insert("fecha_local".into(), json!(c.fecha_local));
        body.insert("periodo_estudio".into(), json!(c.periodo_estudio));
        body.insert("etapa".into(), json!(c.etapa));
        body.insert("origen_registro".into(), json!(c.origen_registro));
        if let Some(evidencia) = &c.evidencia {
            body.insert("evidencia".into(), json!(evidencia));
        }
        if let Some(corte_seq) = &c.corte_seq {
            body.insert("corte_seq".into(), json!(corte_seq));
        }
        body.insert("renglones".into(), Value::Array(c.renglones.iter().map(|r| json!({"producto_id":r.producto_id,"cantidad_sistema":r.cantidad_sistema,"cantidad_contada":r.cantidad_contada,"diferencia":r.diferencia})).collect::<Vec<_>>()));
        let body = Value::Object(body);
        let p = self.path(&c.id)?;
        let w: WriteResponse = self.client.put(&p, &body).await.map_err(Self::err)?;
        if w.ok {
            Ok(())
        } else {
            Err(RepositoryError::Unexpected(
                "CouchDB no confirmó el conteo".into(),
            ))
        }
    }
    async fn obtener_por_id(&self, id: &str) -> Result<Option<ConteoFisico>, RepositoryError> {
        if !id.starts_with("conteo:") {
            return Err(RepositoryError::Conflict);
        }
        let p = self.path(id)?;
        match self.client.get::<Value>(&p).await {
            Ok(v) => Ok(Some(Self::from_doc(v)?)),
            Err(CouchDbError::Server { status: 404, .. }) => Ok(None),
            Err(e) => Err(Self::err(e)),
        }
    }
    async fn listar_por_puesto(&self, p: &str) -> Result<Vec<ConteoFisico>, RepositoryError> {
        self.client
            .find_all(json!({"tipo":"conteo_fisico","puesto_id":p}))
            .await
            .map_err(Self::err)?
            .into_iter()
            .map(Self::from_doc)
            .collect()
    }
}
