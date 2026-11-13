use crate::{
    domain::{
        entities::{
            puesto::Puesto,
            usuario::{Rol, Usuario},
        },
        repositories::{
            puesto_repository::PuestoRepository, repository_error::RepositoryError,
            usuario_repository::UsuarioRepository,
        },
    },
    infrastructure::couchdb::{client::CouchDbClient, error::CouchDbError},
};
use async_trait::async_trait;
use serde_json::{Value, json};
#[derive(Clone)]
pub struct CouchDbCatalogoRepository {
    client: CouchDbClient,
}
#[derive(serde::Deserialize)]
struct Write {
    ok: bool,
}
impl CouchDbCatalogoRepository {
    pub fn new(client: CouchDbClient) -> Self {
        Self { client }
    }
    fn path(&self, id: &str) -> Result<String, RepositoryError> {
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
        {
            return Err(RepositoryError::Unexpected(
                "ID contiene caracteres no permitidos".into(),
            ));
        }
        Ok(format!("{}/{}", self.client.database(), id))
    }
    fn require_kind(id: &str, kind: &str) -> Result<(), RepositoryError> {
        if !id.starts_with(&format!("{kind}:")) {
            return Err(RepositoryError::Conflict);
        }
        Ok(())
    }
    fn err(e: CouchDbError) -> RepositoryError {
        match e {
            CouchDbError::Server { status: 409, .. } => RepositoryError::Conflict,
            CouchDbError::Server { status: 404, .. } => RepositoryError::NotFound,
            CouchDbError::Http(e) => RepositoryError::ConnectionError(e.to_string()),
            e => RepositoryError::Unexpected(e.to_string()),
        }
    }
    async fn list(
        &self,
        kind: &str,
        field: &str,
        value: &str,
    ) -> Result<Vec<Value>, RepositoryError> {
        let selector = if field.is_empty() {
            json!({"tipo":kind})
        } else {
            json!({"tipo":kind,(field):value})
        };
        self.client.find_all(selector).await.map_err(Self::err)
    }
    fn required(v: &Value, field: &str) -> Result<String, RepositoryError> {
        v.get(field)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| RepositoryError::Unexpected(format!("Documento sin {field}")))
    }
    fn puesto(v: &Value) -> Result<Puesto, RepositoryError> {
        let id = Self::required(v, "_id")?;
        Self::require_kind(&id, "puesto")?;
        if v.get("tipo").and_then(Value::as_str) != Some("puesto") {
            return Err(RepositoryError::Conflict);
        }
        Ok(Puesto {
            id,
            nombre: Self::required(v, "nombre")?,
            mercado: Self::required(v, "mercado")?,
            ubicacion: Self::required(v, "ubicacion")?,
            propietario_usuario_id: Self::required(v, "propietario_usuario_id")?,
            activo: v
                .get("activo")
                .and_then(Value::as_bool)
                .ok_or_else(|| RepositoryError::Unexpected("Puesto sin activo".into()))?,
            revision: Some(Self::required(v, "_rev")?),
        })
    }
    fn usuario(v: &Value) -> Result<Usuario, RepositoryError> {
        let id = Self::required(v, "_id")?;
        Self::require_kind(&id, "usuario")?;
        if v.get("tipo").and_then(Value::as_str) != Some("usuario") {
            return Err(RepositoryError::Conflict);
        }
        let rol = match v.get("rol").and_then(Value::as_str) {
            Some("vendedor") => Rol::Vendedor,
            Some("administrador") => Rol::Administrador,
            _ => {
                return Err(RepositoryError::Unexpected(
                    "Rol de usuario desconocido".into(),
                ));
            }
        };
        Ok(Usuario {
            id,
            nombre: Self::required(v, "nombre")?,
            rol,
            puesto_id: Self::required(v, "puesto_id")?,
            activo: v
                .get("activo")
                .and_then(Value::as_bool)
                .ok_or_else(|| RepositoryError::Unexpected("Usuario sin activo".into()))?,
            revision: Some(Self::required(v, "_rev")?),
        })
    }
}
#[async_trait]
impl PuestoRepository for CouchDbCatalogoRepository {
    async fn crear(&self, p: Puesto) -> Result<(), RepositoryError> {
        Self::require_kind(&p.id, "puesto")?;
        let path = self.path(&p.id)?;
        let x:Write=match self.client.put(&path,&json!({"_id":p.id,"tipo":"puesto","nombre":p.nombre,"mercado":p.mercado,"ubicacion":p.ubicacion,"propietario_usuario_id":p.propietario_usuario_id,"activo":p.activo})).await{Ok(x)=>x,Err(CouchDbError::Server{status:409,..})=>return Err(RepositoryError::AlreadyExists),Err(e)=>return Err(Self::err(e))};
        if x.ok {
            Ok(())
        } else {
            Err(RepositoryError::Unexpected(
                "CouchDB no confirmó el puesto".into(),
            ))
        }
    }
    async fn actualizar(&self, p: Puesto) -> Result<(), RepositoryError> {
        Self::require_kind(&p.id, "puesto")?;
        let path = self.path(&p.id)?;
        let old: Value = self.client.get(&path).await.map_err(Self::err)?;
        if old["tipo"] != "puesto" || old["_id"].as_str() != Some(p.id.as_str()) {
            return Err(RepositoryError::Conflict);
        }
        if p.revision.is_none() || old["_rev"].as_str() != p.revision.as_deref() {
            return Err(RepositoryError::Conflict);
        }
        let x:Write=self.client.put(&path,&json!({"_id":p.id,"_rev":old["_rev"],"tipo":"puesto","nombre":p.nombre,"mercado":p.mercado,"ubicacion":p.ubicacion,"propietario_usuario_id":p.propietario_usuario_id,"activo":p.activo})).await.map_err(Self::err)?;
        if x.ok {
            Ok(())
        } else {
            Err(RepositoryError::Unexpected(
                "CouchDB no confirmó la actualización".into(),
            ))
        }
    }
    async fn obtener_por_id(&self, id: &str) -> Result<Option<Puesto>, RepositoryError> {
        Self::require_kind(id, "puesto")?;
        let path = self.path(id)?;
        match self.client.get::<Value>(&path).await {
            Ok(v) => Ok(Some(Self::puesto(&v)?)),
            Err(CouchDbError::Server { status: 404, .. }) => Ok(None),
            Err(e) => Err(Self::err(e)),
        }
    }
    async fn listar(&self, activo: Option<bool>) -> Result<Vec<Puesto>, RepositoryError> {
        let selector = if let Some(value) = activo {
            json!({"tipo":"puesto","activo":value})
        } else {
            json!({"tipo":"puesto"})
        };
        self.client
            .find_all(selector)
            .await
            .map_err(Self::err)?
            .iter()
            .map(Self::puesto)
            .collect()
    }
}
#[async_trait]
impl UsuarioRepository for CouchDbCatalogoRepository {
    async fn crear(&self, u: Usuario) -> Result<(), RepositoryError> {
        Self::require_kind(&u.id, "usuario")?;
        let path = self.path(&u.id)?;
        let rol = match u.rol {
            Rol::Vendedor => "vendedor",
            Rol::Administrador => "administrador",
        };
        let x:Write=match self.client.put(&path,&json!({"_id":u.id,"tipo":"usuario","nombre":u.nombre,"rol":rol,"puesto_id":u.puesto_id,"activo":u.activo})).await{Ok(x)=>x,Err(CouchDbError::Server{status:409,..})=>return Err(RepositoryError::AlreadyExists),Err(e)=>return Err(Self::err(e))};
        if x.ok {
            Ok(())
        } else {
            Err(RepositoryError::Unexpected(
                "CouchDB no confirmó el usuario".into(),
            ))
        }
    }
    async fn actualizar(&self, u: Usuario) -> Result<(), RepositoryError> {
        Self::require_kind(&u.id, "usuario")?;
        let path = self.path(&u.id)?;
        let old: Value = self.client.get(&path).await.map_err(Self::err)?;
        if old["tipo"] != "usuario"
            || old["_id"].as_str() != Some(u.id.as_str())
            || old["puesto_id"].as_str() != Some(u.puesto_id.as_str())
        {
            return Err(RepositoryError::Conflict);
        }
        if u.revision.is_none() || old["_rev"].as_str() != u.revision.as_deref() {
            return Err(RepositoryError::Conflict);
        }
        let rol = match u.rol {
            Rol::Vendedor => "vendedor",
            Rol::Administrador => "administrador",
        };
        let x:Write=self.client.put(&path,&json!({"_id":u.id,"_rev":old["_rev"],"tipo":"usuario","nombre":u.nombre,"rol":rol,"puesto_id":u.puesto_id,"activo":u.activo})).await.map_err(Self::err)?;
        if x.ok {
            Ok(())
        } else {
            Err(RepositoryError::Unexpected(
                "CouchDB no confirmó el usuario".into(),
            ))
        }
    }
    async fn obtener_por_id(&self, id: &str) -> Result<Option<Usuario>, RepositoryError> {
        Self::require_kind(id, "usuario")?;
        let path = self.path(id)?;
        match self.client.get::<Value>(&path).await {
            Ok(v) => Ok(Some(Self::usuario(&v)?)),
            Err(CouchDbError::Server { status: 404, .. }) => Ok(None),
            Err(e) => Err(Self::err(e)),
        }
    }
    async fn listar_por_puesto(&self, p: &str) -> Result<Vec<Usuario>, RepositoryError> {
        let docs = self.list("usuario", "puesto_id", p).await?;
        docs.iter().map(Self::usuario).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_deserializa_un_producto_como_usuario() {
        let document = json!({"_id":"usuario:uno","_rev":"1-test","tipo":"producto",
            "nombre":"dato","rol":"administrador","puesto_id":"puesto:uno","activo":true});
        assert!(CouchDbCatalogoRepository::usuario(&document).is_err());
    }
    #[test]
    fn no_inventa_campos_ausentes() {
        let document = json!({"_id":"puesto:uno","_rev":"1-test","tipo":"puesto","activo":true});
        assert!(CouchDbCatalogoRepository::puesto(&document).is_err());
    }
}
