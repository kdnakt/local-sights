# CI/CD Pipeline — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/nfr-design/security-design.md`、`reliability-design.md`、`performance-design.md`。質問票の回答は `infrastructure-design-questions.md` の Q1（PR #20 のマージを待つ）と、まとめの確認。

U1 では CI・リリースのワークフローの手順を増やさない。U1 の PR は、PR #20（行カバレッジ 80% の確かめ）をマージした後の `main` の CI で確かめる。

## 進め方

1. PR #20 を `main` にスクワッシュマージする（Q1）。
2. U1 のコードのブランチをその後の `main` から切る（短命のブランチ、team.md Way of Working）。
3. コード生成で、純粋な部品はテストを先に書き、AWS 接続と GUI の層は実装の後にテストを書く（team.md Testing Posture）。テストは FakeGateway と止めた時計を使い、実際の AWS に接続しない（NFR6.1）。
4. 手元で `cargo fmt --all --check`・`cargo clippy`・`cargo test`・`cargo llvm-cov -p local-sights-core --fail-under-lines 80` と、画面側の検査を通してから PR を出す。
5. 開発者が手元で、実際の AWS に対して確認用プログラムを動かし（NFR2.1 の比較を含む）、GUI で結果が出ることを目で確かめる（team.md Walking Skeleton）。
6. CI がすべて通ってから、`main` にスクワッシュマージする。直接の push はしない。
7. タグは付けない。リリースは今の決まり（`v0.1.2` のタグで GitHub Releases に公開）で、別に行う。

## 段と合否

| 段（CI のジョブ） | 確かめること | U1 に関わる点 |
|-------------------|--------------|----------------|
| Rust library (fmt, clippy, test) | rustfmt、clippy、ライブラリの単体・結合テストと行カバレッジ 80%（PR #20 から）、doc テスト | CallPacer・StreamSchedule・止める理由の強さ・要約の 1 行のテスト。既存のテストもすべて通ったまま（NFR7.1） |
| Front end (tsc, eslint, prettier, vitest, build) | 型・lint・整形・Vitest と行カバレッジ 80%（PR #20 から）・バンドル・npm audit | U1 は画面の変更が小さい。既存の画面のテストが通ったまま |
| cargo-deny | 脆弱性・ライセンス・禁止・出どころ（実際にビルドする機能が対象） | 新しい依存 futures-util（default-features なし、std だけ）がここで確かめられる（NFR8.5） |
| Tauri app (ubuntu-latest / macos-latest) | ワークスペースの clippy、アプリのビルド、アプリのテスト | FetchStatus の移動に合わせて直したアプリ側が通る（NFR7.1） |
| GitGuardian Security Checks | 秘密情報の混入 | 秘密の値をリポジトリに置かない（NFR8.2） |

ブランチの保護ルールが必須にしているチェックの名前（例：`Rust library (fmt, clippy, test)`）は変えない。名前を変えると必須チェックが「待ち」のままになり、マージできなくなる（PR #20 で起きたため戻した）。

## 配布・戻し方

| 項目 | 決まり |
|------|--------|
| 配布の仕方 | 今のまま。`main` に `vX.Y.Z` のタグを付けると macOS のランナーがテストしてから `.dmg` を作り、GitHub Releases に公開する。U1 の PR だけではタグを付けない |
| 段階的な出し方（blue-green・canary） | なし。利用者が新しい `.dmg` の `.app` を手で入れ替える |
| 戻し方 | コードは `main` のスクワッシュのコミットを revert する PR で戻す。配布済みの版は、GitHub Releases に残した前の版の `.dmg` を入れ直す |

## CI の秘密情報

CI には AWS の認証情報を置かない（NFR6.1、project.md Forbidden）。U1 で新しい秘密情報は要らない。ワークフローの権限は今のまま `contents: read`。

## Assumptions & Open Questions

None.
