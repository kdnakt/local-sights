# CI/CD Pipeline — U3 あきらめの決まり（u3-give-up-rule）

上流：`construction/u3-give-up-rule/nfr-design/security-design.md`・`logical-components.md`、U1・U2 の `infrastructure-design/cicd-pipeline.md`。まとめの確認は `infrastructure-design-questions.md`。

U3 はライブラリの中の StreamSchedule・RetryPolicy・EventFetcher を整えるだけで、CI・リリースのワークフローの手順を変えない。

## 進め方

1. U1・U2 と同じブランチ `claude/parallel-fetch` に U3 のコードを足す。作業単位をまとめて 1 つの PR にする。
2. StreamSchedule の数えの決まりはテストを先に書く。結合テストは偽物のゲートウェイと止めた時計で、実際の AWS に接続しない（NFR6.1）。
3. 手元で `cargo fmt --all --check`・`cargo clippy -p local-sights-core --all-targets`・`cargo test -p local-sights-core`・`cargo test -p local-sights-core --doc`・`cargo llvm-cov -p local-sights-core --fail-under-lines 80` を通す。アプリ側が変わらないことを `cargo clippy --workspace --all-targets` と `cargo test -p local-sights` で確かめる。
4. PR の CI がすべて通ってから `main` にスクワッシュマージする。直接の push はしない。
5. U3 だけではタグを付けない。

## 段と合否

| 段（CI のジョブ） | U3 に関わる点 |
|-------------------|---------------|
| Rust library (fmt, clippy, test) | StreamSchedule の数えのテスト（retry.rs から移したものを含む）、終わる順を入れ替える結合テスト、doc テスト、行カバレッジ 80% |
| Front end (tsc, eslint, prettier, vitest, build) | 画面は変えない。既存のテストが通ったまま |
| cargo-deny | 依存を足さないので変わらない |
| Tauri app (ubuntu-latest / macos-latest) | アプリ側は変えない（`RetryPolicy::default()` の使い方は同じ）。ビルドとテストが通ったまま |
| GitGuardian Security Checks | 今のまま |

必須チェックの名前（例：`Rust library (fmt, clippy, test)`）は変えない（project.md Corrections）。

## 戻し方

コードは `main` のスクワッシュのコミットを revert する PR で戻す。配布済みの版は、GitHub Releases に残した前の版の `.dmg` を入れ直す（U1 と同じ）。

## CI の秘密情報

CI には AWS の認証情報を置かない。U3 で新しい秘密情報は要らない。

## Assumptions & Open Questions

None.
