# opencode-rust-mobile

Phiên bản **Rust** của template dự án opencode. Nó giữ **nguyên vẹn toàn bộ luồng điều phối AI**
của `opencode-project-template` và chỉ đổi phần **machine tooling** + **stack mặc định của project con**
từ Node/React sang **Rust mobile (Dioxus)** — UI full-Rust, một codebase build ra **iOS · Android · desktop**.

> Trạng thái: **hoàn tất Pha 1–4.** Pha 5 (tùy chọn) còn để ngỏ — xem [Roadmap](#roadmap).

---

## Mục lục

- [Ý tưởng: hai tầng](#ý-tưởng-hai-tầng)
- [Cài đặt & build](#cài-đặt--build)
- [`opencode-tools` — 6 lệnh con](#opencode-tools--6-lệnh-con)
- [Luồng điều phối AI (`/start`)](#luồng-điều-phối-ai-start)
- [Dùng làm template cho dự án mới](#dùng-làm-template-cho-dự-án-mới)
- [Stack mặc định của project con](#stack-mặc-định-của-project-con)
- [CI/CD · Security · Monitoring](#cicd--security--monitoring)
- [Phát triển & kiểm thử](#phát-triển--kiểm-thử)
- [Cấu trúc thư mục](#cấu-trúc-thư-mục)
- [Quyết định thiết kế](#quyết-định-thiết-kế)
- [Roadmap](#roadmap)

---

## Ý tưởng: hai tầng

Template gồm hai tầng tách biệt. "Chuyển sang Rust" chỉ đụng tầng thứ hai.

| Tầng | Là gì | Đổi sang Rust? |
|------|-------|----------------|
| **A — Điều phối AI** | `AGENTS.md`, `.opencode/agent/*.md`, `.opencode/command/*.md`, `.agent/*.md`, `skills/`, `spec/`, `opencode.jsonc`. Toàn Markdown/config, định tuyến ý định và dẫn dắt agent. | **Không.** Copy nguyên. Luồng `/start → brainstorm → design → graph → loop`, các human checkpoint và mô hình worktree-per-task giữ y hệt. |
| **B — Machine tooling** | Các script thực thi trên máy. Trước là Node `.mjs`. | **Có.** Port thành một binary Rust duy nhất `opencode-tools`. |

Tầng A gọi tầng B qua **CLI + file I/O**. Port B sang Rust mà giữ nguyên hợp đồng đó → cả luồng vẫn chạy.

---

## Cài đặt & build

Cần [Rust](https://rustup.rs) (toolchain `stable`, kèm `clippy` + `rustfmt` — đã khai trong `rust-toolchain.toml`).

```sh
# clone
git clone git@github.com:ocanhdt12-gif/opencode-rust-mobile.git
cd opencode-rust-mobile

# build binary tooling
cargo build --release

# binary nằm ở target/release/opencode-tools
./target/release/opencode-tools --help
```

Cho tiện gọi từ bất kỳ đâu, thêm binary vào PATH:

```sh
# cách 1: cài vào ~/.cargo/bin
cargo install --path crates/tools

# cách 2: symlink
ln -s "$PWD/target/release/opencode-tools" ~/.local/bin/opencode-tools
```

---

## `opencode-tools` — 6 lệnh con

Một binary đa lệnh thay cho các script `.mjs` cũ. Mỗi lệnh giữ **đúng CLI contract** (flag, stdout, file sinh ra)
của script nó thay thế, nên tầng A gọi như cũ mà không cần biết đã đổi ngôn ngữ.

```
opencode-tools <command> [options]
```

| Lệnh | Thay cho | Vai trò |
|------|----------|---------|
| `detect` | `detect-profile.mjs` | Dò stack/profile dự án → JSON cho `/brainstorm` |
| `inventory` | `generate-inventory.mjs` | Sinh `docs/generated/inventory.md` |
| `perms` | `apply-verify-permissions.mjs` | Đồng bộ allow-rule của reviewer với verify command |
| `sync` | `template-sync.mjs` | Kéo managed paths mới từ template canonical |
| `dispatch` | `parallel-dispatch.mjs` | Chạy task song song qua git worktree-per-task |
| `live` | `live-studio.mjs` | Hook vibe-studio: event enter/leave nhân vật |

### `detect` — dò profile dự án

Chỉ đọc cây repo (không ghi gì), in gợi ý cho `.context/project-config.md`. Nhận cả Node (npm/pnpm/yarn/bun,
prisma/drizzle) **và** Rust (Cargo.toml → `package_manager: cargo`, verify = cargo).

```sh
opencode-tools detect          # JSON (mặc định)
opencode-tools detect --md     # Markdown dễ đọc
```

### `inventory` — kiểm kê file

Đọc `source_roots` từ `.context/project-config.md`, sinh `docs/generated/inventory.md` (bảng theo đuôi file +
danh sách theo thư mục + git HEAD). Bỏ qua `node_modules`, `target`, `dist`, … Chạy lại bao nhiêu lần cũng ra kết quả nhất quán.

```sh
opencode-tools inventory
```

### `perms` — đồng bộ allow-rule verify

Chỉ sửa block giữa marker `# verify-commands:start` / `# verify-commands:end` trong
`.opencode/agent/reviewer.md` và `spec-validator.md`. **Mặc định dry-run**; thêm `--write` để ghi thật.
Chỉ auto-allow lệnh an toàn (không shell-meta, không git/db phá hoại, không `rm`/`--force`); pattern là **exact**.

```sh
opencode-tools perms            # dry-run (xem trước)
opencode-tools perms --write    # ghi thật
```

### `sync` — đồng bộ với template canonical

Chỉ ghi đè **managed paths** (`AGENTS.md`, `.agent/`, `.opencode/`, `scripts/`, `skills/`). Không đụng code,
spec, docs, `.env`, `package.json`… Nếu managed paths đang có thay đổi **chưa commit** → dừng, không ghi đè.

```sh
opencode-tools sync                       # đồng bộ dự án hiện tại
opencode-tools sync --project /path/proj  # chỉ định dự án đích
opencode-tools sync --dry-run             # chỉ xem, không ghi
opencode-tools sync --prune               # xoá file template đã bỏ (trong managed paths)
opencode-tools sync --template <git-url>  # đổi nguồn template
opencode-tools sync --no-config           # bỏ qua .template-sync.json
```

### `dispatch` — chạy task song song (worktree-per-task)

Lo phần git/worktree + kiểm tra độc lập + merge an toàn cho loop song song. **Không tự spawn agent**
(việc đó do `.agent/loop.md` điều phối).

```sh
opencode-tools dispatch plan --layer 1                 # chia wave (task trùng file → khác wave)
opencode-tools dispatch setup --task layer-1/task-03   # tạo worktree + branch cho 1 task
opencode-tools dispatch merge --layer 1                # gom branch task về integration branch
opencode-tools dispatch merge --layer 1 --no-verify    # bỏ gợi ý verify
opencode-tools dispatch cleanup --layer 1              # xoá worktree + branch tạm
opencode-tools dispatch status                         # liệt kê worktree + branch wt/*
```

Chế độ **change** (feature/bug) thay `--layer N` bằng `--scope <feature|bug>-<slug> --phase N`.

### `live` — hook vibe-studio

Map event vòng đời opencode → action enter/leave nhân vật, in ra `<action> role-<agentId>`.

```sh
opencode-tools live UserPromptSubmit --agent builder    # → chat-enter role-builder
opencode-tools live PreToolUse --tool Agent --agent reviewer  # → agent-enter role-reviewer
opencode-tools live Stop                                # → chat-leave role-default
```

Mapping: `UserPromptSubmit→chat-enter` · `Stop→chat-leave` · `PreToolUse(Agent|SendMessage)→agent-enter` ·
`PostToolUse(Agent)→agent-leave` · `SubagentStop→agent-leave`. Event không khớp → no-op.

---

## Luồng điều phối AI (`/start`)

`/start` là **một** cửa vào để dựng dự án lần đầu. Cả chuỗi tự chạy, chỉ **dừng ở human checkpoint**;
user không phải gõ lại từng lệnh — reply `ok` là chạy tiếp.

```
1. ĐỌC SPEC
   ├─ có SPECIFICATIONS.md → dùng luôn
   └─ chưa có              → /spec-init (reverse-engineer từ code)
   → subagent spec-validator cross-check spec vs code/docs (max 2 vòng)

2. BRAINSTORM            (.agent/brainstorm.md)
   → làm rõ yêu cầu → design doc + chốt .context/project-config.md
     (dùng `opencode-tools detect` để gợi ý profile)
   ⏸ CHECKPOINT: user approve design

3. DESIGN  (nếu có UI)   (.agent/design.md)
   → design tokens + screen specs + diagram
   ⏸ CHECKPOINT: confirm design tokens

4. GRAPH                 (.agent/graph.md)
   → chia layer + task → tasks/<slug>/layer-{N}-task-{NN}.md + layer-plan diagram
   ⏸ CHECKPOINT: duyệt layer plan

5. LOOP                  (.agent/loop.md)
   → `opencode-tools dispatch plan --layer N` chia wave
   → mỗi task 1 git worktree → spawn builder song song (full loop)
       PASS → reviewer review độc lập → commit trong worktree
       FAIL → error-analyzer → fix → retry (max 3)
   → cả layer xong → dispatcher merge → verify tổng thể → cleanup
   → DevOps lo git/CI-CD/deploy; layer cuối → spec-publisher cập nhật spec/CHANGELOG
   ⏸ checkpoint sau mỗi layer (layer N+1 chỉ mở khi layer N PASS + user approve)
```

**HARD-GATE:** không code trước khi design approve; không chạy layer N+1 trước khi layer N PASS + approve.

Sau khi dự án build xong, mọi thay đổi đi qua `/change` (agent `change-request`) — **không** chạy lại `/start`.

### Lệnh slash có sẵn (`.opencode/command/`)

| Lệnh | Dùng khi |
|------|----------|
| `/start` | Dựng dự án lần đầu (chạy cả chuỗi) |
| `/spec-init` · `/spec-publish` | Khởi tạo / xuất bản spec |
| `/brainstorm` · `/design` · `/graph` | Chạy tay từng bước (manual override) |
| `/feature` · `/change` | Thêm/sửa/bỏ tính năng qua Change Request |
| `/bug` · `/bug-check` | Điều tra & sửa bug / soi read-only |
| `/resume` | Tiếp tục việc dở từ Run Journal |
| `/dashboard` | Mở Vibe Code Studio (dashboard trực quan) |

---

## Dùng làm template cho dự án mới

```sh
# 1. tạo dự án mới từ template (copy, bỏ .git để init repo riêng)
cp -r opencode-rust-mobile my-app && cd my-app
rm -rf .git target && git init

# 2. build tooling
cargo build --release && cargo install --path crates/tools   # hoặc thêm vào PATH

# 3. mở trong opencode và chạy
/start
```

Khi template canonical cập nhật, kéo phần managed paths về bằng:

```sh
opencode-tools sync --dry-run    # xem trước
opencode-tools sync              # áp dụng (chỉ đụng AGENTS.md/.agent//.opencode//scripts//skills/)
```

Nguồn sync + managed paths khai ở [`.template-sync.json`](.template-sync.json) (đã trỏ sẵn về `opencode-rust-mobile`).

```

---

## Stack mặc định của project con (Dioxus mobile)

Project sinh từ template này mặc định dùng Rust:

- **UI multiplatform:** [Dioxus](https://dioxuslabs.com) — viết 1 lần ra iOS/Android/desktop/web, RSX giống Leptos.
- **Build:** `dx` CLI (`dx serve` dev · `dx bundle --platform android|ios --release`).
- **Async:** tokio. **Error:** `thiserror` (library) + `anyhow` (binary).
- **Verify mặc định:** `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings`
  → `cargo build --workspace --release` → `cargo test --workspace`.
- Convention đầy đủ: [`skills/rust-mobile-guidelines/SKILL.md`](skills/rust-mobile-guidelines/SKILL.md).
- Mẫu cấu hình: [`.context/project-config.mobile.md`](.context/project-config.mobile.md).

---

## CI/CD · Security · Monitoring

### CI (repo này)

`.github/workflows/ci.yml` có **2 job**, chạy mỗi push/PR:

| Job | Bước |
|-----|------|
| `check` | `cargo fmt --check` → `clippy -D warnings` → `build --release` → `test` → monitoring gate (tìm health endpoint) |
| `security` | `cargo audit` (RustSec advisory) + `cargo deny check` (license/duplicate/nguồn crate) |

### Security (chuẩn hệ Rust)

- **`cargo audit`** — quét lỗ hổng crate theo RustSec advisory DB (tương đương `npm audit`).
- **`cargo deny`** — gate license + duplicate version + nguồn crate; cấu hình ở [`deny.toml`](deny.toml).
  ```sh
  cargo install cargo-audit cargo-deny
  cargo audit && cargo deny check
  ```
- Hướng dẫn đầy đủ cho project con (auth/JWT Axum, BOLA/IDOR, secrets, input hardening):
  [`skills/security/rust-security.md`](skills/security/rust-security.md).

### Monitoring (observability)

- `tracing` + OpenTelemetry (OTLP export), health/readiness endpoint Axum, `TraceLayer`, error tracking.
- Hướng dẫn: [`skills/monitoring/rust-monitoring.md`](skills/monitoring/rust-monitoring.md).
- **Monitoring gate** trong CI: project con có web **bắt buộc** có `GET /health` (liveness) + `/ready` (readiness).

### CI/CD cho project con

Khi dựng dự án, DevOps agent dùng [`.agent/devops-rust.md`](.agent/devops-rust.md) để sinh:
- workflow GitHub Actions Rust (check + security + monitoring gate),
- Dockerfile multi-stage (distroless) hoặc binary + systemd cho VPS,
- health check deploy + auto-rollback.

---

## Phát triển & kiểm thử

```sh
cargo fmt --all --check                                   # format
cargo clippy --workspace --all-targets -- -D warnings     # lint (deny warnings)
cargo build --workspace --release                         # build
cargo test --workspace                                    # test (34 golden test)
```

**Golden test** so output của mỗi tool với kỳ vọng từ bản `.mjs` cũ:

| Suite | Số test |
|-------|---------|
| `detect_golden` | 6 |
| `inventory_golden` | 5 |
| `perms_golden` | 8 |
| `sync_golden` | 5 |
| `dispatch_golden` | 6 |
| `live_golden` | 4 |

CI (`.github/workflows/ci.yml`) chạy cả 4 bước trên mỗi push/PR.

Logic thuần (parse, phân loại, chia wave) tách khỏi I/O nên test không cần mạng/git; phần git/rsync là shell-out.

---

## Cấu trúc thư mục

```
opencode-rust-mobile/
├── Cargo.toml                     # workspace
├── rust-toolchain.toml            # stable + clippy + rustfmt
├── crates/
│   ├── core/                      # lib: config · detect · inventory · perms · sync · dispatch · live · git · fsx · log
│   │   └── tests/                 # golden test (34)
│   └── tools/                     # bin opencode-tools (6 subcommand)
├── .github/workflows/ci.yml       # fmt + clippy -D warnings + build + test
├── docs/ARCHITECTURE.md           # kiến trúc chi tiết
│
│   # ── Tầng A (điều phối AI, copy nguyên từ template gốc) ──
├── AGENTS.md                      # router
├── opencode.jsonc
├── .opencode/{agent,command}/     # 9 agent + 12 command
├── .agent/                        # workflow prompt-level (loop, brainstorm, graph, devops…)
├── skills/                        # skill (gồm rust-guidelines)
├── spec/  ·  tasks/  ·  .devops/  ·  .claude/
└── .context/
    ├── project-config.rust.md     # mẫu cấu hình Rust stack
    └── …
```

---

## Quyết định thiết kế

1. **vibe-studio giữ TypeScript** — UI trình duyệt, không ép sang Rust ở Pha 1. Chỉ port hook `live`.
2. **Web stack mặc định = full-stack Rust** — Axum + Leptos.
3. **Tooling = một binary đa lệnh** `opencode-tools <subcommand>`.
4. **Git = shell-out** tới binary `git` (không libgit2), khớp hành vi bản cũ.
5. **Tên template:** `opencode-rust-mobile`.

---

## Roadmap

- [x] **Pha 1** — Cargo workspace, `core` lib, `opencode-tools` (6 subcommand), CI, docs.
- [x] **Pha 2** — Port thật 6 tool, mỗi tool có golden test (34 test pass).
- [x] **Pha 3** — Rust-hóa stack project con: `detect` nhận `Cargo.toml`; verify mặc định `cargo …`;
  `skills/rust-guidelines/`; `.context/project-config.rust.md`.
- [x] **Pha 4** — Copy tầng A, rewire 9 file gọi tooling (`node scripts/*.mjs` → `opencode-tools`), smoke test.
- [ ] **Mở rộng** — thêm native API binding (camera/GPS) qua `jni`/`objc2` khi project cần.
```
