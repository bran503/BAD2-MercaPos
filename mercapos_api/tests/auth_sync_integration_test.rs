use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header::AUTHORIZATION, header::CONTENT_TYPE},
};
use backend::{
    app::build_app,
    app_state::AppState,
    application::auth::{ahora_epoch, hash_password},
    domain::{
        entities::{
            credencial::Credencial,
            puesto::Puesto,
            usuario::{Rol, Usuario},
        },
        repositories::{
            auth_repository::AuthRepository, puesto_repository::PuestoRepository,
            usuario_repository::UsuarioRepository,
        },
    },
    infrastructure::couchdb::{
        client::CouchDbClient,
        repositories::{
            auth_repository::CouchDbAuthRepository, catalogo_repository::CouchDbCatalogoRepository,
            conteo_repository::CouchDbConteoRepository,
            incidencia_repository::CouchDbIncidenciaRepository,
            movimiento_repository::CouchDbMovimientoRepository,
            operacion_repository::CouchDbOperacionRepository,
            producto_repository::CouchDbProductoRepository,
        },
    },
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

fn variable(nombre: &str) -> String {
    std::env::var(nombre).unwrap_or_else(|_| panic!("{nombre} requerido"))
}

async fn construir_app() -> Router {
    let database = variable("MERCAPOS_TEST_DATABASE");
    assert!(
        database.starts_with("mercapos_test_"),
        "Use una base exclusiva de pruebas"
    );
    let auth_database = variable("MERCAPOS_TEST_AUTH_DATABASE");
    assert!(
        auth_database.starts_with("mercapos_test_auth_"),
        "Use una base de autenticación exclusiva de pruebas"
    );
    assert_ne!(database, auth_database, "Aísle negocio y autenticación");
    let couchdb = CouchDbClient::new(
        variable("MERCAPOS_TEST_URL"),
        variable("MERCAPOS_TEST_USERNAME"),
        variable("MERCAPOS_TEST_PASSWORD"),
        database.clone(),
    )
    .unwrap();
    let auth_couchdb = CouchDbClient::new(
        variable("MERCAPOS_TEST_URL"),
        variable("MERCAPOS_TEST_USERNAME"),
        variable("MERCAPOS_TEST_PASSWORD"),
        auth_database,
    )
    .unwrap();
    assert!(
        couchdb
            .get::<Value>(&format!("{}/_design/validacion_v2", database))
            .await
            .is_ok(),
        "La base de pruebas requiere el esquema v2 (validacion_v2) instalado con instalar_esquema.py"
    );
    let catalogos = Arc::new(CouchDbCatalogoRepository::new(couchdb.clone()));
    let state = AppState::new(
        couchdb.clone(),
        Arc::new(CouchDbProductoRepository::new(couchdb.clone())),
        Arc::new(CouchDbMovimientoRepository::new(couchdb.clone())),
        Arc::new(CouchDbOperacionRepository::new(couchdb.clone())),
        Arc::new(CouchDbConteoRepository::new(couchdb.clone())),
        catalogos.clone(),
        catalogos,
        Arc::new(CouchDbAuthRepository::new(auth_couchdb)),
        Arc::new(CouchDbIncidenciaRepository::new(couchdb)),
    );
    build_app(state)
}

async fn peticion(
    app: &Router,
    metodo: Method,
    ruta: &str,
    token: Option<&str>,
    cuerpo: Option<Value>,
) -> (StatusCode, Value) {
    let mut constructor = Request::builder().method(metodo).uri(ruta);
    if cuerpo.is_some() {
        constructor = constructor.header(CONTENT_TYPE, "application/json");
    }
    if let Some(token) = token {
        constructor = constructor.header(AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = cuerpo
        .map(|valor| Body::from(serde_json::to_vec(&valor).unwrap()))
        .unwrap_or_else(Body::empty);
    let respuesta = app
        .clone()
        .oneshot(constructor.body(body).unwrap())
        .await
        .unwrap();
    let estado = respuesta.status();
    let bytes = respuesta.into_body().collect().await.unwrap().to_bytes();
    let valor = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (estado, valor)
}

async fn login(app: &Router, usuario_id: &str, password: &str) -> String {
    let (estado, cuerpo) = peticion(
        app,
        Method::POST,
        "/auth/login",
        None,
        Some(json!({"usuario_id": usuario_id, "password": password})),
    )
    .await;
    assert_eq!(estado, StatusCode::OK, "login falló: {cuerpo}");
    cuerpo["token"].as_str().unwrap().to_owned()
}

#[tokio::test]
#[ignore = "requiere CouchDB aislado y variables MERCAPOS_TEST_* explícitas"]
async fn autenticacion_autorizacion_y_sincronizacion() {
    let app = construir_app().await;
    let sufijo = uuid::Uuid::new_v4().simple().to_string();
    let puesto_a = format!("puesto:a_{sufijo}");
    let puesto_b = format!("puesto:b_{sufijo}");
    let admin_a = format!("usuario:admin_{sufijo}");
    let vendedor_a = format!("usuario:vendedor_a_{sufijo}");
    let vendedor_b = format!("usuario:vendedor_b_{sufijo}");
    let producto_a = format!("producto:a_{sufijo}");
    let producto_faltante = format!("producto:inexistente_{sufijo}");
    let password = "secreto-123";
    let database = variable("MERCAPOS_TEST_DATABASE");
    let couchdb = CouchDbClient::new(
        variable("MERCAPOS_TEST_URL"),
        variable("MERCAPOS_TEST_USERNAME"),
        variable("MERCAPOS_TEST_PASSWORD"),
        database,
    )
    .unwrap();
    let catalogos = CouchDbCatalogoRepository::new(couchdb.clone());
    let auth_couchdb = CouchDbClient::new(
        variable("MERCAPOS_TEST_URL"),
        variable("MERCAPOS_TEST_USERNAME"),
        variable("MERCAPOS_TEST_PASSWORD"),
        variable("MERCAPOS_TEST_AUTH_DATABASE"),
    )
    .unwrap();
    let auth = CouchDbAuthRepository::new(auth_couchdb);
    for (id, nombre) in [(&puesto_a, "Puesto A"), (&puesto_b, "Puesto B")] {
        PuestoRepository::crear(
            &catalogos,
            Puesto {
                id: id.clone(),
                nombre: nombre.to_string(),
                mercado: "Mercado de prueba".to_string(),
                ubicacion: "Pasillo 1".to_string(),
                propietario_usuario_id: admin_a.clone(),
                activo: true,
                revision: None,
            },
        )
        .await
        .unwrap();
    }
    for (id, rol, puesto) in [
        (&admin_a, Rol::Administrador, &puesto_a),
        (&vendedor_a, Rol::Vendedor, &puesto_a),
        (&vendedor_b, Rol::Vendedor, &puesto_b),
    ] {
        UsuarioRepository::crear(
            &catalogos,
            Usuario {
                id: id.clone(),
                nombre: id.clone(),
                rol,
                puesto_id: puesto.clone(),
                activo: true,
                revision: None,
            },
        )
        .await
        .unwrap();
        auth.guardar_credencial(Credencial {
            usuario_id: id.clone(),
            password_hash: hash_password(password).unwrap(),
            actualizado_en: ahora_epoch(),
        })
        .await
        .unwrap();
    }

    let (estado, _) = peticion(
        &app,
        Method::GET,
        &format!("/productos?puesto_id={puesto_a}"),
        None,
        None,
    )
    .await;
    assert_eq!(estado, StatusCode::UNAUTHORIZED);

    let (estado, _) = peticion(
        &app,
        Method::POST,
        "/auth/login",
        None,
        Some(json!({"usuario_id": admin_a, "password": "incorrecta"})),
    )
    .await;
    assert_eq!(estado, StatusCode::UNAUTHORIZED);

    let token_admin = login(&app, &admin_a, password).await;
    let token_vendedor = login(&app, &vendedor_a, password).await;
    let token_vendedor_b = login(&app, &vendedor_b, password).await;

    let (estado, cuerpo) = peticion(&app, Method::GET, "/auth/me", Some(&token_admin), None).await;
    assert_eq!(estado, StatusCode::OK);
    assert_eq!(cuerpo["usuario"]["id"], json!(admin_a));

    let (estado, cuerpo) = peticion(&app, Method::GET, "/puestos", Some(&token_admin), None).await;
    assert_eq!(estado, StatusCode::OK);
    assert!(
        cuerpo
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["id"] == json!(puesto_a))
    );

    let (estado, _) = peticion(
        &app,
        Method::POST,
        "/usuarios",
        Some(&token_admin),
        Some(json!({
            "id": format!("usuario:cross_{sufijo}"),
            "nombre": "Fuera de ámbito",
            "rol": "vendedor",
            "puesto_id": puesto_b,
            "password": password,
        })),
    )
    .await;
    assert_eq!(estado, StatusCode::FORBIDDEN);

    let (estado, cuerpo) = peticion(
        &app,
        Method::POST,
        "/productos",
        Some(&token_admin),
        Some(json!({
            "id": producto_a,
            "puesto_id": puesto_a,
            "nombre": "Tomate",
            "categoria": "Verdura",
            "precio_unitario": 1.25,
            "unidad": "kg",
            "actualizado_en": "2026-11-01T10:00:00-06:00",
            "actualizado_por": admin_a,
        })),
    )
    .await;
    assert_eq!(estado, StatusCode::CREATED, "crear producto: {cuerpo}");

    let (estado, _) = peticion(
        &app,
        Method::GET,
        &format!("/productos?puesto_id={puesto_a}"),
        Some(&token_vendedor_b),
        None,
    )
    .await;
    assert_eq!(estado, StatusCode::FORBIDDEN);

    let (estado, _) = peticion(
        &app,
        Method::POST,
        "/usuarios",
        Some(&token_vendedor),
        Some(json!({
            "id": format!("usuario:otro_{sufijo}"),
            "nombre": "Otro",
            "rol": "vendedor",
            "puesto_id": puesto_a,
        })),
    )
    .await;
    assert_eq!(estado, StatusCode::FORBIDDEN);

    let (estado, cuerpo) = peticion(
        &app,
        Method::POST,
        "/movimientos",
        Some(&token_admin),
        Some(json!({
            "id": format!("mov:inicial_{sufijo}"),
            "producto_id": producto_a,
            "puesto_id": puesto_a,
            "cantidad": 10,
            "motivo": "inventario_inicial",
            "dispositivo_id": "dispositivo-prueba",
            "usuario_id": admin_a,
            "fecha_local": "2026-11-01T10:00:00-06:00",
        })),
    )
    .await;
    assert_eq!(estado, StatusCode::CREATED, "stock inicial: {cuerpo}");

    let venta_id = format!("operacion:venta_{sufijo}");
    let (estado, cuerpo) = peticion(
        &app,
        Method::POST,
        "/operaciones",
        Some(&token_vendedor),
        Some(json!({
            "id": venta_id,
            "tipo_operacion": "venta",
            "puesto_id": puesto_a,
            "usuario_id": vendedor_a,
            "dispositivo_id": "dispositivo-prueba",
            "fecha_local": "2026-11-01T10:05:00-06:00",
            "renglones": [{"producto_id": producto_a, "cantidad": 2, "precio_unitario": 1.25}],
            "total": 2.50,
            "pago": {
                "metodo": "tarjeta",
                "modo": "simulado",
                "estado": "aprobado",
                "autorizacion": "AUT-123456",
                "fecha": "2026-11-01T10:05:00-06:00",
            },
        })),
    )
    .await;
    assert_eq!(estado, StatusCode::CREATED, "venta: {cuerpo}");

    let (estado, cuerpo) = peticion(
        &app,
        Method::GET,
        &format!("/operaciones/{venta_id}"),
        Some(&token_vendedor),
        None,
    )
    .await;
    assert_eq!(estado, StatusCode::OK);
    assert_eq!(cuerpo["pago"]["autorizacion"], json!("AUT-123456"));

    let (estado, _) = peticion(
        &app,
        Method::GET,
        &format!("/operaciones/{venta_id}"),
        Some(&token_vendedor_b),
        None,
    )
    .await;
    assert_eq!(estado, StatusCode::FORBIDDEN);

    let (estado, _) = peticion(
        &app,
        Method::POST,
        "/operaciones",
        Some(&token_vendedor),
        Some(json!({
            "id": format!("operacion:suplantada_{sufijo}"),
            "tipo_operacion": "venta",
            "puesto_id": puesto_a,
            "usuario_id": admin_a,
            "dispositivo_id": "dispositivo-prueba",
            "fecha_local": "2026-11-01T10:06:00-06:00",
            "renglones": [{"producto_id": producto_a, "cantidad": 1, "precio_unitario": 1.25}],
            "total": 1.25,
        })),
    )
    .await;
    assert_eq!(estado, StatusCode::FORBIDDEN);

    let sync_ok = format!("operacion:sync_ok_{sufijo}");
    let sync_aviso = format!("operacion:sync_aviso_{sufijo}");
    let sync_stock = format!("operacion:sync_stock_{sufijo}");
    let sync_error = format!("operacion:sync_error_{sufijo}");
    let (estado, cuerpo) = peticion(
        &app,
        Method::POST,
        "/sync/operaciones",
        Some(&token_vendedor),
        Some(json!({
            "operaciones": [
                {
                    "id": sync_ok,
                    "tipo_operacion": "venta",
                    "puesto_id": puesto_a,
                    "usuario_id": vendedor_a,
                    "dispositivo_id": "dispositivo-prueba",
                    "fecha_local": "2026-11-01T10:07:00-06:00",
                    "renglones": [{"producto_id": producto_a, "cantidad": 1, "precio_unitario": 1.25}],
                    "total": 1.25,
                },
                {
                    "id": sync_aviso,
                    "tipo_operacion": "venta",
                    "puesto_id": puesto_a,
                    "usuario_id": vendedor_a,
                    "dispositivo_id": "dispositivo-prueba",
                    "fecha_local": "2026-11-01T10:08:00-06:00",
                    "renglones": [{"producto_id": producto_a, "cantidad": 1, "precio_unitario": 1.50}],
                    "total": 1.50,
                },
                {
                    "id": sync_stock,
                    "tipo_operacion": "venta",
                    "puesto_id": puesto_a,
                    "usuario_id": vendedor_a,
                    "dispositivo_id": "dispositivo-prueba",
                    "fecha_local": "2026-11-01T10:08:30-06:00",
                    "renglones": [{"producto_id": producto_a, "cantidad": 100, "precio_unitario": 1.25}],
                    "total": 125.00,
                },
                {
                    "id": sync_error,
                    "tipo_operacion": "venta",
                    "puesto_id": puesto_a,
                    "usuario_id": vendedor_a,
                    "dispositivo_id": "dispositivo-prueba",
                    "fecha_local": "2026-11-01T10:09:00-06:00",
                    "renglones": [{"producto_id": producto_faltante, "cantidad": 1, "precio_unitario": 1.00}],
                    "total": 1.00,
                }
            ]
        })),
    )
    .await;
    assert_eq!(estado, StatusCode::OK, "sync: {cuerpo}");
    assert_eq!(cuerpo["aceptadas"], json!(1));
    assert_eq!(cuerpo["con_incidencia"], json!(2));
    assert_eq!(cuerpo["rechazadas"], json!(1));
    let resultados = cuerpo["resultados"].as_array().unwrap();
    assert!(
        resultados
            .iter()
            .any(|r| { r["operacion_id"] == json!(sync_ok) && r["estado"] == json!("aceptada") })
    );
    assert!(resultados.iter().any(|r| {
        r["operacion_id"] == json!(sync_aviso) && r["estado"] == json!("aceptada_con_incidencia")
    }));
    assert!(resultados.iter().any(|r| {
        r["operacion_id"] == json!(sync_stock) && r["estado"] == json!("aceptada_con_incidencia")
    }));
    assert!(
        resultados.iter().any(|r| {
            r["operacion_id"] == json!(sync_error) && r["estado"] == json!("rechazada")
        })
    );

    let (estado, cuerpo) = peticion(
        &app,
        Method::GET,
        &format!("/sync/incidencias?puesto_id={puesto_a}"),
        Some(&token_vendedor),
        None,
    )
    .await;
    assert_eq!(estado, StatusCode::OK, "incidencias: {cuerpo}");
    let incidencias = cuerpo["incidencias"].as_array().unwrap();
    assert!(
        incidencias.iter().any(|i| {
            i["operacion_id"] == json!(sync_aviso) && i["tipo"] == json!("advertencia")
        })
    );
    assert!(
        incidencias.iter().any(|i| {
            i["operacion_id"] == json!(sync_stock) && i["tipo"] == json!("advertencia")
        })
    );
    assert!(
        incidencias
            .iter()
            .any(|i| { i["operacion_id"] == json!(sync_error) && i["tipo"] == json!("rechazo") })
    );

    let (estado, cuerpo) = peticion(
        &app,
        Method::GET,
        &format!("/operaciones/{sync_stock}"),
        Some(&token_vendedor),
        None,
    )
    .await;
    assert_eq!(
        estado,
        StatusCode::OK,
        "venta con stock insuficiente: {cuerpo}"
    );
    assert_eq!(cuerpo["id"], json!(sync_stock));

    let (estado, cuerpo) = peticion(
        &app,
        Method::GET,
        &format!("/puestos/{puesto_a}/existencias"),
        Some(&token_admin),
        None,
    )
    .await;
    assert_eq!(estado, StatusCode::OK, "existencias: {cuerpo}");
    let existencia = cuerpo
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["producto_id"] == json!(producto_a))
        .unwrap();
    assert_eq!(existencia["existencia_mil"], json!(-94000));

    let (estado, _) = peticion(
        &app,
        Method::GET,
        &format!("/sync/incidencias?puesto_id={puesto_a}"),
        Some(&token_vendedor_b),
        None,
    )
    .await;
    assert_eq!(estado, StatusCode::FORBIDDEN);

    let (estado, _) = peticion(
        &app,
        Method::PUT,
        &format!("/sync/incidencias/{sync_error}/resolver"),
        Some(&token_vendedor),
        Some(json!({"nota": "revisado"})),
    )
    .await;
    assert_eq!(estado, StatusCode::FORBIDDEN);

    let (estado, cuerpo) = peticion(
        &app,
        Method::PUT,
        &format!("/sync/incidencias/{sync_error}/resolver"),
        Some(&token_admin),
        Some(json!({"nota": "corregido en tienda"})),
    )
    .await;
    assert_eq!(estado, StatusCode::OK, "resolver: {cuerpo}");
    assert_eq!(cuerpo["incidencia"]["resuelta"], json!(true));

    let (estado, _) = peticion(
        &app,
        Method::POST,
        "/auth/logout",
        Some(&token_vendedor),
        None,
    )
    .await;
    assert_eq!(estado, StatusCode::NO_CONTENT);
    let (estado, _) = peticion(&app, Method::GET, "/auth/me", Some(&token_vendedor), None).await;
    assert_eq!(estado, StatusCode::UNAUTHORIZED);
}
