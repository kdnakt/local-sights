# NFR Design 質問票 — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/nfr-requirements/`（performance・security・scalability・reliability・observability の各要件、tech-stack-decisions.md）、`construction/u1-concurrent-skeleton/functional-design/`、`inception/contract-design/contract-summary.md`。

U1 の仕組み（速さの上限の許可待ち、同時 8 本の並べ方、SDK の再試行の上書き、閉じるときの止め方、診断ログの 1 行）は、機能設計と NFR 要件でほぼ決まっている。ここでは、NFR 要件のレビューで残った「手元の比較の手順」だけを聞く。カバレッジ 80%（NFR7）とテスト先行（NFR6）は、コード生成の確かめの手段に入れる（質問にしない）。

## Q1. 手元の実際の AWS での比較（main の版と U1 の版）は、どう測りますか？

背景：今の main の確認用プログラム（FetchCheck）には、リージョンを指定する引数も、かかった時間の表示もありません。U1 の版には両方が入ります。条件をそろえないと、比べた結果が当てになりません。1 回だけ測ると、ネットワークのばらつきで結果が変わることもあります。

A. 環境変数でリージョンをそろえ、`time` で全体の時間を測る：両方とも `AWS_REGION=ap-northeast-1` を付けて、同じプロファイル・同じロググループ・同じ時間範囲で、それぞれ 3 回ずつ動かす。`time` が出す実時間の中央値と件数を比べる。件数が同じで U1 の中央値のほうが短ければ合格。手順は U1 の確認の手順書に書く
B. 1 回ずつ測る：A と同じ条件で 1 回ずつ。手間は少ないが、ばらつきの影響を受ける
X. Other (please specify)

[Answer]: A. 環境変数でリージョンをそろえ、`time` で全体の時間を測る：両方とも `AWS_REGION=ap-northeast-1` を付けて、同じプロファイル・同じロググループ・同じ時間範囲で、それぞれ 3 回ずつ動かす。`time` が出す実時間の中央値と件数を比べる。件数が同じで U1 の中央値のほうが短ければ合格。手順は U1 の確認の手順書に書く

## Consolidated Summary Confirmation

- 速さの上限：CallPacer はロックの中で枠を予約してから tokio の時計で待つ（間隔は 1,000,000,000 ÷ 上限 ナノ秒を切り上げ）。待ちと止める信号が同時に来たときは止める信号を先に見る
- 同時に読む：取得 1 回のタスクの中で FuturesUnordered に最大 8 本を並べ、1 本 1 ページずつ進める。渡し待ちは最大 8 ページ
- SDK の再試行：GetLogEvents の呼び出しだけ、操作ごとの設定で再試行を 1 回に上書きする
- 閉じるとき：止める信号（理由つき）で、許可待ち・再試行の待ちからすぐ戻る。返ったページは渡さない
- 診断ログ：取得 1 回の要約（本数・呼び出しの数・制限の回数・時間・同時の最大・終わりの状態）を数える入れ物を取得 1 回ごとに作り、終わりに 1 行を標準エラー出力へ。確認用プログラムも同じ組み立てを使う
- 確かめの手段：テスト先行（純粋な部品：CallPacer の間隔の計算、StreamSchedule、止める理由の強さ、要約の文字列）、行カバレッジ 80%（CI）、自動テストは実際の AWS に接続しない
- 手元の比較：両方とも AWS_REGION=ap-northeast-1、同じ条件で 3 回ずつ、time の実時間の中央値と件数を比べる（Q1）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
