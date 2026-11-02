# DevOps — Rust Mobile (Dioxus)

Bổ sung cho `.agent/devops.md` khi project con là **Dioxus mobile** (`ui_framework: dioxus`). Thay build web
bằng `dx` CLI, tách runner iOS (chỉ macOS), giữ thứ tự: `git init` → remote → **generate CI** → commit → push.

## Phase 1.5: Generate CI (mobile)

Sinh `.github/workflows/ci.yml`:

```yaml
name: CI
on:
  push: { branches: [main, develop] }
  pull_request: { branches: [main] }
jobs:
  # ── lint/test/build chung (ubuntu) ──
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: clippy, rustfmt }
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo check --workspace
      - run: cargo test --workspace

  # ── security (ubuntu) ──
  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: rustsec/audit-check@v2
        with: { token: "${{ secrets.GITHUB_TOKEN }}" }
      - uses: EmbarkStudios/cargo-deny-action@v2
        with: { command: check }

  # ── build Android bundle (.apk/.aab) ──
  android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: aarch64-linux-android }
      - uses: nttld/setup-ndk@v1
        with: { ndk-version: r26d }
      - run: cargo install dioxus-cli
      - run: dx bundle --platform android --release

  # ── build iOS (.ipa) — CHỈ macOS ──
  ios:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: aarch64-apple-ios }
      - run: cargo install dioxus-cli
      - run: dx bundle --platform ios --release
        # signing: cần Apple cert + provisioning profile (secret) cho build release thật
```

Copy `deny.toml` từ template vào repo con.

## Build artifact

| Platform | Lệnh | Ra |
|----------|------|-----|
| Android | `dx bundle --platform android --release` | `.apk` / `.aab` |
| iOS | `dx bundle --platform ios --release` (macOS) | `.ipa` (cần signing) |
| Desktop | `dx bundle --platform desktop --release` | binary/installer theo OS |

## Phát hành

- **Android:** `.aab` → Google Play Console (internal testing → production). Ký bằng upload key.
- **iOS:** `.ipa` → App Store Connect / TestFlight. Cần Apple Developer account + signing.
- Secret (keystore, Apple cert, provisioning) để ở GitHub Actions secrets — **không** commit.

## Khác biệt so với web/Node

| Web (Node) | Mobile (Dioxus) |
|------------|-----------------|
| `npm run build` | `dx bundle --platform <p> --release` |
| deploy server/VPS | publish store (Play / App Store) |
| health check HTTP | crash reporting (sentry) + store vitals |
| 1 runner ubuntu | ubuntu (android/check) + macos (ios) |

## Lưu ý

- `rustup target add` các target mobile trước khi bundle (CI đã khai qua `with: targets:`).
- iOS **bắt buộc** runner macOS — đừng để job iOS trên ubuntu (link fail).
- Dùng `rustls` thay openssl để cross-compile mượt.
- Monitoring mobile: crash reporting thay health endpoint — xem `skills/monitoring/rust-mobile-monitoring` (nếu có) hoặc sentry crate.
