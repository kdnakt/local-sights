# Code Generation 質問票 — U6 ディスクキャッシュ（u6-disk-cache）

上流の成果物：`construction/u6-disk-cache/functional-design/`（functional-spec.md・rules.md・entities.md）、`inception/units-generation/unit-of-work.md`、`inception/requirements-analysis/requirements.md`。

機能設計は、キャッシュのファイルの具体的な書式を Code Generation で決めるとしている（entities.md、BR4.2）。計画を書く前に、この 1 点だけを決める。ほかの選択（SHA-256 は `sha2` クレート、OS のフォルダは Tauri のパスの仕組みで決めてライブラリに渡す、BR1.6）は、ほかに有力な選び方がないため計画に書く。

## Q1. キャッシュのファイルを、どの書式で保存しますか？

背景：1 つのキー（プロファイル・リージョン・ロググループ）のキャッシュを 1 つのファイルにまとめます（Q3）。BR4.2 により、キーとキャッシュ済みの範囲はイベントを読まずに確かめられる必要があります。100 万件近いログを保存することもあります。

A. JSON Lines（1 行目にキー・書式の版・キャッシュ済みの範囲、2 行目以降に 1 行 1 イベント。今使っている serde_json だけで書け、中身を人が読める。大きさは圧縮しない分だけ大きい）
B. 独自のバイナリ（bincode などの新しいクレートを足す。小さく速いが、中身を人が読めず、書式の版の管理を自分で作る）
C. JSON Lines を gzip で圧縮（A と同じ形を圧縮する。圧縮のクレートを足す。小さくなるが、1 行目だけを読むにも展開が要る）
X. Other (please specify)

[Answer]: A. JSON Lines（1 行目にキー・書式の版・キャッシュ済みの範囲、2 行目以降に 1 行 1 イベント。今使っている serde_json だけで書け、中身を人が読める。大きさは圧縮しない分だけ大きい）

## Consolidated Summary Confirmation

回答のまとめ：

- キャッシュのファイルは JSON Lines で保存する。1 行目にキー・書式の版・キャッシュ済みの範囲を置き、2 行目以降に 1 行 1 イベントを書く。新しいクレートは足さず serde_json で書く（Q1）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct

## Plan Approval

対象：`code-generation-plan.md`（埋め込みの Testing Contract を含む）と `unit-test-instructions.md`。

[Approval Fingerprint]: sha256:v3:ab77c7ab0608e3920f5d47bbaf60d1037da425f7f53aebc3f03c70aa74bac5c5
[Planned Source]: f8a66d21bde8517ee7a02e39acda7b608423ee107bfaeeee7b8ebf38beb0e1a1

- Approve Plan
- Request Changes

[Answer]: Approve Plan
