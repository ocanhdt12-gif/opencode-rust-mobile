---
name: rust-mobile-guidelines
description: "Convention code mobile Rust (Dioxus) cho project con sinh từ opencode-rust-mobile. Dùng trong Phase 4 Loop khi viết/sửa UI Dioxus + Phase 5 Review. Gồm: layout multiplatform, component/signal, platform iOS/Android (dx CLI, Xcode, Android SDK), native API qua crate, async, state, test, và build .ipa/.apk."
---

# Rust Mobile Guidelines — Dioxus (full-Rust UI)

Project sinh từ `opencode-rust-mobile` mặc định dùng **Dioxus** — UI viết 100% Rust, một codebase
build ra **iOS · Android · desktop · web**. Skill này giữ convention để code nhất quán, dễ review.

> Stack: Dioxus 0.6+ · `dx` CLI (dioxus-cli) để serve/build/bundle · tokio cho async · cùng hệ error
> `thiserror`/`anyhow` như bản web. Cú pháp component giống Leptos (RSX) nên quen tay nhanh.

## 1. Layout

- **Cargo workspace** kể cả 1 crate.
  ```
  Cargo.toml            # [workspace] + dioxus ở [workspace.dependencies]
  Dioxus.toml           # cấu hình dx: app name, bundle id, platform
  crates/
    ui/                 # component Dioxus (RSX) — nền tảng chung
    core/               # logic thuần (model, api client, state) — KHÔNG phụ thuộc UI, dễ test
    app/                # bin: khởi tạo dioxus::launch cho từng platform
  assets/               # icon, font, ảnh (dx bundle)
  ```
- Tách **core** (logic) khỏi **ui** (component). Core test được không cần renderer.
- Version chung ở `[workspace.dependencies]`, crate con `dioxus.workspace = true`.

## 2. Component & RSX

- Component = `fn` trả `Element`, nhận props qua `#[component]`.
  ```rust
  #[component]
  fn Counter(start: i32) -> Element {
      let mut count = use_signal(|| start);
      rsx! {
          button { onclick: move |_| count += 1, "Tapped {count}" }
      }
  }
  ```
- Props bất biến; state cục bộ qua `use_signal`. Tên component PascalCase, một component một file khi lớn.
- Không nhồi logic nặng vào RSX — tính toán trước, RSX chỉ render.

## 3. State

- **Cục bộ:** `use_signal`. **Chia sẻ cây con:** `use_context_provider` / `use_context`.
- **Global app state:** một struct trong `core`, provide ở root qua context. Tránh global mutable tùy tiện.
- Side-effect/async: `use_resource` (fetch theo dependency) hoặc `spawn` cho fire-and-forget.
- Derived state: `use_memo` thay vì tính lại mỗi render.

## 4. Async & networking

- Runtime tokio (dioxus desktop/mobile có sẵn). HTTP: `reqwest` (feature `rustls-tls` cho mobile, tránh openssl native).
- Gọi API trong `use_resource`:
  ```rust
  let data = use_resource(move || async move { api::fetch_items().await });
  ```
- Không block UI thread; việc nặng → `tokio::task::spawn_blocking`.
- Đặt API client trong `core`, trả `Result<_, CoreError>` (thiserror) — UI map sang trạng thái hiển thị.

## 5. Platform (iOS / Android)

Dùng `dx` CLI (cài: `cargo install dioxus-cli`).

```sh
dx serve --platform desktop        # dev nhanh trên desktop
dx serve --platform android        # chạy trên emulator/thiết bị Android
dx serve --platform ios            # chạy trên simulator iOS (cần macOS + Xcode)
dx bundle --platform android       # ra .apk/.aab
dx bundle --platform ios           # ra .ipa (cần signing)
```

**Yêu cầu toolchain (ghi rõ cho dev biết):**
- **Android:** Android SDK + NDK, biến môi trường `ANDROID_HOME`/`ANDROID_NDK_HOME`, target `aarch64-linux-android` (`rustup target add`).
- **iOS:** chỉ build được trên **macOS** + Xcode, target `aarch64-apple-ios` (+ `aarch64-apple-ios-sim` cho simulator), cần Apple signing để ra `.ipa` thật.

## 6. Native API (camera, GPS, storage…)

- Ưu tiên crate cross-platform sẵn: `rfd` (file dialog), `keyring` (secure storage), `tokio`/`reqwest` (net).
- API đặc thù chưa có crate → viết qua FFI: `jni` (Android) / `objc2` (iOS), bọc trong `core` sau một trait
  để UI không phụ thuộc platform. Giữ `#[cfg(target_os = "...")]` ở một chỗ.
- Permission (camera/location): khai trong `Dioxus.toml` / manifest tương ứng; kiểm tra runtime trước khi dùng.

## 7. Style & an toàn

- `cargo fmt` + `cargo clippy --all-targets -- -D warnings` sạch.
- Không `unwrap()`/`expect()` trên luồng UI thật (panic = crash app). Lỗi → trạng thái hiển thị.
- Asset (ảnh/font) qua `asset!()` macro để `dx` bundle đúng đường dẫn mọi platform.
- Giữ bundle nhẹ: dùng `rustls` thay openssl, bật `lto`/`strip` ở release profile.

## 8. Test

- **Unit (core):** `#[cfg(test)] mod tests` — test logic/model/api-client (mock HTTP) không cần renderer.
- **Integration:** `tests/*.rs` ở crate `core`.
- Component logic: tách phần tính toán ra hàm pure để test; render test còn non trên mobile nên đừng phụ thuộc.
- Chạy: `cargo test --workspace`.

## 9. Verify commands (đưa vào project-config)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace                 # typecheck nhanh
cargo test --workspace
dx build --release                      # build bundle (platform mặc định)
```

## 10. Pitfalls

- Quên `rustup target add` cho target mobile → link fail. Thêm target trước khi `dx bundle`.
- Dùng openssl native → khó cross-compile; luôn `rustls`.
- Block async trong component → UI đơ. Dùng `use_resource`/`spawn`.
- iOS build trên Linux → không được; cần macOS. CI nên tách runner `macos-latest` cho job iOS.
- State global mutable lung tung → render thừa; dùng signal/context + `use_memo`.
