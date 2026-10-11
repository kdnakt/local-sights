<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-10T16:01:07Z — Tauri 実機での確認（WKWebView 描画、DatePicker のハイフン入力、100 万行の性能測定）は非対話の作業では実行できないため、開発者が手元で行う項目として code-summary.md と README に残す方針で開発者に指示した；実行していない確認を「合格」と書かないことを明示した。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-10T16:01:07Z — 計画 Step 10 の「Cloudscape の表で測ってから決める」を、測れないため「既存の仮想スクロール＋トークンを既定にし、試作は環境変数で切り替え」に変えた；利用者の Q3 の選択（測って判断）は、測定後に既定を切り替える手順として残る。
- 2026-10-10T16:01:07Z — 開発者はコアのテストを `cargo +stable` で実行した；手元の既定ツールチェーン 1.90.0 が Cargo.lock の MSRV 1.94.1 未満のため。CI は stable を使うので影響はない。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-10T16:01:07Z — 上部ナビゲーションの中身を TopNavigation の utilities ではなく ButtonDropdown／Button にした；utilities には disabled がなく、取得中に接続を変えられない表（R-05）を満たせないため。見た目は同じ黒いバー。
- 2026-10-10T16:01:07Z — Cloudscape 採用で配布物が約 0.3 MB → 約 1.8 MB に増えた；ローカルアプリで配信コストはなく、要件に上限はないため受け入れた。文言束の遅延読み込みで減らせることを記録した。

- 2026-10-11T07:13:46Z — ゲートの修正依頼（カレンダーの切れ、プロファイル名、起動時の選択、左ペインの幅、アプリ名、アイコン）とレビュー R-01・R-03 を計画の Step 14 にまとめ、承認を取ってから 1 回の実装で直した；計画を直す前に報告の記録が必要だったため、順序は「記録 → 計画 → 承認 → 実装」とした。
- 2026-10-11T08:00:06Z — 2 回目の修正依頼（6 点）は Step 15 にまとめた；左ペインが狭くならない原因は表の長い名前の最小幅で、幅の単位を画面幅の % に変え、閉じる状態も足した。アイコンの四隅の白は qlmanage の描き出しが原因で、resvg による描き出しとアルファ検証のスクリプトに置き換えた。
- 2026-10-11T08:00:06Z — 計画の進捗印は修正のたびに engine が全部 `[ ]` に戻す；開発者は担当ステップの箱だけを付けるため、Step 1〜14 の箱はコンダクターが付け直す（手動の 3 項目は未チェックのまま）。
- 2026-10-11T08:40:00Z — 3 回目の修正依頼（日付の初期値、アイコンの大きさ）は Step 16 にまとめた；初期値の「昨日」はローカルゾーンの暦日で求め（夏時間の切り替え日でも日付の計算は壊れない）、時刻は補完の規則と同じ 00:00:00／23:59:59 にした。`format_input_in_zone` は瞬間→文字列なので、暦日から書式を組む crate 内関数を time_range.rs に足して session から呼んだ（計画からの小さな逸脱）。アイコンは Apple の規則どおり 1024 のうち中央 824 の板にし、余白の透明は各辺 4 点で検証する。
- 2026-10-11T07:13:46Z — 起動時に SDK デフォルトのプロファイルを選ぶ変更は、MVP の学び「起動時は何も選ばず、前回の選択も覚えない」の前半を利用者の指示で置き換えたもの；後半（覚えない）は維持。project.md の学びを書き換える必要があるか、学びの確認で利用者に委ねる。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-10T16:01:07Z — DatePicker のマスクはハイフンを区切りとして受け取り `yyyy/mm/dd` に正規化する（jsdom の解析）；実機でも同じなら FR2.4「ハイフンは形式の誤り」は「自動で直る」に読み替える必要があり、Build and Test または承認時に利用者に確認する。
- 2026-10-11T07:13:46Z — カレンダーの幅は Cloudscape の DatePicker がドロップダウンをトリガー幅に固定する仕様のため、日付欄を 234px にして対処した；カレンダー本体 238px との差で右端が数 px 切れる可能性が残り、実機で確かめる。
