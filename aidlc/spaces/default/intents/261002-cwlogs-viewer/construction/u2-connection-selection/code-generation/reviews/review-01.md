## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-04T13:24:48Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | crates/local-sights-core/src/catalog/files.rs > read_shared_file（63 行付近）と catalog/mod.rs > SectionReader::feed_line、テスト broken_lines_are_skipped_and_the_rest_is_used | BR1.3 は「読めない、または解釈できない」ファイルを 0 件にして、ファイルの種類だけを知らせると定めている。実装で Unreadable になるのは I/O エラーと UTF-8 でない場合だけで、形式の崩れ（閉じていない見出し、見出し行の後ろのコメントなど）は黙って読み飛ばし、知らせも出さない。そのため見出しが崩れたプロファイルは通知なしに一覧から消える。テストは「崩れた行は読み飛ばして残りを使う」という逆の振る舞いを確かめており、code-summary.md の「計画との違い」にもこの逸脱は書かれていない | 次のどちらかを行う。(a) 解釈できない場合の判定（例：見出しとして始まるのに閉じていない行）を足し、そのファイルを 0 件で Unreadable として知らせる。(b) 寛容に読む方針を意図した逸脱として code-summary.md の「計画との違い」に書き、BR1.3 との差を人間が承認できるようにする。どちらでも、その方針を確かめるテストを置く | New |
| R-02 | Major | crates/local-sights-core/src/fetcher.rs 61 行（region: request.region()）、tests/u1_fetch_flow.rs、traceability.json の BR2.8 | BR2.8（画面からの取得は選んだリージョンで接続する）の配線のうち、取得の要求（ValidatedFetch）から GetLogEventsRequest.region へ渡す 1 行にテストがない。u1_fetch_flow.rs とライブラリ内のテストには GetLogEvents の呼び出しの region を確かめる断言がなく、この行を None にしてもテストは通る（その場合、取得はプロファイルの既定のリージョンに黙ってずれる）。DescribeLogGroups 側は region を断言している。traceability.json は BR2.8 の対応先を gateway/aws.rs だけにしているが、regionを実際に渡す経路は request.rs・session.rs・fetcher.rs にある | tests/u1_fetch_flow.rs か fetcher のテストに、with_connection で region を持たせた要求を run_fetch に渡し、偽物の gateway が記録した GetLogEventsRequest の region と profile_name が選んだ値と一致する断言を足す。traceability.json の BR2.8 の対応先に fetcher.rs か session.rs を加える | New |
| R-03 | Minor | crates/local-sights-core/src/session.rs > can_change_connection、src/components/FetchForm.tsx（disabled は fetching のみ）、src/components/ConfirmDialog.tsx（aria-modal、フォーカスの閉じ込めなし）、code-summary.md「主な判断」最終項目 | BR2.6 は「確認を待っている間に使えるのは確認ダイアログの [変える]・[キャンセル] だけ」と定めるが、実装では確認待ちの間もストリーム名・日時の入力と絞り込みの欄が使え、ダイアログは aria-modal でありながら Tab で背面の入力欄へ出られる。code-summary は BR2.5 の文言を根拠にこの扱いを「主な判断」に書いており隠してはいないが、BR2.6 の最後の文とは食い違う。BR5.2 が求めるのは開いたときの [キャンセル] へのフォーカスと Escape だけなので、フォーカスの閉じ込め自体は必須ではない | BR2.5 と BR2.6 のどちらに合わせるかを決める。BR2.6 に合わせるなら、確認待ちの間は入力欄と絞り込みを無効にするか、ダイアログ内でフォーカスを循環させる。BR2.5 に合わせるなら、code-summary の記述を「BR2.6 の最後の文からの逸脱」と明記し、ルールの食い違いを次の機能設計の見直しに送る | New |
| R-04 | Minor | src/components/LogGroupPane.tsx > renderStatus（Partial の分岐が NoMatches より先） | 一覧が Partial（途中まで）で、取得済みの一覧を絞り込んで 0 件になったとき、「途中までである」文言だけが出て BR3.10 の「一致するロググループがありません」が出ない。コアの empty_state は NoMatches を返しているのに画面が使わない。利用者は絞り込みの結果が空の理由を、一覧が途中までのせいか入力のせいか判別できない | Partial の表示に加えて emptyState が NoMatches のときはその文言も並べて出し、LogGroupPane.test.tsx に Partial かつ 0 件一致のテストを足す | New |
| R-05 | Minor | src-tauri/src/lib.rs > change_connection（logs_cleared の分岐で timeline.lock().await.clear() を spawn） | 表示中のログの破棄（BR2.4）を非同期に spawn した別タスクで行うため、その後に始めた取得の run_fetch（同じ timeline のロックを取る）との順序が保証されない。取得が先にロックを取ると、あとから clear が走って新しい取得の保持ログを消す可能性がある。人間の操作の間隔では起こりにくく、画面の行はフロント側が別に管理しているため影響は小さい | clear を spawn せず、コマンド側でロックを取って確実に消してから SESSION_CHANGED を送るか、取得開始側で世代を確かめる | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| cargo test --workspace | PASS（lib 121、u1_fetch_flow 8、u2_log_group_listing 9） | 実際の AWS には接続しない。code-summary の件数と一致 |
| cargo fmt --check | PASS | 出力なし |
| cargo clippy --workspace --all-targets | PASS | 警告なし |
| npx vitest run | PASS（54 件） | code-summary と一致 |
| npx tsc --noEmit・npx eslint . | PASS | 出力なし |
| npx prettier --check . | PASS | All matched files use Prettier code style |
| テスト以外の unwrap・expect・panic の検索 | 該当なし | team.md のエラー処理の決まりを守っている |

**確認した点（指摘なし）：** BR1.2（見出しと region 以外は保持せず、エラーにも中身を出さない）、BR1.5 の順序（AWS_REGION、AWS_DEFAULT_REGION、AWS_PROFILE のプロファイル、[default]）、BR2.4（Idle への戻し・ログ・件数・直近の結果の消去）、BR2.5 のダイアログ（キャンセルで元に戻る）、BR2.7 の 2 段の検証、BR3.1 の同じトークンの防御、BR3.6 の古い応答と取得の打ち切り、BR3.7 の絞り込みの保持、BR3.9（DescribeLogGroups には nextToken だけを渡す）、BR5.2 の Tab 順・矢印キー・フォーカス・Escape。リージョンの一覧は静的で、ネットワークも API も使わない。Tauri の権限は自アプリのコマンドと core:event:default だけ。秘密情報・テレメトリ・実際の AWS への接続は見つからなかった。

### Summary

Critical はなく、Major は 2 件（BR1.3 の「解釈できない」が未実装で逸脱も未記載であること、BR2.8 の取得側の region 配線にテストがないこと）。どちらも直せる範囲で、ほかのルールは仕様どおりに実装され、テストも品質基準を満たすため READY とする。
