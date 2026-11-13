use backend::domain::entities::producto::Producto;
use backend::domain::repositories::producto_repository::ProductoRepository;
use backend::infrastructure::couchdb::client::CouchDbClient;
use backend::infrastructure::couchdb::repositories::producto_repository::CouchDbProductoRepository;

#[tokio::test]
#[ignore = "requiere CouchDB aislado y variables MERCAPOS_TEST_* explícitas"]
async fn probar_crud_producto() {
    let database =
        std::env::var("MERCAPOS_TEST_DATABASE").expect("MERCAPOS_TEST_DATABASE requerido");
    assert!(
        database.starts_with("mercapos_test_"),
        "Use una base exclusiva de pruebas"
    );
    let client = CouchDbClient::new(
        std::env::var("MERCAPOS_TEST_URL").unwrap(),
        std::env::var("MERCAPOS_TEST_USERNAME").unwrap(),
        std::env::var("MERCAPOS_TEST_PASSWORD").unwrap(),
        database,
    )
    .unwrap();

    let repositorio = CouchDbProductoRepository::new(client);

    // Generar un ID único para cada ejecución
    let id = format!("producto:producto_test_{}", uuid::Uuid::new_v4());

    let producto = Producto {
        id: id.clone(),
        puesto_id: "puesto:test".to_string(),
        nombre: "Tomate".to_string(),
        categoria: "Verdura".to_string(),
        precio_unitario: 0.75,
        unidad: "kg".to_string(),
        activo: true,
        actualizado_en: "2026-10-29T22:00:00-06:00".to_string(),
        actualizado_por: "usuario:producto_test".to_string(),
        revision: None,
    };

    // CREAR
    repositorio.crear(producto.clone()).await.unwrap();

    // CONSULTAR
    let encontrado = repositorio
        .obtener_por_id(&id)
        .await
        .unwrap()
        .expect("El producto debería existir");

    assert_eq!(encontrado.nombre, "Tomate");
    assert_eq!(encontrado.precio_unitario, 0.75);

    // ACTUALIZAR
    let mut actualizado = encontrado;
    actualizado.precio_unitario = 0.90;

    repositorio.actualizar(actualizado).await.unwrap();

    // COMPROBAR ACTUALIZACIÓN
    let resultado = repositorio
        .obtener_por_id(&id)
        .await
        .unwrap()
        .expect("El producto actualizado debería existir");

    assert_eq!(resultado.precio_unitario, 0.90);

    // LISTAR ACTIVOS
    let productos = repositorio.listar("puesto:test", Some(true)).await.unwrap();

    assert!(productos.iter().any(|p| p.id == id));

    println!("CRUD de productos completado correctamente");
}
