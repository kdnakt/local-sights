# CI/CD Pipeline — U2 呼び出しの速さ（u2-call-pacing）

上流：`construction/u2-call-pacing/nfr-design/security-design.md`・`logical-components.md`、U1 の `infrastructure-design/cicd-pipeline.md`。まとめの確認は `infrastructure-design-questions.md`。

U2 はライブラリの中の CallPacer を変えるだけで、CI・リリースのワークフローの手順を変えない。

## 進め方

1. U1 と同じブランチ `claude/parallel-fetch` で、U2 のコードを足す（team.md Way of Working の短命のブランチの中で、作業単位をまとめて 1 つの PR にする）。
2. CallPacer の下げ方・戻し方はテストを先に書き、止めた時計で確かめる。結合テストは偽物のゲートウェイで、実際の AWS に接続しない（NFR6.1）。
3. 手元で `cargo fmt --all --check`・`cargo clippy -p local-sights-core --all-targets`・`cargo test -p local-sights-core`・`cargo llvm-cov -p local-sights-core --fail-under-lines 80` を通す。
4. PR の CI がすべて通ってから `main` にスクワッシュマージする。直接の push はしない。
5. U2 だけではタグを付けない。

## 段と合否

| 段（CI のジョブ） | U2 に関わる点 |
|-------------------|---------------|
| Rust library (fmt, clippy, test) | CallPacer の単体テスト、制限が続くと間隔が広がり止むと戻る結合テスト、下限まで下げた状態で閉じるテスト、行カバレッジ 80%。U1 の要約の書式のテストは `min_rate` の欄を足した形に直す（NFR7.1） |
| Front end (tsc, eslint, prettier, vitest, build) | 画面は変えない（FR2.6）。既存のテストが通ったまま |
| cargo-deny | 依存を足さないので変わらない |
| Tauri app (ubuntu-latest / macos-latest) | アプリ側は変えない。ビルドとテストが通ったまま |
| GitGuardian Security Checks | 秘密の値を置かない（今のまま） |

必須チェックの名前（例：`Rust library (fmt, clippy, test)`）は変えない（project.md Corrections）。

## 戻し方

コードは `main` のスクワッシュのコミットを revert する PR で戻す。配布済みの版は、GitHub Releases に残した前の版の `.dmg` を入れ直す（U1 と同じ）。

## CI の秘密情報

CI には AWS の認証情報を置かない。U2 で新しい秘密情報は要らない。

## Assumptions & Open Questions

None.
