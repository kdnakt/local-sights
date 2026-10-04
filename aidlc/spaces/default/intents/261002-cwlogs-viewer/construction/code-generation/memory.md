<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-10-04T08:30:00Z — [u1-walking-skeleton] functional-spec で「Code Generation で決める」とした画面とライブラリのつなぎ方を、計画の前に質問して決めた（Q1：Tauri のコマンドとイベント）; project.md Corrections の「先送りした技術選定は必要になった時点で質問する」に従った。
- 2026-10-04T08:30:00Z — [u1-walking-skeleton] Testing Contract の custom の順序を、ライブラリ側は純粋なロジックをテスト先行・AWS 接続と取得の流れと AppSession を実装後テスト、画面側はすべて実装後テストと解釈した。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-10-04T08:30:00Z — [u1-walking-skeleton] 計画にないファイル（src/format.ts など 5 つ）と、Tauri の feature `custom-protocol`、`FetchSink::on_batch` の job_id 引数を足した; 詳細と理由は code-summary.md。
- 2026-10-04T08:30:00Z — [u1-walking-skeleton] 再レビュー（上限の 2 回目）で出た R-06 を人間の判断で直したが、骨組みのチェックポイントがレビュー済みの内容との一致を求めたため、人間の判断で R-06 と機能設計の R-08（rules.md の applies_to の名前）をいったん戻した; どちらも U3（取得の作り込み）で直す。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-10-04T14:30:00Z — [u2-connection-selection] 設定ファイルは寛容に読み、解釈できない行だけを読み飛ばす（機能設計の BR1.3 からの意図した逸脱、人間の判断）; 1 行の崩れで全プロファイルが消えるのを避ける代わりに、崩れたプロファイルは知らせなしに一覧から消える。
- 2026-10-04T14:30:00Z — [u2-connection-selection] リージョンの一覧は組み込みの静的な一覧にした; ネットワークを使わない代わりに、新しいリージョンは手で足す必要がある。
- 2026-10-04T08:30:00Z — [u1-walking-skeleton] 伏せ字は、利用者が入力した名前にはアクセスキー ID の形だけ、SDK 由来の項目には 40 文字・100 文字以上の判定も使う形にした（R-02）; 失敗した対象が読めることと、秘密を出さないことの両立。
- 2026-10-04T08:30:00Z — [u1-walking-skeleton] `src-tauri` には自動テストを置かず、判断をライブラリ側（AppSession）に寄せてテストした; つなぎの部分は GUI の目視で確かめる。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-10-04T08:30:00Z — [u1-walking-skeleton] 画面側のテスト（`npm test`）と `cargo-deny` を CI でどう回すかは CI Pipeline ステージで決める。
- 2026-10-04T14:30:00Z — [u2-connection-selection] 再レビュー（上限の 2 回目）の R-06（絞り込みと一覧の選択が async コマンドになり、到着順の保証が弱まる）は、project.md の決まりに従い U3 で扱う（同期コマンドに戻すか連番を付ける）。U1 から持ち越した R-06（取得開始時に前回の行を消す）・R-08 も U3 で扱う。
- 2026-10-04T14:30:00Z — [u2-connection-selection] 作業単位ごとのチェックポイントは、後の単位が前の単位のファイルを変えるたびに前の単位の承認が古くなる仕組みのため、人間の承認で無効にした。以降はステージごとの承認で進む。
