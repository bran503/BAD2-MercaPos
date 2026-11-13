use async_trait::async_trait;
use serde_json::{Value, json};

use crate::domain::entities::incidencia::{
    IncidenciaSincronizacion, TipoIncidencia, id_de_incidencia,
};
use crate::domain::repositories::incidencia_repository::IncidenciaRepository;
use crate::domain::repositories::repository_error::RepositoryError;
use crate::infrastructure::couchdb::{client::CouchDbClient, error::CouchDbError};

#[derive(Clone)]
pub struct CouchDbIncidenciaRepository {
    client: CouchDbClient,
}

impl CouchDbIncidenciaRepository {
    pub fn new(client: CouchDbClient) -> Self {
        Self { client }
    }

    fn ruta(&self, id: &str) -> Result<String, RepositoryError> {
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
        {
            return Err(RepositoryError::Unexpected(
                "El identificador de la incidencia no es válido".into(),
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

    fn doc(incidencia: &IncidenciaSincronizacion) -> Value {
        json!({
            "_id": incidencia.id,
            "tipo": "incidencia",
            "schema_version": 1,
            "operacion_id": incidencia.operacion_id,
            "puesto_id": incidencia.puesto_id,
            "usuario_id": incidencia.usuario_id,
            "tipo_incidencia": incidencia.tipo.as_str(),
            "codigo": incidencia.codigo,
            "motivo": incidencia.motivo,
            "intentos": incidencia.intentos,
            "creada_en": incidencia.creada_en,
            "actualizada_en": incidencia.actualizada_en,
            "resuelta": incidencia.resuelta,
            "resuelta_por": incidencia.resuelta_por,
            "resuelta_en": incidencia.resuelta_en,
            "nota": incidencia.nota,
        })
    }

    fn desde_doc(doc: &Value) -> Result<IncidenciaSincronizacion, RepositoryError> {
        let texto = |campo: &str| {
            doc.get(campo)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| RepositoryError::Unexpected(format!("Incidencia sin campo {campo}")))
        };
        let numero = |campo: &str| {
            doc.get(campo)
                .and_then(Value::as_i64)
                .ok_or_else(|| RepositoryError::Unexpected(format!("Incidencia sin campo {campo}")))
        };
        let tipo = TipoIncidencia::desde_str(&texto("tipo_incidencia")?)
            .ok_or_else(|| RepositoryError::Unexpected("Tipo de incidencia desconocido".into()))?;
        Ok(IncidenciaSincronizacion {
            id: texto("_id")?,
            operacion_id: texto("operacion_id")?,
            puesto_id: texto("puesto_id")?,
            usuario_id: texto("usuario_id")?,
            tipo,
            codigo: doc.get("codigo").and_then(Value::as_u64).unwrap_or(0) as u16,
            motivo: texto("motivo")?,
            intentos: doc.get("intentos").and_then(Value::as_u64).unwrap_or(1) as u32,
            creada_en: numero("creada_en")?,
            actualizada_en: numero("actualizada_en")?,
            resuelta: doc
                .get("resuelta")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            resuelta_por: doc
                .get("resuelta_por")
                .and_then(Value::as_str)
                .map(str::to_owned),
            resuelta_en: doc.get("resuelta_en").and_then(Value::as_i64),
            nota: doc.get("nota").and_then(Value::as_str).map(str::to_owned),
            revision: doc.get("_rev").and_then(Value::as_str).map(str::to_owned),
        })
    }
}

#[async_trait]
impl IncidenciaRepository for CouchDbIncidenciaRepository {
    async fn guardar(
        &self,
        mut incidencia: IncidenciaSincronizacion,
    ) -> Result<(), RepositoryError> {
        let ruta = self.ruta(&incidencia.id)?;
        if let Ok(existente) = self.client.get::<Value>(&ruta).await {
            incidencia.revision = existente
                .get("_rev")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if let Some(creada_en) = existente.get("creada_en").and_then(Value::as_i64) {
                incidencia.creada_en = creada_en;
            }
            let intentos = existente
                .get("intentos")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            incidencia.intentos = (intentos as u32).saturating_add(1);
        }
        let mut doc = Self::doc(&incidencia);
        if let Some(rev) = &incidencia.revision {
            doc["_rev"] = json!(rev);
        }
        let respuesta: Value = self
            .client
            .put(&ruta, &doc)
            .await
            .map_err(Self::convertir_error)?;
        if respuesta.get("ok").and_then(Value::as_bool) == Some(true) {
            Ok(())
        } else {
            Err(RepositoryError::Unexpected(
                "CouchDB no confirmó la incidencia".into(),
            ))
        }
    }

    async fn obtener_por_operacion(
        &self,
        operacion_id: &str,
    ) -> Result<Option<IncidenciaSincronizacion>, RepositoryError> {
        let id = id_de_incidencia(operacion_id);
        let ruta = self.ruta(&id)?;
        let doc = match self.client.get::<Value>(&ruta).await {
            Ok(doc) => doc,
            Err(CouchDbError::Server { status: 404, .. }) => return Ok(None),
            Err(error) => return Err(Self::convertir_error(error)),
        };
        if doc.get("tipo").and_then(Value::as_str) != Some("incidencia")
            || doc.get("operacion_id").and_then(Value::as_str) != Some(operacion_id)
        {
            return Ok(None);
        }
        Ok(Some(Self::desde_doc(&doc)?))
    }

    async fn listar(
        &self,
        puesto_id: Option<&str>,
        resuelta: Option<bool>,
    ) -> Result<Vec<IncidenciaSincronizacion>, RepositoryError> {
        let mut selector = json!({"tipo": "incidencia"});
        if let Some(puesto) = puesto_id {
            selector["puesto_id"] = json!(puesto);
        }
        if let Some(estado) = resuelta {
            selector["resuelta"] = json!(estado);
        }
        self.client
            .find_all(selector)
            .await
            .map_err(Self::convertir_error)?
            .iter()
            .map(Self::desde_doc)
            .collect()
    }

    async fn resolver(
        &self,
        operacion_id: &str,
        resuelta_por: &str,
        nota: Option<String>,
        ahora: i64,
    ) -> Result<Option<IncidenciaSincronizacion>, RepositoryError> {
        let id = id_de_incidencia(operacion_id);
        let ruta = self.ruta(&id)?;
        let mut doc = match self.client.get::<Value>(&ruta).await {
            Ok(doc) => doc,
            Err(CouchDbError::Server { status: 404, .. }) => return Ok(None),
            Err(error) => return Err(Self::convertir_error(error)),
        };
        if doc.get("operacion_id").and_then(Value::as_str) != Some(operacion_id) {
            return Ok(None);
        }
        doc["resuelta"] = json!(true);
        doc["resuelta_por"] = json!(resuelta_por);
        doc["resuelta_en"] = json!(ahora);
        doc["actualizada_en"] = json!(ahora);
        if let Some(nota) = nota {
            doc["nota"] = json!(nota);
        }
        let respuesta: Value = self
            .client
            .put(&ruta, &doc)
            .await
            .map_err(Self::convertir_error)?;
        if respuesta.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(RepositoryError::Unexpected(
                "CouchDB no confirmó la resolución".into(),
            ));
        }
        Ok(Some(Self::desde_doc(&doc)?))
    }
}
