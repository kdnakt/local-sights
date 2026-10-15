# Infrastructure Design 質問票 — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/nfr-design/`（performance・security・scalability・reliability・observability の各設計、logical-components.md）、`construction/u1-concurrent-skeleton/functional-design/functional-spec.md`、`inception/domain-design/components.md`、`inception/contract-design/contract-summary.md`。

このアプリはローカルのデスクトップアプリで、サーバー・クラウドの資源・監視の仕組みを持たない（team.md Deployment、project.md Forbidden）。U1 は新しいクラウドの資源も、配布の形（`.dmg`）の変更も持たない。ここで決めることは「U1 のコードの PR を、CI のどの確かめの上でマージするか」だけである。

## Q1. U1 のコードの PR は、PR #20（CI で行カバレッジ 80% を確かめる）のマージを待ってから出しますか？

背景：NFR 要件（NFR7）は「行カバレッジ 80% 以上を CI で確かめる」ことを求めている。その仕組みは PR #20 で作ったが、まだ `main` にマージされていない（CI は通っている）。U1 のコードの PR を先に出すと、その PR の CI ではカバレッジを確かめない。

A. PR #20 のマージを待つ：U1 のコードのブランチは PR #20 がマージされた後の `main` から切る。U1 の PR の CI でカバレッジの下限を確かめられる。コード生成の作業そのものは先に進め、PR を出す前に `main` を取り込む
B. 待たない：U1 の PR を先に出し、カバレッジは手元で `cargo llvm-cov` を走らせて確かめた結果を PR に書く
X. Other (please specify)

[Answer]: A. PR #20 のマージを待つ（利用者の回答：「カバレッジのプルリクマージして進めて」。PR #20 をマージしてから U1 のコードのブランチを `main` から切る）

## Consolidated Summary Confirmation

- 配置：今までどおりローカルのデスクトップアプリ（macOS Apple Silicon の `.dmg`）。サーバー・クラウドの資源・ネットワークの設定は足さない。AWS へは利用者の端末から CloudWatch Logs の読み取り API だけを呼ぶ
- 使う AWS のサービス：CloudWatch Logs だけ（変更なし）。U1 で変わるのは呼び方（同時 8 本、リージョンごとの速さの上限、GetLogEvents だけ SDK の再試行なし）で、新しい API・権限は要らない
- 監視：監視の仕組み・アラート・ダッシュボードは持たない。取得 1 回ごとの要約の 1 行を標準エラー出力へ出し、手元の確認用プログラムでも同じ 1 行を見る。外へは送らない
- CI：今の CI（fmt・clippy・テスト・画面の検査・cargo-deny・Tauri アプリのビルド）に、PR #20 の行カバレッジ 80% の確かめを足した上で U1 の PR をマージする。U1 では CI の手順を増やさない（futures-util は既存の cargo-deny で確かめる）。CI は実際の AWS に接続しない
- 順番：PR #20 を先にマージし、U1 のコードのブランチはその後の `main` から切る（Q1）
- 配布：U1 の PR だけではタグを付けない。版の上げ方・リリースの手順は今の決まり（`v0.1.2`、タグで GitHub Releases に公開）のまま

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
