---
name: rust-guidelines
description: "Convention code Rust cho project con sinh từ opencode-rust-template. Dùng trong Phase 4 Loop khi viết/sửa code Rust và Phase 5 Review khi review diff Rust. Bao gồm: layout workspace, error handling (thiserror/anyhow), async (tokio), web fullstack Axum + Leptos, test layout, và verify commands chuẩn (cargo fmt/clippy/test)."
---

# Rust Guidelines — Convention cho project con (Rust)

> 📱 Project **mobile** (Dioxus): xem thêm `skills/rust-mobile-guidelines/SKILL.md` cho UI/platform/build .apk/.ipa.

Project sinh từ `opencode-rust-template` mặc định dùng Rust. Skill này giữ các quy ước để code Rust nhất quán, an toàn, dễ review. Áp dụng khi **viết/sửa** (Phase 4 Loop) và **review** (Phase 5).

> Stack mặc định cho web: **Axum** (backend) + **Leptos** (frontend WASM). Tooling template: **shell-out git**, 1 binary `opencode-tools`.

## 1. Layout

- **Cargo workspace** ngay cả khi mới 1 crate — dễ tách sau.
  ```
  Cargo.toml            # [workspace] members, [workspace.dependencies]
  crates/
    core/               # domain + logic thuần, không I/O nặng → dễ test
    <bin>/              # binary(s): axum server, cli…
    web/                # (nếu có UI) Leptos app
  ```
- Dependency version khai ở `[workspace.dependencies]`, crate con dùng `foo.workspace = true`. Một nguồn version duy nhất.
- Module theo tính năng, không theo loại (`user/`, không `models/ handlers/`). Mỗi file một trách nhiệm rõ.

## 2. Error handling

- **Library crate** (`core/`): dùng `thiserror` — định nghĩa enum lỗi cụ thể, cho caller match được.
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum StoreError {
      #[error("not found: {0}")]
      NotFound(String),
      #[error(transparent)]
      Io(#[from] std::io::Error),
  }
  ```
- **Binary / app** (`main`, handler): dùng `anyhow::Result` + `.context("…")` để thêm ngữ cảnh.
- **Không** `unwrap()`/`expect()` trên đường chạy thật. Chỉ chấp nhận trong test, hoặc khi invariant được chứng minh (kèm comment lý do).
- **Không** `panic!` để xử lý lỗi dự kiến — trả `Result`.

## 3. Async (tokio)

- Runtime: `#[tokio::main]` cho binary; `#[tokio::test]` cho test async.
- Không block trong async: dùng `tokio::fs`, `tokio::time::sleep`; việc CPU nặng → `tokio::task::spawn_blocking`.
- Giữ `Send + 'static` cho future spawn qua `tokio::spawn`.
- Timeout cho I/O mạng: `tokio::time::timeout`.

## 4. Web — Axum + Leptos

- **Axum**: router mảnh, handler nhận `State`/extractor, trả `impl IntoResponse`. State chia sẻ bọc trong `axum::extract::State<Arc<…>>`.
- Lỗi handler: một `AppError` impl `IntoResponse` (map sang status code) — đừng để `?` làm panic.
- **Leptos**: ưu tiên **SSR + hydrate** cho trang cần SEO/tốc độ tải; component thuần, signal cho state. Server function (`#[server]`) cho call backend thay vì tự viết fetch.
- Chia `web` (Leptos) và `api`/`server` (Axum) rõ; chung type qua crate `core` (dùng `serde` cho DTO).

## 5. Style & an toàn

- `cargo fmt` là chuẩn format duy nhất — không tranh luận tay.
- `cargo clippy -- -D warnings` phải sạch. Sửa gốc; chỉ `#[allow(...)]` kèm comment lý do khi clippy sai thật (vd so sánh phức hợp).
- Public API có doc-comment `///` nêu mục đích + invariant.
- Tránh `unsafe`. Nếu buộc phải dùng → block nhỏ nhất + comment `// SAFETY:` chứng minh.
- `clone()` khi cần cho rõ ràng; đừng tối ưu sớm bằng lifetime rối.

## 6. Test layout

- **Unit test**: `#[cfg(test)] mod tests` cuối file, cạnh code nó test.
- **Integration/golden test**: `tests/<ten>.rs` ở crate root — test qua public API, giống cách tool này test port (so output với kỳ vọng).
- Test thuần (không I/O) ưu tiên: tách logic ra hàm pure để test không cần mạng/DB.
- Chạy: `cargo test --workspace`. Thêm test cho mỗi bug fix trước khi sửa (RED→GREEN).

## 7. Verify commands (chuẩn, đưa vào project-config)

Chạy trước khi báo xong, theo thứ tự:

```
cargo fmt --all --check          # format
cargo clippy --workspace --all-targets -- -D warnings   # lint, deny warnings
cargo build --workspace --release   # build
cargo test --workspace           # test
```

Khai trong `.context/project-config.md`:
```
package_manager: cargo
source_roots: [src]              # hoặc [crates/core, crates/server, crates/web]
test_command: cargo test --workspace
lint_command: cargo clippy --workspace --all-targets -- -D warnings
typecheck_command: cargo check --workspace
build_command: cargo build --workspace --release
install_command: cargo fetch
```

## 8. Pitfalls hay gặp (ghi lại để khỏi lặp)

- Quên `--workspace` → chỉ build/test crate hiện tại, miss lỗi crate khác.
- `clippy` fail CI vì warning nhỏ → chạy `cargo clippy --fix` local trước khi commit.
- Edit `Cargo.lock` tay → đừng; để cargo quản. Commit `Cargo.lock` cho binary (reproducible), cân nhắc bỏ cho library.
- Block async bằng `std::fs`/`std::thread::sleep` → treo runtime. Dùng bản `tokio::`.
