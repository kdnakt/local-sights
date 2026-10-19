## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-18T03:28:43Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261017-insights-query/construction/u1-filter-skeleton/code-generation/code-summary.md > 残っていること | 薄い一本が実際の AWS に対してまだ一度も動かされていない。team.md の Walking Skeleton が求める確認 2（確認用プログラム fetch_check を本物の AWS で実行）と確認 3（macOS と Windows で GUI を起動して結果を目で見る）が未実施で、自動テストはすべて偽のゲートウェイ上で通っているだけである。QueryStore への取得、評価、結果の表示が画面から AWS まで通ることは、まだ証拠がない | 承認の前に、unit-test-instructions.md の手順で fetch_check を実行して `matched N of M events` を確かめ（timestamp-check.sh と同じ期間なら 2190 件）、macOS と Windows の GUI で Log Analytics の実行・[Cancel]・接続変更の確認・ログ管理の画面が変わらないことを確かめて、結果を記録する。確認済みと記録されるまでは「薄い一本が動く」と扱わない | New |
| R-02 | Minor | crates/local-sights-core/src/insights_session.rs > finish_evaluation と on_evaluation_progress | [Cancel] が最後の区切りの進み具合の確認より後、評価の完了通知より前に届くと、cancel_requested の印が捨てられて結果が Done として出る。保持が 0 件で評価の区切りが 1 回もない場合も同じである。R-01 の本文（評価中は次の区切りで止める）は区切りが残っている間は守られているが、最後の区切りの後は取り消しが効かない | finish_evaluation で cancel_requested が立っていれば結果を捨てて Cancelled にする。または、この窓は結果が出るだけで害がないとして、仕様に書いて受け入れる。どちらかを決める | New |
| R-03 | Minor | src-tauri/src/lib.rs > recover_if_query_still_running と crates/local-sights-core/src/query/runner.rs > evaluate_held | 評価の区切りが異常終了（スレッドの panic）すると、runner は QueryOutcome::EvaluationFailed を返すだけで sink に何も知らせない。そのあと回復の処理が、実行を「GetLogEvents の失敗（kind Other）」として FetchFailed にするため、利用者には取得の失敗と表示され、実際は評価の失敗である | 評価の失敗用の理由を InsightsSession に足すか、少なくとも api_name を GetLogEvents にしない。あるいは、起こらない前提の経路としてコメントに書いておく | New |
| R-04 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u1-filter-skeleton/code-generation/code-summary.md > 計画や設計との違い | 読み取りのエラーの場所（Span）が QueryProblem と日時の文字列だけにあり、条件の木の各部分に付いていない、と書かれている。U1 では評価時のエラーが日時の文字列由来だけなので問題は出ないが、U2 でエラーの下線と色分けを足すときに、条件の木に場所を足す作り直しが要る | U2 の機能設計に、条件の木へ Span を持たせる作業を明記して引き継ぐ | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc engine sensor-traceability | PASS: gaps 0, orphans 0, invalid 0, findings 0 | BR1.1 から BR7.3 までの対応に抜けはない |
| cargo test -p local-sights-core --lib query:: | PASS: 50 件 | 読み取り・項目・日時・評価・QueryStore・QueryRunner のテストがすべて通る |
| cargo test -p local-sights-core --lib insights_session:: | PASS: 9 件 | クエリの画面の状態のテストが通る |
| cargo clippy --workspace --all-targets | 警告 0 | 追加の対応は要らない |
| npx vitest run | PASS: 41 ファイル・437 件 | 画面側と既存のログ管理の画面のテストが通る |
| 非テストコードの unwrap と expect の検索（query/、insights_session.rs、src-tauri/src/lib.rs） | 該当なし | 禁止事項（unwrap と expect を使わない）を守っている |

**設計レビューの指摘 R-01 から R-05 の反映の確認**

- R-01（取り消しの印）：解消。InsightsRun に cancel_requested があり、runner が on_fetch_ended の答えで評価を止める。finish_fetch は印があれば Cancelled にする。最後のストリームの後に届く [Cancel] のテストもある。ただし評価の最後の区切りより後は上の Minor のとおり。
- R-02（取得の 1 本化）：解消。any_fetch_running と fetch_owner に集約され、設定の画面・ディスクを使う作業・SSO のログイン・接続の変更・入力の変更・閉じる確認と abort_fetch（query_abort も止める）が置き換わっている。残っている Phase::Fetching の直接参照は、ログ管理の画面自身の状態管理だけである。
- R-03（日時の読み取り）：解消。instant.rs が新しく読み取りを書き、曖昧・存在しない時刻だけ time_range::instant_in_zone を共用している（既存の parse_in_zone の挙動は変わっていない）。
- R-04（排他）：解消。begin_query が 1 回のロックで可否の確認・取得の開始・前の結果の消去を行い、stop handle の登録も同じロックの中で行っている。revision が古ければ StaleInput で断る。
- R-05（行のキー）：解消。QueryResult は QueryHoldings の位置の整数と epoch を持ち、rows_at と window が epoch で守っている。

**制約の確認**

- 呼ぶ AWS API は既存の取得の流れ（DescribeLogStreams と GetLogEvents）だけで、新しい API は足していない。テストの偽ゲートウェイは describe_log_groups を panic させ、クエリが一覧を呼ばないことも確かめている。
- 秘密の認証情報とアクセスキー ID は、新しいコードのログ・エラー・画面に出していない。runner と lib.rs の eprintln は固定の文だけである。
- ログ管理の画面：time_range.rs の変更は関数の切り出しだけで、ConditionArea.tsx は取得中の入力停止の条件を足しただけで、既存のテストは全部通る。画面はナビゲーションで隠すだけで残す作りである。

### Summary

設計レビューの 5 件の指摘はいずれもコードで解消されており、自動の確認（テスト・clippy・追跡の検査）はすべて通っているため、実装としては READY である。承認の前に人間が重く見るべきは、team.md の Walking Skeleton が求める実際の AWS と GUI（macOS と Windows）での確認がまだ行われていない点で、残りは小さな取り消しの隙間と失敗の表示の誤りである。
