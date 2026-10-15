# CI/CD Pipeline — U5 中断（u5-cancel）

上流：`construction/u5-cancel/nfr-design/`（security-design.md の設計の正本、performance・scalability・reliability・observability の各設計、logical-components.md）、`construction/u5-cancel/functional-design/functional-spec.md`、`inception/domain-design/components.md`、`inception/contract-design/contract-summary.md`。まとめの確認は `infrastructure-design-questions.md`。

U5 はライブラリ・アプリ・画面を変えるが、CI・リリースのワークフローの手順は変えない。

## 進め方

1. U1〜U4 と同じブランチ `claude/parallel-fetch` に U5 のコードを足す。作業単位をまとめて 1 つの PR にする。
2. 純粋な判断（decide_job_status、AppSession の押せるかの判定と状態の変わり方）はテストを先に書く。取得の流れの打ち切り・Tauri のコマンド・画面は実装してからテストを書く（NFR6.20）。一覧の段の打ち切りを確かめるため、テストの偽物のゲートウェイに DescribeLogStreams を遅らせる・返さない台本を足す。実際の AWS には接続しない（NFR6.13）。
3. 手元で `cargo fmt --all --check`・`cargo clippy -p local-sights-core --all-targets`・`cargo llvm-cov -p local-sights-core --fail-under-lines 80`・`cargo test -p local-sights-core --doc`、画面は `npx tsc --noEmit`・`npx eslint .`・`npx prettier --check .`・`npm run test:coverage`・`npx vite build`・`npm run check:version`・`npm audit --audit-level=high`、アプリは `cargo clippy --workspace --all-targets`・`cargo test -p local-sights` を通す。
4. 手元の GUI で中断を確かめる（monitoring-design の Logs & Tracing）。
5. PR の CI がすべて通ってから `main` にスクワッシュマージする。直接の push はしない。
6. U5 だけではタグを付けない。

## 段と合否

| 段（CI のジョブ） | U5 に関わる点 |
|-------------------|---------------|
| Rust library (fmt, clippy, test) | 止め方・N・終わりの状態・AppSession の結合テストと単体テスト、閉じるときの回帰テスト、行カバレッジ 80%（NFR7.4） |
| Front end (tsc, eslint, prettier, vitest, build) | ボタン・注意・状態行・文言のテスト、行カバレッジ 80%。版の整合と npm audit は今のまま |
| cargo-deny | 依存を足さないので変わらない（NFR8.14） |
| Tauri app (ubuntu-latest / macos-latest) | `cancel_fetch` と持ち手の登録の順のテスト、権限の 3 か所がそろってビルドが通ること |
| GitGuardian Security Checks | 今のまま |

必須チェックの名前（例：`Rust library (fmt, clippy, test)`）は変えない（project.md Corrections）。

## 配布・戻し方

コードは `main` のスクワッシュのコミットを revert する PR で戻す。配布済みの版は、GitHub Releases に残した前の版の `.dmg` を入れ直す。

## CI の秘密情報

CI には AWS の認証情報を置かない。U5 で新しい秘密情報は要らない。

## Assumptions & Open Questions

None.
