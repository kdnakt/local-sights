# Functional Design Questions — U4 互換の確認用プログラム（u4-compat-check）

## Sources

- `inception/units-generation/unit-of-work.md`（U4 の責務：比べるクエリの組、書き出しの読み込みと比較、手元で実行、書き出しはリポジトリに置かない）
- `inception/requirements-analysis/requirements.md`（SM1、NFR3、NFR4、OQ5）
- U2 の `code-summary.md` の「U4 に引き継ぐ仮の決まり」（型の比べ方、AND・OR・NOT の大文字、`limit 0` と 10,000 件を超えるログ）と、U1 の決まり（`@log` の値はロググループ名だけのため、`@log` の値を比べるクエリは組に入れない）
- U2 の Q1a・Q1b の確かめ（利用者の手元で、CLI から Logs Insights にクエリを流した。CloudTrail のロググループ）

## Q1. コンソールの Logs Insights の結果を、どうやって手元に用意しますか？

背景：確認用プログラムは、同じロググループ・同じ期間について、コンソールの Logs Insights の結果とアプリの結果を比べる（SM1）。アプリ（と確認用プログラム）は読み取りの 3 つの API しか呼べないため、Logs Insights の結果は別に用意する。用意した結果はログの中身を含むため、リポジトリに置かない（NFR4）。

A. U2 の確かめと同じく、あなたの手元で CLI（`aws logs start-query` と `get-query-results`）から比べるクエリの組を流し、結果を JSON のファイルとしてリポジトリの外のフォルダに保存するスクリプトを用意する。確認用プログラムはそのフォルダを読む（推奨：クエリの組を一度に流せ、コンソールで 1 本ずつ書き出す手間がない。U2 の確かめで使えることも分かっている）
B. コンソールで 1 本ずつクエリを実行し、「Export results」から JSON か CSV で書き出して、リポジトリの外のフォルダに置く。確認用プログラムはそのフォルダを読む
C. Not yet defined
X. Other (please specify)

[Answer]: A. U2 の確かめと同じく、手元で CLI（start-query・get-query-results）から比べるクエリの組を流し、結果を JSON でリポジトリの外に保存するスクリプトを用意する
