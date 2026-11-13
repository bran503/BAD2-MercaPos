use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use backend::domain::entities::producto::Producto;
use backend::domain::repositories::producto_repository::ProductoRepository;
use backend::domain::repositories::repository_error::RepositoryError;

use backend::application::use_cases::producto::actualizar_producto::{
    ActualizarProducto, ActualizarProductoError,
};
use backend::application::use_cases::producto::crear_producto::{
    CrearProducto, CrearProductoError,
};

struct ProductoRepositoryFake {
    productos: Mutex<HashMap<String, Producto>>,
}

impl ProductoRepositoryFake {
    fn new() -> Self {
        Self {
            productos: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ProductoRepository for ProductoRepositoryFake {
    async fn crear(&self, producto: Producto) -> Result<(), RepositoryError> {
        let mut productos = self.productos.lock().unwrap();

        if productos.contains_key(&producto.id) {
            return Err(RepositoryError::AlreadyExists);
        }

        productos.insert(producto.id.clone(), producto);
        Ok(())
    }

    async fn actualizar(&self, producto: Producto) -> Result<(), RepositoryError> {
        let mut productos = self.productos.lock().unwrap();

        if !productos.contains_key(&producto.id) {
            return Err(RepositoryError::NotFound);
        }

        productos.insert(producto.id.clone(), producto);
        Ok(())
    }

    async fn obtener_por_id(&self, id: &str) -> Result<Option<Producto>, RepositoryError> {
        let productos = self.productos.lock().unwrap();
        Ok(productos.get(id).cloned())
    }

    async fn listar(
        &self,
        puesto_id: &str,
        activo: Option<bool>,
    ) -> Result<Vec<Producto>, RepositoryError> {
        let productos = self.productos.lock().unwrap();

        Ok(productos
            .values()
            .filter(|p| p.puesto_id == puesto_id && activo.is_none_or(|estado| p.activo == estado))
            .cloned()
            .collect())
    }
}
fn producto_prueba() -> Producto {
    Producto {
        id: "producto:producto_test_001".to_string(),
        puesto_id: "puesto:0001".to_string(),
        nombre: "Tomate".to_string(),
        categoria: "Verdura".to_string(),
        precio_unitario: 0.75,
        unidad: "kg".to_string(),
        activo: true,
        actualizado_en: "2026-10-29T22:00:00-06:00".to_string(),
        actualizado_por: "usuario_001".to_string(),
        revision: None,
    }
}

#[tokio::test]
async fn crear_producto_valido() {
    let repositorio = Arc::new(ProductoRepositoryFake::new());
    let caso_uso = CrearProducto::new(repositorio.clone());

    let producto = producto_prueba();

    let resultado = caso_uso.ejecutar(producto.clone()).await;

    assert!(resultado.is_ok());

    let encontrado = repositorio.obtener_por_id(&producto.id).await.unwrap();

    assert!(encontrado.is_some());
}
#[tokio::test]
async fn rechazar_precio_negativo() {
    let repositorio = Arc::new(ProductoRepositoryFake::new());
    let caso_uso = CrearProducto::new(repositorio);

    let mut producto = producto_prueba();
    producto.precio_unitario = -5.0;

    let resultado = caso_uso.ejecutar(producto).await;

    assert!(matches!(
        resultado,
        Err(CrearProductoError::DatosInvalidos(_))
    ));
}
#[tokio::test]
async fn rechazar_nombre_vacio() {
    let repositorio = Arc::new(ProductoRepositoryFake::new());
    let caso_uso = CrearProducto::new(repositorio);

    let mut producto = producto_prueba();
    producto.nombre = "   ".to_string();

    let resultado = caso_uso.ejecutar(producto).await;

    assert!(matches!(
        resultado,
        Err(CrearProductoError::DatosInvalidos(_))
    ));
}

#[tokio::test]
async fn actualizar_producto_inexistente() {
    let repositorio = Arc::new(ProductoRepositoryFake::new());
    let caso_uso = ActualizarProducto::new(repositorio);

    let producto = producto_prueba();

    let resultado = caso_uso.ejecutar(producto).await;

    assert!(matches!(
        resultado,
        Err(ActualizarProductoError::Repositorio(
            RepositoryError::NotFound
        ))
    ));
}
