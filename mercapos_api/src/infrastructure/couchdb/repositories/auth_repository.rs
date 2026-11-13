use async_trait::async_trait;
use serde_json::{Value, json};

use crate::domain::entities::credencial::Credencial;
use crate::domain::entities::sesion::Sesion;
use crate::domain::repositories::auth_repository::AuthRepository;
use crate::domain::repositories::repository_error::RepositoryError;
use crate::infrastructure::couchdb::{client::CouchDbClient, error::CouchDbError};

#[derive(Clone)]
pub struct CouchDbAuthRepository {
    client: CouchDbClient,
}

impl CouchDbAuthRepository {
    pub fn new(client: CouchDbClient) -> Self {
        Self { client }
    }

    fn validar_id(id: &str) -> Result<(), RepositoryError> {
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
        {
            return Err(RepositoryError::Unexpected(
                "El identificador contiene caracteres no permitidos".into(),
            ));
        }
        Ok(())
    }

    fn ruta(&self, id: &str) -> Result<String, RepositoryError> {
        Self::validar_id(id)?;
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

    fn sesion_desde_doc(doc: &Value) -> Result<Sesion, RepositoryError> {
        let texto = |campo: &str| {
            doc.get(campo)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| RepositoryError::Unexpected(format!("Sesión sin campo {campo}")))
        };
        let numero = |campo: &str| {
            doc.get(campo)
                .and_then(Value::as_i64)
                .ok_or_else(|| RepositoryError::Unexpected(format!("Sesión sin campo {campo}")))
        };
        Ok(Sesion {
            token_hash: texto("token_hash")?,
            usuario_id: texto("usuario_id")?,
            creada_en: numero("creada_en")?,
            expira_en: numero("expira_en")?,
            revocada: doc
                .get("revocada")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }
}

#[async_trait]
impl AuthRepository for CouchDbAuthRepository {
    async fn guardar_credencial(&self, credencial: Credencial) -> Result<(), RepositoryError> {
        let id = format!("credencial:{}", credencial.usuario_id);
        let ruta = self.ruta(&id)?;
        let revision = match self.client.get::<Value>(&ruta).await {
            Ok(doc) => doc.get("_rev").and_then(Value::as_str).map(str::to_owned),
            Err(CouchDbError::Server { status: 404, .. }) => None,
            Err(error) => return Err(Self::convertir_error(error)),
        };
        let mut doc = json!({
            "_id": id,
            "tipo": "credencial",
            "schema_version": 1,
            "usuario_id": credencial.usuario_id,
            "password_hash": credencial.password_hash,
            "actualizado_en": credencial.actualizado_en,
        });
        if let Some(rev) = revision {
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
                "CouchDB no confirmó la credencial".into(),
            ))
        }
    }

    async fn obtener_credencial(
        &self,
        usuario_id: &str,
    ) -> Result<Option<Credencial>, RepositoryError> {
        let ruta = self.ruta(&format!("credencial:{usuario_id}"))?;
        let doc = match self.client.get::<Value>(&ruta).await {
            Ok(doc) => doc,
            Err(CouchDbError::Server { status: 404, .. }) => return Ok(None),
            Err(error) => return Err(Self::convertir_error(error)),
        };
        if doc.get("tipo").and_then(Value::as_str) != Some("credencial") {
            return Err(RepositoryError::Unexpected(
                "El documento no es una credencial".into(),
            ));
        }
        let password_hash = doc
            .get("password_hash")
            .and_then(Value::as_str)
            .ok_or_else(|| RepositoryError::Unexpected("Credencial sin hash".into()))?
            .to_owned();
        let actualizado_en = doc
            .get("actualizado_en")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        Ok(Some(Credencial {
            usuario_id: usuario_id.to_owned(),
            password_hash,
            actualizado_en,
        }))
    }

    async fn eliminar_credencial(&self, usuario_id: &str) -> Result<(), RepositoryError> {
        let id = format!("credencial:{usuario_id}");
        let ruta = self.ruta(&id)?;
        let doc = match self.client.get::<Value>(&ruta).await {
            Ok(doc) => doc,
            Err(CouchDbError::Server { status: 404, .. }) => return Ok(()),
            Err(error) => return Err(Self::convertir_error(error)),
        };
        let rev = doc
            .get("_rev")
            .and_then(Value::as_str)
            .ok_or_else(|| RepositoryError::Unexpected("Credencial sin revisión".into()))?;
        self.client
            .delete::<Value>(&format!("{ruta}?rev={rev}"))
            .await
            .map_err(Self::convertir_error)?;
        Ok(())
    }

    async fn contar_credenciales(&self) -> Result<u64, RepositoryError> {
        let docs = self
            .client
            .find_all(json!({"tipo": "credencial"}))
            .await
            .map_err(Self::convertir_error)?;
        Ok(docs.len() as u64)
    }

    async fn crear_sesion(&self, sesion: Sesion) -> Result<(), RepositoryError> {
        let id = format!("sesion:{}", sesion.token_hash);
        let ruta = self.ruta(&id)?;
        let doc = json!({
            "_id": id,
            "tipo": "sesion",
            "schema_version": 1,
            "token_hash": sesion.token_hash,
            "usuario_id": sesion.usuario_id,
            "creada_en": sesion.creada_en,
            "expira_en": sesion.expira_en,
            "revocada": sesion.revocada,
        });
        let respuesta: Value = match self.client.put(&ruta, &doc).await {
            Ok(respuesta) => respuesta,
            Err(CouchDbError::Server { status: 409, .. }) => {
                return Err(RepositoryError::AlreadyExists);
            }
            Err(error) => return Err(Self::convertir_error(error)),
        };
        if respuesta.get("ok").and_then(Value::as_bool) == Some(true) {
            Ok(())
        } else {
            Err(RepositoryError::Unexpected(
                "CouchDB no confirmó la sesión".into(),
            ))
        }
    }

    async fn obtener_sesion(&self, token_hash: &str) -> Result<Option<Sesion>, RepositoryError> {
        let ruta = self.ruta(&format!("sesion:{token_hash}"))?;
        let doc = match self.client.get::<Value>(&ruta).await {
            Ok(doc) => doc,
            Err(CouchDbError::Server { status: 404, .. }) => return Ok(None),
            Err(error) => return Err(Self::convertir_error(error)),
        };
        if doc.get("tipo").and_then(Value::as_str) != Some("sesion") {
            return Err(RepositoryError::Unexpected(
                "El documento no es una sesión".into(),
            ));
        }
        Ok(Some(Self::sesion_desde_doc(&doc)?))
    }

    async fn revocar_sesion(&self, token_hash: &str) -> Result<(), RepositoryError> {
        let id = format!("sesion:{token_hash}");
        let ruta = self.ruta(&id)?;
        let mut doc = match self.client.get::<Value>(&ruta).await {
            Ok(doc) => doc,
            Err(CouchDbError::Server { status: 404, .. }) => return Ok(()),
            Err(error) => return Err(Self::convertir_error(error)),
        };
        doc["revocada"] = json!(true);
        self.client
            .put::<Value, Value>(&ruta, &doc)
            .await
            .map_err(Self::convertir_error)?;
        Ok(())
    }
}
