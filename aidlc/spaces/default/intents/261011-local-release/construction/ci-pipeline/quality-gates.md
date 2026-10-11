# Quality Gates — リリース準備

## マージ前（`ci.yml`、PR と `main` への push）

どれか 1 つでも失敗したらマージしない（team.md：CI が赤のままマージはしない）。

| 門 | コマンド | ジョブ | 合格の条件 |
|----|----------|--------|------------|
| Rust の整形 | `cargo fmt --all --check` | `rust-core` | 差分なし |
| Rust の静的検査 | `cargo clippy -p local-sights-core --all-targets`、`cargo clippy --workspace --all-targets` | `rust-core`、`tauri-app` | deny レベルの指摘なし（警告はエラーにしない） |
| Rust のテスト | `cargo test -p local-sights-core` | `rust-core` | すべて成功（Build and Test では 346/346） |
| 版の検査 | `npm run check:version` | `frontend` | 4 つのファイルの版が一致 |
| 型検査 | `npx tsc --noEmit` | `frontend` | エラーなし |
| ESLint | `npx eslint .` | `frontend` | エラーなし |
| Prettier | `npx prettier --check .` | `frontend` | 差分なし |
| 画面側のテスト | `npx vitest run` | `frontend` | すべて成功（Build and Test では 195/195） |
| 画面側のバンドル | `npx vite build` | `frontend` | 成功 |
| npm の依存関係 | `npm audit --audit-level=high` | `frontend` | high 以上 0 件 |
| Rust の依存関係 | `cargo-deny check` | `cargo-deny` | 脆弱性・ライセンス・取得元の違反なし |
| アプリ本体のビルド | `npm run build`、`cargo build -p local-sights` | `tauri-app`（ubuntu・macOS） | 成功 |

## 公開の前（`release.yml`、タグの push）

前の門がすべて成功したときだけ次に進む。どこかで失敗したら Release は作らない。

| 門 | 内容 | ジョブ |
|----|------|--------|
| タグと版 | `npm run check:version -- --tag "$TAG"`：タグが `vMAJOR.MINOR.PATCH` で、4 つのファイルの版と一致 | `verify` |
| `main` 上のコミット | `git merge-base --is-ancestor` でタグのコミットが `origin/main` に含まれる | `verify` |
| テスト | `cargo test -p local-sights-core`、`npx vitest run` | `test` |
| ランナー | `uname -m` が `arm64` | `build-and-publish` |
| 成果物 | `target/release/bundle/dmg/` に `aarch64` の `.dmg` がちょうど 1 つ | `build-and-publish` |

## 任意の確認（`release-dry-run.yml`、手動）

| 門 | 内容 |
|----|------|
| 版の検査 | `npm run check:version` |
| `.dmg` のビルド | `npm run tauri build -- --bundles dmg` が成功し、`aarch64` の `.dmg` がちょうど 1 つある。artifact として 7 日間残す |

## 供給網の守り

- すべての外部 action をコミットの SHA で固定（ci-config.md の表）。Dependabot が毎週更新の PR を出す。
- 書き込み権限は `release.yml` の `build-and-publish` だけ。

## 外した門

- 数値のカバレッジ下限：team.md で設けないと決めている。
- 署名・公証：team.md で今はしないと決めている。
