# CI/CD Pipeline — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/nfr-design/`（security-design.md の設計の正本、performance・scalability・reliability・observability の各設計、logical-components.md）、`construction/u4-elapsed-time/functional-design/functional-spec.md`、`inception/domain-design/components.md`、`inception/contract-design/contract-summary.md`。まとめの確認は `infrastructure-design-questions.md`。

U4 はライブラリ（AppSession）と画面（状態行と経過の部品）を変えるが、CI・リリースのワークフローの手順は変えない。

## 進め方

1. U1〜U3 と同じブランチ `claude/parallel-fetch` に U4 のコードを足す。作業単位をまとめて 1 つの PR にする。
2. 純粋な判断（表記、合わせ直しと保持、AppSession の経過の決まり）はテストを先に書く。時計のフックと状態行は実装してからテストを書く（NFR6.12）。時計はライブラリも画面も差し替え、実際の AWS に接続しない（NFR6.2）。
3. 手元で `cargo fmt --all --check`・`cargo clippy -p local-sights-core --all-targets`・`cargo llvm-cov -p local-sights-core --fail-under-lines 80`・`cargo test -p local-sights-core --doc`、画面は `npx tsc --noEmit`・`npx eslint .`・`npx prettier --check .`・`npm run test:coverage`・`npx vite build`、アプリは `cargo clippy --workspace --all-targets`・`cargo test -p local-sights` を通す。
4. 手元の GUI で経過を確かめる（monitoring-design の Logs & Tracing）。
5. PR の CI がすべて通ってから `main` にスクワッシュマージする。直接の push はしない。
6. U4 だけではタグを付けない。

## 段と合否

| 段（CI のジョブ） | U4 に関わる点 |
|-------------------|---------------|
| Rust library (fmt, clippy, test) | AppSession の経過のテスト（時計を差し替える）、SessionView の `elapsed` の JSON の形、行カバレッジ 80%（NFR7.3） |
| Front end (tsc, eslint, prettier, vitest, build) | `src/elapsed.ts` の表記と合わせ直し、ElapsedTime・StatusLine・App のテスト、`src/test/fixtures.ts` の見本の更新で tsc が通ること、文言の一覧のテスト、行カバレッジ 80%（NFR7.3、NFR9.1） |
| cargo-deny | 依存を足さないので変わらない（NFR8.11） |
| Tauri app (ubuntu-latest / macos-latest) | アプリ側のコードは変えない。ビルドとテストが通ったまま |
| GitGuardian Security Checks | 今のまま |

必須チェックの名前（例：`Rust library (fmt, clippy, test)`）は変えない（project.md Corrections）。

## 配布・戻し方

コードは `main` のスクワッシュのコミットを revert する PR で戻す。配布済みの版は、GitHub Releases に残した前の版の `.dmg` を入れ直す（U1 と同じ）。

## CI の秘密情報

CI には AWS の認証情報を置かない。U4 で新しい秘密情報は要らない。

## Assumptions & Open Questions

None.
