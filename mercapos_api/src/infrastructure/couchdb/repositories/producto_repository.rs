use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domain::entities::producto::Producto;
use crate::domain::repositories::producto_repository::ProductoRepository;
use crate::domain::repositories::repository_error::RepositoryError;

use crate::infrastructure::couchdb::client::CouchDbClient;
use crate::infrastructure::couchdb::error::CouchDbError;

#[derive(Debug, Serialize, Deserialize)]
struct ProductoDocument {
    #[serde(rename = "_id")]
    id: String,

    #[serde(rename = "_rev", skip_serializing_if = "Option::is_none")]
    rev: Option<String>,

    tipo: String,
    puesto_id: String,
    schema_version: Option<u32>,
    nombre: String,
    categoria: String,
    precio_unitario: f64,
    unidad: String,
    activo: bool,
    actualizado_en: String,
    actualizado_por: String,
}

#[derive(Debug, Deserialize)]
struct CouchDbResponse {
    ok: bool,
}

impl From<ProductoDocument> for Producto {
    fn from(documento: ProductoDocument) -> Self {
        Self {
            id: documento.id,
            puesto_id: documento.puesto_id,
            nombre: documento.nombre,
            categoria: documento.categoria,
            precio_unitario: documento.precio_unitario,
            unidad: documento.unidad,
            activo: documento.activo,
            actualizado_en: documento.actualizado_en,
            actualizado_por: documento.actualizado_por,
            revision: documento.rev,
        }
    }
}

impl From<&Producto> for ProductoDocument {
    fn from(producto: &Producto) -> Self {
        Self {
            id: producto.id.clone(),
            puesto_id: producto.puesto_id.clone(),
            schema_version: Some(2),
            rev: producto.revision.clone(),
            tipo: "producto".to_string(),
            nombre: producto.nombre.clone(),
            categoria: producto.categoria.clone(),
            precio_unitario: producto.precio_unitario,
            unidad: producto.unidad.clone(),
            activo: producto.activo,
            actualizado_en: producto.actualizado_en.clone(),
            actualizado_por: producto.actualizado_por.clone(),
        }
    }
}
#[derive(Clone)]
pub struct CouchDbProductoRepository {
    client: CouchDbClient,
}

impl CouchDbProductoRepository {
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
                "El ID del producto contiene caracteres no permitidos".to_string(),
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
}

#[async_trait]
impl ProductoRepository for CouchDbProductoRepository {
    async fn crear(&self, producto: Producto) -> Result<(), RepositoryError> {
        if !producto.id.starts_with("producto:") {
            return Err(RepositoryError::Conflict);
        }
        let ruta = self.ruta_documento(&producto.id)?;

        let documento = ProductoDocument::from(&producto);

        let respuesta: CouchDbResponse = match self.client.put(&ruta, &documento).await {
            Ok(respuesta) => respuesta,

            Err(CouchDbError::Server { status: 409, .. }) => {
                return Err(RepositoryError::AlreadyExists);
            }

            Err(error) => return Err(Self::convertir_error(error)),
        };

        if !respuesta.ok {
            return Err(RepositoryError::Unexpected(
                "CouchDB no confirmó la creación".to_string(),
            ));
        }

        Ok(())
    }

    async fn actualizar(&self, producto: Producto) -> Result<(), RepositoryError> {
        if !producto.id.starts_with("producto:") {
            return Err(RepositoryError::Conflict);
        }
        let ruta = self.ruta_documento(&producto.id)?;

        let mut documento: ProductoDocument = self
            .client
            .get(&ruta)
            .await
            .map_err(Self::convertir_error)?;

        if documento.tipo != "producto"
            || documento.schema_version != Some(2)
            || documento.id != producto.id
            || documento.puesto_id != producto.puesto_id
        {
            return Err(RepositoryError::Conflict);
        }
        if producto.revision.is_none() || documento.rev.as_deref() != producto.revision.as_deref() {
            return Err(RepositoryError::Conflict);
        }

        documento.nombre = producto.nombre;
        documento.categoria = producto.categoria;
        documento.precio_unitario = producto.precio_unitario;
        documento.unidad = producto.unidad;
        documento.activo = producto.activo;
        documento.actualizado_en = producto.actualizado_en;
        documento.actualizado_por = producto.actualizado_por;

        // Conservamos el _rev que CouchDB necesita para actualizar.
        let respuesta: CouchDbResponse = self
            .client
            .put(&ruta, &documento)
            .await
            .map_err(Self::convertir_error)?;

        if !respuesta.ok {
            return Err(RepositoryError::Unexpected(
                "CouchDB no confirmó la actualización".to_string(),
            ));
        }

        Ok(())
    }

    async fn obtener_por_id(&self, id: &str) -> Result<Option<Producto>, RepositoryError> {
        if !id.starts_with("producto:") {
            return Err(RepositoryError::Conflict);
        }
        let ruta = self.ruta_documento(id)?;

        match self.client.get::<ProductoDocument>(&ruta).await {
            Ok(documento) => {
                if documento.tipo != "producto"
                    || documento.schema_version != Some(2)
                    || documento.id != id
                {
                    return Err(RepositoryError::Conflict);
                }
                Ok(Some(documento.into()))
            }

            Err(CouchDbError::Server { status: 404, .. }) => Ok(None),

            Err(error) => Err(Self::convertir_error(error)),
        }
    }

    async fn listar(
        &self,
        puesto_id: &str,
        activo: Option<bool>,
    ) -> Result<Vec<Producto>, RepositoryError> {
        let mut selector = json!({
            "tipo": "producto", "puesto_id": puesto_id
        });

        if let Some(estado) = activo {
            selector["activo"] = json!(estado);
        }

        self.client
            .find_all(selector)
            .await
            .map_err(Self::convertir_error)?
            .into_iter()
            .map(|doc| {
                let parsed: ProductoDocument = serde_json::from_value(doc)
                    .map_err(|e| RepositoryError::Unexpected(e.to_string()))?;
                if parsed.tipo != "producto"
                    || parsed.schema_version != Some(2)
                    || !parsed.id.starts_with("producto:")
                {
                    return Err(RepositoryError::Unexpected(
                        "Documento producto incompatible".into(),
                    ));
                }
                Ok(parsed.into())
            })
            .collect()
    }
}
