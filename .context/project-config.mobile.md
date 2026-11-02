# project-config.md — Cấu hình dự án Rust Mobile (canonical mẫu)

> Mẫu cho project con sinh từ opencode-rust-mobile (stack Dioxus mobile).
> `/brainstorm` ghi file thật `.context/project-config.md` — KHÔNG điền tay.
> `opencode-tools detect` nhận Dioxus (Dioxus.toml / dependency dioxus) và gợi ý sẵn giá trị dưới.

## Config

```yaml
# ── Git ──
default_branch: main
integration_prefix: layer

# ── Package / source ──
package_manager: cargo
source_roots: [crates/core, crates/ui, crates/app]   # tách core(logic)/ui(component)/app(bin)

# ── Rust / Mobile ──
rust_edition: "2021"
rust_toolchain: stable
workspace: true
ui_framework: dioxus        # full-Rust UI, multiplatform
async_runtime: tokio
http_client: reqwest        # feature rustls-tls (tránh openssl native)
platforms: [android, ios, desktop]   # bỏ ios nếu không build trên macOS

# ── Mobile targets (rustup target add trước khi bundle) ──
android_targets: [aarch64-linux-android, armv7-linux-androideabi]
ios_targets: [aarch64-apple-ios, aarch64-apple-ios-sim]
bundle_id: com.example.app   # đổi theo project

# ── Verify commands ──
install_command: cargo fetch
build_command: dx build --release      # dx = dioxus-cli
test_command: cargo test --workspace
lint_command: cargo clippy --workspace --all-targets -- -D warnings
typecheck_command: cargo check --workspace
fmt_command: cargo fmt --all --check
migration_command: null

# ── Database (nếu app cần local store) ──
db_tool: none               # none | sqlx(sqlite) | sled
migration_required: false

# ── Worktree hooks (optional — parallel loop) ──
worktree_setup_cmd: null
worktree_teardown_cmd: null
```

## Check commands (chạy trước khi báo xong)

Thứ tự chuẩn cho mobile:
1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo check --workspace` (typecheck nhanh)
4. `cargo test --workspace`
5. `dx build --release` (bundle — chỉ chạy khi toolchain platform sẵn sàng)

> iOS chỉ build được trên **macOS** + Xcode. CI iOS tách runner `macos-latest`.

## UI rules (Dioxus)

- Component = `fn -> Element`, props bất biến, state qua `use_signal` / context.
- Async qua `use_resource` / `spawn`; không block UI thread.
- Tách logic sang crate `core` (test được không cần renderer).
- Asset qua `asset!()` macro để bundle đúng mọi platform.

## Notes

- Convention đầy đủ: `skills/rust-mobile-guidelines/SKILL.md`.
- CI/CD mobile: `.agent/devops-mobile.md` (build .apk/.ipa, tách runner iOS).
- `opencode-tools perms` auto-allow lệnh cargo/dx an toàn vào reviewer.
