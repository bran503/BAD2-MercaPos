use backend::infrastructure::couchdb::client::CouchDbClient;
use serde_json::{Value, json};

fn variable(nombre: &str) -> String {
    std::env::var(nombre).unwrap_or_else(|_| panic!("{nombre} requerido"))
}

#[tokio::test]
#[ignore = "requiere CouchDB aislado y variables MERCAPOS_TEST_* explícitas"]
async fn recorre_todas_las_paginas_con_pocos_documentos() {
    let database = variable("MERCAPOS_TEST_DATABASE");
    assert!(
        database.starts_with("mercapos_test_"),
        "Use una base de prueba"
    );
    let client = CouchDbClient::new(
        variable("MERCAPOS_TEST_URL"),
        variable("MERCAPOS_TEST_USERNAME"),
        variable("MERCAPOS_TEST_PASSWORD"),
        database,
    )
    .unwrap();
    // El esquema v2 no permite borrar documentos comerciales, así que los
    // fixtures usan identificadores estables y se sobrescriben en cada corrida.
    let run = "paginacion";
    let mut ids = Vec::new();
    for index in 0..25 {
        let id = format!("producto:paginacion_{index}");
        ids.push(id.clone());
        let path = format!("{}/{}", client.database(), id);
        let mut document = json!({
            "_id": id,
            "tipo": "producto",
            "schema_version": 2,
            "puesto_id": "puesto:test_paginacion",
            "test_run": run,
            "nombre": format!("Prueba {index}"),
            "categoria": "prueba",
            "precio_unitario": 1.0,
            "unidad": "unidad",
            "activo": true,
            "actualizado_en": "2026-11-01T12:00:00Z",
            "actualizado_por": "usuario:test_paginacion"
        });
        if let Ok(existente) = client.get::<Value>(&path).await
            && let Some(revision) = existente.get("_rev").and_then(Value::as_str)
        {
            document["_rev"] = json!(revision);
        }
        let response: Value = client.put(&path, &document).await.unwrap();
        assert_eq!(response["ok"], true);
    }

    let result = client
        .find_all_with_limit(json!({"tipo":"producto","test_run":run}), 10)
        .await;

    let documents = result.unwrap();
    assert_eq!(
        documents.len(),
        25,
        "el selector omitió documentos de otra página"
    );
    assert!(documents.iter().all(|document| document["test_run"] == run));
    let mut persistidos = 0;
    for id in &ids {
        let path = format!("{}/{}", client.database(), id);
        if client.get::<Value>(&path).await.is_ok() {
            persistidos += 1;
        }
    }
    assert_eq!(
        persistidos, 25,
        "los fixtures deben quedar en la base de prueba"
    );
}
