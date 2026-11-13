use reqwest::{Client, Method, Url, header::ACCEPT};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::time::Duration;

use super::error::CouchDbError;

#[derive(Clone)]
pub struct CouchDbClient {
    client: Client,
    base_url: String,
    username: String,
    password: String,
    database: String,
}

#[derive(Deserialize)]
struct FindPage {
    docs: Vec<Value>,
    bookmark: Option<String>,
}

impl CouchDbClient {
    pub fn new(
        base_url: String,
        username: String,
        password: String,
        database: String,
    ) -> Result<Self, CouchDbError> {
        let mut parsed_url =
            Url::parse(&base_url).map_err(|e| CouchDbError::Configuration(e.to_string()))?;

        if !matches!(parsed_url.scheme(), "http" | "https") {
            return Err(CouchDbError::Configuration(
                "La URL debe utilizar HTTP o HTTPS".to_string(),
            ));
        }

        if !parsed_url.path().ends_with('/') {
            let path = format!("{}/", parsed_url.path());
            parsed_url.set_path(&path);
        }

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: parsed_url.to_string(),
            username,
            password,
            database,
        })
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn send<T, R>(
        &self,
        method: Method,
        path: &str,
        body: Option<&T>,
    ) -> Result<R, CouchDbError>
    where
        T: Serialize + Sync,
        R: DeserializeOwned,
    {
        let metodo = method.as_str().to_owned();
        let mut request = self
            .client
            .request(method, self.endpoint(path))
            .basic_auth(&self.username, Some(&self.password))
            .header(ACCEPT, "application/json");

        if let Some(data) = body {
            request = request.json(data);
        }

        let response = request.send().await?;

        let status = response.status();

        if !status.is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Sin detalles".to_string());
            tracing::error!(
                status = status.as_u16(),
                metodo = %metodo,
                ruta = %path,
                detalle = %message,
                "CouchDB rechazó la petición"
            );

            return Err(CouchDbError::Server {
                status: status.as_u16(),
                message,
            });
        }

        Ok(response.json::<R>().await?)
    }

    pub async fn get<R>(&self, path: &str) -> Result<R, CouchDbError>
    where
        R: DeserializeOwned,
    {
        self.send::<serde_json::Value, R>(Method::GET, path, None)
            .await
    }

    pub async fn put<T, R>(&self, path: &str, body: &T) -> Result<R, CouchDbError>
    where
        T: Serialize + Sync,
        R: DeserializeOwned,
    {
        self.send(Method::PUT, path, Some(body)).await
    }

    pub async fn post<T, R>(&self, path: &str, body: &T) -> Result<R, CouchDbError>
    where
        T: Serialize + Sync,
        R: DeserializeOwned,
    {
        self.send(Method::POST, path, Some(body)).await
    }

    pub async fn delete<R>(&self, path: &str) -> Result<R, CouchDbError>
    where
        R: DeserializeOwned,
    {
        self.send::<serde_json::Value, R>(Method::DELETE, path, None)
            .await
    }

    pub fn database(&self) -> &str {
        &self.database
    }

    pub async fn find_all(&self, selector: Value) -> Result<Vec<Value>, CouchDbError> {
        self.find_all_with_limit(selector, 500).await
    }

    /// Recorre por completo el selector mediante los bookmarks de CouchDB.
    /// El límite es configurable para probar el avance entre páginas con pocos
    /// documentos, pero siempre se mantiene acotado.
    pub async fn find_all_with_limit(
        &self,
        selector: Value,
        limit: usize,
    ) -> Result<Vec<Value>, CouchDbError> {
        if !(1..=500).contains(&limit) {
            return Err(CouchDbError::Configuration(
                "El límite de página debe estar entre 1 y 500".into(),
            ));
        }
        let path = format!("{}/_find", self.database);
        let mut bookmark: Option<String> = None;
        let mut documents = Vec::new();
        loop {
            let mut query = json!({"selector":selector.clone(),"limit":limit});
            if let Some(cursor) = &bookmark {
                query["bookmark"] = json!(cursor);
            }
            let page: FindPage = self.post(&path, &query).await?;
            let size = page.docs.len();
            documents.extend(page.docs);
            // A short page does not prove that the cursor is exhausted. Keep
            // following the server bookmark until it returns no documents.
            if size == 0 {
                return Ok(documents);
            }
            if page.bookmark.is_none() || page.bookmark == bookmark {
                return Err(CouchDbError::Configuration(
                    "CouchDB no avanzó a la página siguiente".into(),
                ));
            }
            bookmark = page.bookmark;
        }
    }

    pub async fn verificar_conexion(&self) -> Result<serde_json::Value, CouchDbError> {
        self.get("_up").await
    }
}
