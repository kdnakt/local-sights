# Build Instructions — Logs Insights 互換のクエリ（IB-11）

上流：各作業単位の `code-generation-plan.md`・`unit-test-instructions.md`・`code-summary.md`（U1〜U4）、`.github/workflows/ci.yml`、team.md の Code Style と Testing Posture。

## 1. 必要なもの

| もの | 版 | 用途 |
|---|---|---|
| Rust（stable、edition 2024） | `rust-toolchain` の指定どおり | 中核ライブラリ・Tauri アプリ・確認用プログラム |
| Node.js と npm | CI と同じ LTS | 画面（React・Vite・Vitest） |
| cargo-llvm-cov | 0.9.1 | ライブラリの行カバレッジ |
| cargo-deny | CI の cargo-deny-action と同じ | 依存関係のチェック |
| Linux だけ：WebKitGTK・GTK の開発用パッケージ | CI の `tauri-app` ジョブと同じ | Tauri アプリのビルド |
| 手元の確認だけ：AWS CLI v2、`jq`、`git` | — | `scripts/compat/insights-export.sh`（U4）。CI では使わない |

AWS の認証情報は、ビルドにも自動テストにも要らない（project.md Forbidden）。

## 2. 依存関係を入れる

```bash
npm ci
cargo fetch
```

この作業で足した依存関係は、画面の `ace-builds`（U1、クエリの欄）だけ。Rust の依存関係は増えていない（`Cargo.toml` の変更は U4 の例の登録だけ）。

## 3. ビルド

```bash
# 画面（Tauri アプリの frontendDist）
npm run build          # = tsc + vite build

# 中核ライブラリと Tauri アプリ
cargo build -p local-sights

# 確認用プログラム（例）。cargo test でもコンパイルされる
cargo build -p local-sights-core --examples
```

## 4. ビルドの確かめ

CI と同じ順に流す。どれか 1 つでも失敗したらマージしない（team.md）。

```bash
npm run check:version              # 版のそろい（Cargo.toml・Cargo.lock・package.json・tauri.conf.json）
cargo fmt --all --check
cargo clippy --workspace --all-targets   # 警告はエラーにしない
npx tsc --noEmit
npx eslint .
npx prettier --check .
npx vite build
npm audit --audit-level=high
cargo deny check
```

## 5. よくあるつまずき

- **`cargo fmt --check` が通らない**：rustfmt の 100 桁を超えた行がある。`cargo fmt -p local-sights-core` で直す（U4 で U3 の計測のテストを直した例がある）。
- **Linux で Tauri アプリがビルドできない**：WebKitGTK・GTK の開発用パッケージがない。CI の `tauri-app` ジョブの手順で入れる。
- **`aidlc` などのツールをサブフォルダから動かすと失敗する**：リポジトリの最上位で動かす。
- **確認用プログラムのテストが `cargo test --workspace` で流れない**：Cargo は例をコンパイルするだけ。例のテストは `cargo test -p local-sights-core --examples` で流す（CI もこの形）。
