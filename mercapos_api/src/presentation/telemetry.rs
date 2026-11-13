use crate::app_state::AppState;
use crate::presentation::auth::AuthUser;
use axum::{
    body::Body,
    extract::{Extension, MatchedPath, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

#[derive(Default)]
struct RouteStats {
    requests: u64,
    errors: u64,
    elapsed_ms: u64,
}

const BUCKETS_LATENCIA_MS: [u64; 12] = [1, 2, 5, 10, 20, 50, 100, 250, 500, 1000, 2500, 5000];

#[derive(Default)]
struct LatencyStats {
    // Conteo acumulado por bucket (le), como exige el formato Prometheus.
    buckets: [u64; BUCKETS_LATENCIA_MS.len()],
    sum_ms: u64,
    count: u64,
}

#[derive(Default)]
pub struct Telemetry {
    requests: AtomicU64,
    errors: AtomicU64,
    elapsed_ms: AtomicU64,
    by_route: Mutex<BTreeMap<(String, String, u16), RouteStats>>,
    latency: Mutex<BTreeMap<(String, String), LatencyStats>>,
}
impl Telemetry {
    fn label(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    }

    pub fn exposition(&self) -> String {
        let mut output = format!(
            "# HELP mercapos_http_requests_total Total HTTP requests\n# TYPE mercapos_http_requests_total counter\nmercapos_http_requests_total {}\n# HELP mercapos_http_errors_total HTTP 5xx responses\n# TYPE mercapos_http_errors_total counter\nmercapos_http_errors_total {}\n# HELP mercapos_http_request_duration_milliseconds_total Cumulative request duration\n# TYPE mercapos_http_request_duration_milliseconds_total counter\nmercapos_http_request_duration_milliseconds_total {}\n",
            self.requests.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
            self.elapsed_ms.load(Ordering::Relaxed)
        );
        output.push_str(
            "# HELP mercapos_http_route_requests_total HTTP requests by method, normalized route and status\n# TYPE mercapos_http_route_requests_total counter\n",
        );
        output.push_str(
            "# HELP mercapos_http_route_errors_total HTTP 5xx responses by method and normalized route\n# TYPE mercapos_http_route_errors_total counter\n",
        );
        output.push_str(
            "# HELP mercapos_http_route_duration_milliseconds_total Cumulative HTTP duration by method and normalized route\n# TYPE mercapos_http_route_duration_milliseconds_total counter\n",
        );
        let by_route = self
            .by_route
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        for ((method, route, status), stats) in by_route.iter() {
            let method = Self::label(method);
            let route = Self::label(route);
            output.push_str(&format!(
                "mercapos_http_route_requests_total{{method=\"{method}\",route=\"{route}\",status=\"{status}\"}} {}\n",
                stats.requests
            ));
            if stats.errors > 0 {
                output.push_str(&format!(
                    "mercapos_http_route_errors_total{{method=\"{method}\",route=\"{route}\"}} {}\n",
                    stats.errors
                ));
            }
            output.push_str(&format!(
                "mercapos_http_route_duration_milliseconds_total{{method=\"{method}\",route=\"{route}\"}} {}\n",
                stats.elapsed_ms
            ));
        }
        output.push_str(
            "# HELP mercapos_http_request_duration_milliseconds HTTP request duration in milliseconds by method and normalized route\n# TYPE mercapos_http_request_duration_milliseconds histogram\n",
        );
        let latency = self
            .latency
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        for ((method, route), stats) in latency.iter() {
            let method = Self::label(method);
            let route = Self::label(route);
            for (index, bucket) in BUCKETS_LATENCIA_MS.iter().enumerate() {
                output.push_str(&format!(
                    "mercapos_http_request_duration_milliseconds_bucket{{method=\"{method}\",route=\"{route}\",le=\"{bucket}\"}} {}\n",
                    stats.buckets[index]
                ));
            }
            output.push_str(&format!(
                "mercapos_http_request_duration_milliseconds_bucket{{method=\"{method}\",route=\"{route}\",le=\"+Inf\"}} {}\n",
                stats.count
            ));
            output.push_str(&format!(
                "mercapos_http_request_duration_milliseconds_sum{{method=\"{method}\",route=\"{route}\"}} {}\n",
                stats.sum_ms
            ));
            output.push_str(&format!(
                "mercapos_http_request_duration_milliseconds_count{{method=\"{method}\",route=\"{route}\"}} {}\n",
                stats.count
            ));
        }
        drop(latency);
        let (threads, rss_bytes) = process_metrics();
        output.push_str(&format!(
            "# HELP mercapos_process_threads Threads of the API process\n# TYPE mercapos_process_threads gauge\nmercapos_process_threads {threads}\n# HELP mercapos_process_resident_memory_bytes Resident memory of the API process\n# TYPE mercapos_process_resident_memory_bytes gauge\nmercapos_process_resident_memory_bytes {rss_bytes}\n"
        ));
        output
    }

    fn record(&self, method: &str, route: &str, status: u16, elapsed_ms: u64) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.elapsed_ms.fetch_add(elapsed_ms, Ordering::Relaxed);
        if (500..600).contains(&status) {
            self.errors.fetch_add(1, Ordering::Relaxed);
        }
        let mut routes = self
            .by_route
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let stats = routes
            .entry((method.to_owned(), route.to_owned(), status))
            .or_default();
        stats.requests += 1;
        stats.elapsed_ms += elapsed_ms;
        if (500..600).contains(&status) {
            stats.errors += 1;
        }
        drop(routes);
        let mut latency = self
            .latency
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let latency_stats = latency
            .entry((method.to_owned(), route.to_owned()))
            .or_default();
        latency_stats.count += 1;
        latency_stats.sum_ms += elapsed_ms;
        for (index, bucket) in BUCKETS_LATENCIA_MS.iter().enumerate() {
            if elapsed_ms <= *bucket {
                latency_stats.buckets[index] += 1;
            }
        }
    }
}

fn process_metrics() -> (u64, u64) {
    let mut threads = 0;
    let mut rss_bytes = 0;
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if let Some(value) = line.strip_prefix("Threads:") {
                threads = value.trim().parse().unwrap_or(0);
            }
            if let Some(value) = line.strip_prefix("VmRSS:") {
                rss_bytes = value
                    .split_whitespace()
                    .next()
                    .and_then(|kilobytes| kilobytes.parse::<u64>().ok())
                    .unwrap_or(0)
                    * 1024;
            }
        }
    }
    (threads, rss_bytes)
}

pub async fn observe(
    telemetry: State<Arc<Telemetry>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let id = uuid::Uuid::new_v4();
    let method = request.method().to_string();
    // MatchedPath is the Axum route template, so document IDs never become labels.
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|path| path.as_str().to_owned())
        .unwrap_or_else(|| "unmatched".to_string());
    let mut response = next.run(request).await;
    let elapsed = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();
    telemetry.record(&method, &route, status, elapsed);
    tracing::info!(request_id=%id,method=%method,route=%route,status,duration_ms=elapsed,"HTTP request completed");
    if let Ok(value) = axum::http::HeaderValue::from_str(&id.to_string()) {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}

pub async fn metrics(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<
    (
        StatusCode,
        [(axum::http::header::HeaderName, &'static str); 1],
        String,
    ),
    (StatusCode, axum::Json<serde_json::Value>),
> {
    auth.exigir_administrador()?;
    Ok((
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        state.telemetry.exposition(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use tower::ServiceExt;

    #[test]
    fn metricas_requieren_rol_administrador() {
        let vendedor = AuthUser {
            id: "usuario:vendedor".into(),
            nombre: "Vendedor".into(),
            rol: crate::domain::entities::usuario::Rol::Vendedor,
            puesto_id: "puesto:1".into(),
            sesion_hash: "hash".into(),
        };
        let error = vendedor.exigir_administrador().unwrap_err();
        assert_eq!(error.0, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn metricas_usan_patron_de_ruta_y_no_id_del_documento() {
        let telemetry = Arc::new(Telemetry::default());
        let app = Router::new()
            .route("/items/{id}", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(
                telemetry.clone(),
                observe,
            ));
        let request = Request::builder()
            .uri("/items/documento:privado")
            .body(Body::empty())
            .unwrap();
        let _ = app.oneshot(request).await.unwrap();

        let metrics = telemetry.exposition();
        assert!(metrics.contains("route=\"/items/{id}\""));
        assert!(!metrics.contains("documento:privado"));
        assert!(metrics.contains(
            "mercapos_http_request_duration_milliseconds_bucket{method=\"GET\",route=\"/items/{id}\",le=\"+Inf\"} 1"
        ));
        assert!(metrics.contains(
            "mercapos_http_request_duration_milliseconds_count{method=\"GET\",route=\"/items/{id}\"} 1"
        ));
        assert!(metrics.contains("mercapos_process_threads"));
        assert!(metrics.contains("mercapos_process_resident_memory_bytes"));
    }
}
