# Rust Monitoring — observability cho Axum + Leptos

Dùng khi thêm/review observability cho project con **Rust**. Các reference OTel khác trong folder này đúng về
**khái niệm** (trace/metric/log, semantic conventions); file này map sang **cách làm trong Rust**.

## 1. Logging / tracing (nền tảng)

- Dùng [`tracing`](https://crates.io/crates/tracing) + `tracing-subscriber` — chuẩn de-facto của hệ Rust.
  ```rust
  tracing_subscriber::fmt()
      .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()) // RUST_LOG=info
      .json()        // structured log cho production
      .init();
  ```
- Instrument handler: `#[tracing::instrument(skip(state))]` để tự tạo span + log tham số (bỏ field nhạy cảm).
- Không `println!` cho log production — dùng `tracing::{info,warn,error}` để đi vào pipeline.

## 2. OpenTelemetry (trace + metric)

- [`tracing-opentelemetry`](https://crates.io/crates/tracing-opentelemetry) + `opentelemetry-otlp` → export OTLP
  tới collector (xem `otel-collector.md`).
  ```rust
  let tracer = opentelemetry_otlp::new_pipeline()
      .tracing()
      .with_exporter(opentelemetry_otlp::new_exporter().tonic())
      .install_batch(opentelemetry_sdk::runtime::Tokio)?;
  ```
- Theo semantic conventions (`otel-semantic-conventions.md`): `http.method`, `http.route`, `http.status_code`,
  `service.name`.
- Metric: `metrics` crate + `metrics-exporter-prometheus` nếu scrape kiểu Prometheus.

## 3. Health / readiness endpoint (Axum) — BẮT BUỘC

CI monitoring gate kiểm sự tồn tại của endpoint này.

```rust
use axum::{routing::get, Json, Router};
use serde_json::json;

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))         // liveness: process sống
}
async fn ready(state: State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    state.db.ping().await?;                  // readiness: dependency (DB…) sẵn sàng
    Ok(Json(json!({ "status": "ready" })))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
}
```

- `/health` = liveness (process sống) · `/ready` = readiness (DB/dependency OK).
- Deploy dùng `/health` cho health check + auto-rollback (xem devops).

## 4. Request middleware (metric + trace tự động)

- `tower-http::trace::TraceLayer` → tự log/trace mỗi request (method, path, status, latency).
  ```rust
  let app = Router::new()
      .merge(routes())
      .layer(tower_http::trace::TraceLayer::new_for_http());
  ```

## 5. Frontend (Leptos) — RUM

- Xem `otel-browser.md` cho khái niệm. Leptos (WASM) có thể gửi web-vitals/error về một endpoint collect
  của backend; giữ nhẹ, đừng chèn SDK nặng vào bundle WASM.

## 6. Error tracking

- Sentry: [`sentry`](https://crates.io/crates/sentry) + `sentry-tower` layer cho Axum → capture panic + error.
- Hoặc tự log `error!` có `error.type` field rồi alert từ log pipeline.

## 7. Checklist

- [ ] `tracing` init với EnvFilter (`RUST_LOG`), JSON log ở production.
- [ ] `/health` (liveness) + `/ready` (readiness) tồn tại.
- [ ] `TraceLayer` gắn vào router → request có trace/latency.
- [ ] (Nếu cần) OTLP export tới collector theo semantic conventions.
- [ ] Deploy health check trỏ `/health` + auto-rollback khi fail.
