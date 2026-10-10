## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-04T07:16:34Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | src/App.tsx > rowsForSession と applySession、src-tauri/src/lib.rs > start_fetch の戻り値 | start_fetch は begin_fetch 直後のビュー（currentJobId が null）をコマンドの応答として返し、画面はそれを applySession で適用する。応答が最初の log-batch または on_started の session-changed より後に届くと、rowsForSession が現在ジョブの行を空にし jobId を null に戻す。次の log-batch は rowsWithBatch が置き換えるため、それ以前のページが一覧から欠ける。通常は応答が先に着くため発生しにくいが、順序は保証されていない。 | start_fetch の応答では行を触らない（phase と canFetch だけ反映する）、または応答の currentJobId が null のときは rowsForSession をスキップする。順序が逆の場合のテストを App.test.tsx に 1 件足す。 | New |
| R-02 | Minor | crates/local-sights-core/src/failure.rs > build_safe_detail と redact_secrets | ロググループ名・ストリーム名・プロファイル名も redact_secrets を通すため、ちょうど 40 文字の英数字・スラッシュ・プラスだけの名前（例：ecs/app/ に 32 桁の 16 進が続くストリーム名）や 100 文字以上の名前が伏せ字になり、利用者がどの対象で失敗したか読み取れない。code-summary にも記載があり意図した安全側の倒し方だが、診断性が下がる。 | 利用者が入力した名前の欄には、アクセスキー ID の形だけを伏せる弱い規則を使うか、伏せ字になる場合があることを BR4.4 の暫定表示として README か U7 の課題に記録する。 | New |
| R-03 | Minor | src-tauri/capabilities/default.json > permissions | core:default はウィンドウ・パス・アプリなど画面が使わない権限も含む。必要なのはイベントの購読（core:event:default）とアプリ自身の 3 コマンドだけで、最小権限の主張とずれる。 | core:default を core:event:default に絞って起動と購読が動くか確かめる（動かない場合は理由を description に書く）。 | New |
| R-04 | Minor | src-tauri/src/lib.rs > start_fetch の spawn と TauriSink | 取得タスクが panic などで on_finished に到達しないと、セッションが Fetching のまま残り、BR1.4 により入力と [Fetch] が永久に無効になる。また、TauriSink とコマンド配線（BR4.5 の順序を画面まで届ける部分）には自動テストがなく、cargo build だけで確認している。 | spawn の結果を監視し、異常終了時は finish_fetch 相当で Failed（種類 Other）に戻す。可能なら FetchSink の呼び出し順をつなぎ側でも 1 件検証する。 | New |
| R-05 | Minor | src/styles.css（text-overflow: ellipsis）> BR5.2 | 1 行表示の省略記号は CSS だけで実現されており、LogTable.test.tsx は改行の置き換えだけを確認している。省略表示そのものは自動では確かめられない。 | 手元の GUI 確認（Walking Skeleton の 3 番）の項目に省略記号の目視を加える。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| cargo test -p local-sights-core --lib（request, time_range, paging, event, timeline, failure, gateway, session） | PASS: 65 件成功 | code-summary の件数と一致。ライブラリ側の公開関数にテストがある |
| cargo test -p local-sights-core --test u1_fetch_flow | PASS: 8 件成功 | BR3.1、BR3.2、BR4.1、BR4.5、BR1.6（RegionMissing で GetLogEvents を呼ばない）を偽 gateway で確認している |
| cargo fmt --check | PASS | 指摘なし |
| cargo clippy --workspace --all-targets | PASS: 警告なし | Tauri アプリを含む全体が組み立つ |
| npx vitest run | PASS: 29 件成功 | 全体 6 ファイル。code-summary と一致 |
| npx tsc --noEmit | PASS | 出力なし |
| npx prettier --check . | PASS | 指摘なし |
| npx eslint . | PASS | 出力なし |

### Rule and policy verification

- BR2.2、BR3.1、BR3.2：time_range.rs の api_end_time は endInstant に 1 を足した値で、fetcher.rs は毎回 startTime、endTime、startFromHead=true を渡す。paging.rs の decide と PageCursor は、同じトークンまたはトークンなしで終わり、最後の同じトークンの応答も pageCount に数える。仕様どおり。
- BR4.1、BR4.5、BR1.6：coordinator.rs が開始時に timeline を空にしてから on_started を呼び、各ページを timeline に追加して on_batch を呼び、最後に on_finished を呼ぶ。エラー時も取得済みの分を残す。gateway は region が無ければ API を呼ばずに RegionMissing を返す。
- project.md Forbidden：呼べる API は GetLogEvents だけ。SDK の生のエラーメッセージは捨てられ、safeDetail は許可リストの項目だけで伏せ字を通る。環境変数を変えるテストは一時ファイルと IMDS 無効だけを使い、実際の AWS には接続しない。テストの認証情報は実行時に組み立てたダミー。テレメトリなし。CSP は connect-src が ipc だけ。
- team.md：unwrap と expect は cfg(test) の中だけ（grep で確認）。ライブラリと GUI が分かれ、AWS は trait の裏にある。
- traceability.json：全 35 件の OK の対象ファイルが存在し、内容が ID に対応している。code-summary の計画との違いは実際の差分と一致している。

### Summary

Critical と Major の指摘はない。BR の実装、エラー・伏せ字の扱い、テストの順序と量、トレーサビリティは証拠と一致し、検証コマンドはすべて成功した。残るのは、応答と event の到着順の競合、伏せ字の過剰適用、権限の絞り込み、取得タスクの異常終了、省略記号の確認という Minor の 5 件で、開発者が実装できる状態にある。
