# Rust Security — map OWASP/auth sang hệ Rust

Dùng khi viết/review phần bảo mật cho project con **Rust** (Axum + Leptos). Các reference khác trong
folder này (OWASP API, BOLA/IDOR, JWT) vẫn đúng về **nguyên tắc**; file này map sang **cách làm trong Rust**.

## 1. Supply-chain (bắt buộc trong CI)

- **`cargo audit`** — quét lỗ hổng crate theo RustSec advisory DB (tương đương `npm audit`).
  ```sh
  cargo install cargo-audit
  cargo audit                      # fail nếu có advisory
  cargo audit --deny warnings      # strict cho CI
  ```
- **`cargo deny`** — gate license + duplicate + nguồn crate (xem `deny.toml` ở repo root).
  ```sh
  cargo install cargo-deny
  cargo deny check                 # advisories + licenses + bans + sources
  ```
- Commit `Cargo.lock` cho binary → build reproducible, audit đúng version thật.
- `wildcards = "deny"` trong deny.toml: cấm version `"*"` trong Cargo.toml.

## 2. Secrets

- Không hardcode. Đọc từ env qua `std::env::var` hoặc crate `config`/`dotenvy` (chỉ load `.env.local` ở dev).
- Bọc giá trị nhạy cảm bằng [`secrecy`](https://crates.io/crates/secrecy) (`Secret<String>`) để tránh lộ qua `Debug`/log.
- `.env*` phải nằm trong `.gitignore` (template đã có). Secret CI để ở GitHub Actions secrets.
- Không log token/password. Nếu dùng `tracing`, redact field nhạy cảm.

## 3. Auth / JWT (Axum)

- JWT: dùng [`jsonwebtoken`](https://crates.io/crates/jsonwebtoken). Luôn verify `exp`, `aud`, `iss`; chọn
  thuật toán cố định (HS256/RS256) — **không** để `alg` từ token quyết định (tránh alg-confusion).
- Hash password: [`argon2`](https://crates.io/crates/argon2) (hoặc `bcrypt`), không bao giờ tự hash SHA.
- Auth middleware: `axum::middleware::from_fn` hoặc extractor custom — verify token **trước** handler.
  ```rust
  async fn require_auth(req: Request, next: Next) -> Result<Response, AppError> {
      let claims = verify_bearer(req.headers())?;   // ? → 401 qua AppError
      // gắn claims vào request extensions cho handler dùng
      Ok(next.run(req).await)
  }
  ```

## 4. BOLA / IDOR (authorization)

- Nguyên tắc (xem `bola-idor.md`) không đổi: **mọi** truy cập resource phải kiểm chủ sở hữu, không tin ID từ client.
- Trong handler: lấy `user_id` từ **claims đã verify**, không từ path/body. Query kèm điều kiện `owner_id = $claims_user`.
- Đừng expose primary key tuần tự ra ngoài nếu không cần; cân nhắc UUID.

## 5. Input / API hardening (Axum)

- Validate input: [`validator`](https://crates.io/crates/validator) + struct DTO; `serde` deny unknown fields
  (`#[serde(deny_unknown_fields)]`) cho endpoint nhạy cảm.
- Giới hạn body: `axum::extract::DefaultBodyLimit`.
- CORS chặt: `tower-http::cors` — allow origin cụ thể, không `*` cho endpoint có cookie/auth.
- Security headers: `tower-http::set_header` (HSTS, X-Content-Type-Options, CSP cho Leptos SSR).
- Rate limit: `tower_governor` hoặc reverse-proxy (nginx/traefik).
- SQL: dùng `sqlx` query macro (compile-time checked) hoặc parameterized — **không** nối chuỗi.

## 6. Error → không rò rỉ

- `AppError` impl `IntoResponse`: map lỗi nội bộ sang status code + message chung; **không** trả stack/SQL ra client.
- Log chi tiết phía server (tracing), trả client message tối thiểu.

## 7. Checklist review diff Rust

- [ ] Không `unwrap()`/`expect()`/`panic!` trên path xử lý request.
- [ ] Token verify đủ exp/aud/iss + alg cố định.
- [ ] Mọi resource access check ownership từ claims, không từ client ID.
- [ ] Input validate + body limit + CORS cụ thể.
- [ ] Không secret trong code/log; `.env*` ignored.
- [ ] `cargo audit` + `cargo deny check` sạch.
