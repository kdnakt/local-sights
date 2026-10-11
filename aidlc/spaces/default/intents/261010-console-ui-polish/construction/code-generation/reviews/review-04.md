## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-11T09:30:00Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | src-tauri/tauri.conf.json > app.security.csp / devCsp | CSP に `font-src` がなく `default-src 'self'` が効く。一方 `dist/assets/*.css` の Cloudscape の Open Sans は `url(data:font/woff2;base64,...)` で埋め込まれている。WKWebView では data: のフォントが CSP で拒否され、フォントが読めずに既定フォントへ落ちる、またはコンソールに違反が出る可能性が高い。jsdom のテストでは検出できず、手動確認の表にも CSP の項目がない | `npm run tauri dev` と `tauri build` の実機で、フォント読み込みの CSP 違反が出ないか確認する。出るなら CSP に `font-src 'self' data:` を足す（秘密情報・外部通信は増えない）。手動確認の表と README のチェックリストに「CSP 違反なし」を加える | Resolved (reviewer) |
| R-02 | Major | aidlc/spaces/default/intents/261010-console-ui-polish/construction/code-generation/code-summary.md > 性能測定（NFR2、R-03） / traceability.json NFR2 | NFR2 の 100 万行の測定が未実施（4 項目すべて「未測定」）で、traceability では Deferred。ログ一覧は Cloudscape 部品ではなく自前の仮想スクロールのまま（SM1 の例外）。判断は安全側で一貫し、例外も明示されているが、NFR2 が満たされたことの根拠はまだない。また試作の切り替えは Vite のビルド時環境変数のため、測定のたびに dev の再起動が要る | 承認時に SM1 の例外（ログ一覧は自前）を人が明示的に受け入れるか決める。測定は承認後の手元作業として期限と記録先（code-summary.md の表）を決めておく。測定が通った場合に `LOG_TABLE_VARIANT` の既定を切り替える手順をフォローアップにする | Unresolved |
| R-03 | Minor | src/dev/perfRows.ts > ROW_COUNT / GROWING / LOG_TABLE_VARIANT、src/api.ts（getRows・rowPositions・setFilter の分岐）、src/App.tsx > usePerfRows | 性能ハーネスは既定では不活性（環境変数が未設定なら `enabled=false`、`.env` ファイルもない）。ただしコードは本番バンドルに常に入り、`VITE_LOCAL_SIGHTS_PERF_ROWS` がシェルに残った状態で `tauri build` すると、配布物が生成行を表示する形で意図せず有効になりうる。ビルド時の歯止めがない | `import.meta.env.DEV` でも条件づけ、本番ビルドでは常に無効かつツリーシェイクで除去されるようにする。難しければ README に「本番ビルド前にこれらの変数を unset する」と書く | Resolved (reviewer) |
| R-04 | Minor | crates/local-sights-core/src/time_range.rs、src/components/ConditionArea.test.tsx、code-summary.md > 計画からの逸脱（Step 8）、手動確認 R-07 | DatePicker にハイフン入力するとマスクが `/` に正規化する（jsdom での解析のみ）。このためコアのハイフン拒否（FR2.4）は画面の入力経路から到達しにくく、「ハイフンで日付欄の直下に理由」の画面テストは入力ではなくコアが持つ旧文字列で代替している。実機未確認。要件側の期待（拒否か自動の直しか）が code-summary に「どちらでもよい」と明記されていない | 実機で R-07 を確認し、結果を code-summary.md に追記する。ハイフンが自動で直る挙動を採用するなら、要件 FR2.4 との対応（拒否は貼り付けなどコアに文字列が届く経路で効く）を一文で明記する | Unresolved |
| R-05 | Minor | src/components/TopBar.tsx（custom content）、code-summary.md 判断 2 | `TopNavigation` の `utilities` に disabled がないため、`ButtonDropdown`／`Button` を custom content に置いた。計画 Step 6 の `menu-dropdown`／`button` からの逸脱だが記録され、R-05 の有効・無効の表は実装・テストされている。コンソールとの見た目・キーボード挙動（overflow での折りたたみなど）の差は実機では未確認で、最小幅 1024px で上部ナビゲーションが崩れないかは NFR7 の手動確認に含まれる | NFR7 の手動確認（1024×640）で、上部ナビゲーションの 4 つの操作部品が溢れず操作できることを確かめる | Unresolved |
| R-06 | Minor | code-summary.md > 配布物の大きさ | JS は 271 kB から 911 kB、CSS は 3.65 kB から 916 kB に増えた（`dist/` 約 1.8 MB。文言束とフォントの埋め込みが主因）。報告はされており、ローカルアプリで上限要件もないため問題ではない。起動時間への影響は実測がない | 必要になった時点で文言束の遅延読み込みを検討する。手動確認の起動時に体感の遅さがないか一言確認する | Unresolved |
| R-07 | Minor | src-tauri/src/lib.rs > setup（`startup_listing` から `start_listing` を呼ぶ箇所）、src/App.tsx > 最初の `useEffect`（`onSessionChanged` の登録と `getSession` の呼び出し） | 起動時の一覧取得（14c）は、画面の読み込みより前に始まる。画面側は `getSession()` の取得と `onSessionChanged` の登録を並行して始めるため、取得が非常に速く終わった場合（例：認証失敗がすぐ返る）に、スナップショットの後かつリスナー登録の前に出た `SESSION_CHANGED` を取りこぼす余地がある。取りこぼすと一覧が Loading のままになりうる。窓は狭く、普通の API 往復では起きにくい | 実機で認証情報なし／ネットワーク切断の状態で起動し、一覧が Loading のまま残らないか確認する。残るなら、リスナーの登録が終わってから `getSession()` を呼ぶ（順に実行する）か、登録後にもう一度 `getSession()` で再同期する | Unresolved |
| R-08 | Minor | aidlc/spaces/default/intents/261010-console-ui-polish/construction/code-generation/traceability.json > GATE-1〜GATE-6、REV-R01、REV-R03 | これらは要求 ID ではなくゲートの修正依頼とレビュー指摘の ID である。`status` と `target` は他の要求と同じ形式で記録され、整合している（検証ツールの合格）。現状は要求 ID と区別がつかない | 許容できる。code-summary.md か traceability の説明に「GATE-*／REV-* はゲートの修正依頼・レビュー指摘の追跡 ID で、要求 ID ではない」と一文添えるとよい | Resolved (reviewer) |
| R-09 | Minor | src/components/PaneResizer.tsx > containerWidthInPixels、src/styles.css > .app-body / .log-group-pane | ペインの幅（`flex-basis` の %）は `.app-body` の内容領域（左右の padding を除く）が基準だが、ドラッグの換算は親要素の `clientWidth`（padding を含む）で割っている。padding は `--ls-space-m` の左右ぶんで、窓幅 1200px なら約 3% の差になり、ドラッグ中にカーソルとハンドルが少しずつずれる。jsdom では padding が無いため、テストでは捕まらない。 | 換算の分母を `clientWidth` から左右の padding を引いた値にする（`getComputedStyle` の `paddingLeft`／`paddingRight`）。直さない場合は実機のドラッグ確認で許容範囲かを見て、code-summary.md に残す。 | Unresolved |
| R-10 | Minor | src/styles.css > .log-group-pane-table table、src/components/LogGroupPane.tsx > name 列の `.log-group-pane-name` | `min-width: 0` と `table-layout: fixed` と省略記号でペインが 20% まで縮む設計は、原因（`white-space: nowrap` の min-content 幅）に合っている。ただし Cloudscape の Table は選択列の幅やセルの内側の要素を自前の CSS で決めるため、実際の Tauri 画面で名前が切られるか（横スクロールが出ないか）は jsdom では確認できない。 | 実機で、最小の 20% まで縮めたときに名前が省略記号で切られ、横スクロールが出ないことを手動確認の項目に含める。出る場合は `.log-group-pane-table` の `overflow-x: hidden` か列幅の指定で対処する。 | Unresolved |
| R-11 | Minor | crates/local-sights-core/src/session.rs > `with_default_time_range`（末尾「Gate fix 16a」節のテスト 5 件） / code-summary.md 判断 33 | 夏時間で `00:00:00` が存在しない日（例：かつての America/Sao_Paulo）では、初期値の開始が `startNonexistentLocalTime` のエラーになると判断 33 に書かれている。この挙動は既存の `DateTimeInput::parse` の経路どおりで設計上は妥当だが、5 件のテストは夏時間の切り替え日として New York（`00:00:00` が存在する）しか扱っておらず、書かれた挙動が自動テストで固定されていない。起動直後に欄の下へ理由が出るので利用者には見える（隠れない）。 | `00:00:00` が存在しないゾーン（例：America/Sao_Paulo の 2018/11/04 付近）で、開始欄の `error()` が `NonexistentLocalTime` になり、終了欄は有効のままであるテストを 1 つ足す。足さない場合は、判断 33 の「テストなし」を一文で明記する。 | New |

> R-02 Reviewer note: NFR2 の 100 万行の測定は未実施のままで、Step 16 は触れていない。悪化はなし。承認時に SM1 の例外（ログ一覧は自前）を人が受け入れるか決め、測定は承認後の手元作業として記録先を決める必要が残る。

> R-04 Reviewer note: Step 16 の変更対象外。ハイフン入力の実機確認（R-07 手動確認）は未実施のまま。Step 16 の初期値は 4 欄に分かれて表示されるだけで、この件を悪化させていない。

> R-05 Reviewer note: TopBar は無変更。最小幅 1024px での上部ナビゲーションの確認は未実施。

> R-06 Reviewer note: Step 16 は JS・CSS の量を増やしていない（画面コードは無変更）。起動時間の体感確認は未実施。

> R-07 Reviewer note: 起動時の一覧取得の経路は無変更。Step 16 は `AppState::load` で同期的に初期値を入れるだけで、`SESSION_CHANGED` の取りこぼしの窓は変わらない。初期値は `getSession()` のスナップショットに含まれるため新しい取りこぼしは生じない。

> R-09 Reviewer note: PaneResizer と styles.css は Step 16 で無変更。実機のドラッグ確認待ち。

> R-10 Reviewer note: ペイン幅 20% での名前の省略は Step 16 で無変更。実機確認待ち。

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `cargo +stable test -p local-sights-core session` | PASS（87 件、失敗 0） | Step 16a の 5 件（ローカル暦日、UTC でないこと、月・年またぎ、夏時間の切り替え日 2 件、now なし・範囲外）を含めて通る。 |
| `cargo +stable test -p local-sights-core time_range` | PASS（20 件、失敗 0） | `date_in_zone`／`format_day_bounds` のテスト（10000 年で None、i64::MAX で None）を含めて通る。 |
| `cargo +stable clippy --workspace --all-targets` | PASS（警告 0） | `src-tauri/src/lib.rs` を含む。 |
| `cargo +stable fmt --all --check` | PASS | 差分なし。 |
| `vitest run src/App.test.tsx` | PASS（24 件） | 起動時の初期値が 4 欄に分かれて出て、画面が `updateInput` を送り返さないテストを含む。 |
| `tsc --noEmit` | PASS | 型エラーなし。 |
| `git status --short src-tauri/icons` | PASS | 期待どおり 6 ファイルの M と `source/` の ?? のみ。 |

### Summary

16a は、昨日・今日の判定を `with_timezone(&zone)` によるローカルゾーンの暦日で行い、`pred_opt` で月・年・うるう日をまたぎ、`parse` と `revalidate` を通すため入力経路が既存と一致する。`now` を与えない生成は従来どおり空のままで、テスト以外のコードに `unwrap`／`expect` はなく、`current_epoch_ms` は `Option` で返す。16b の SVG は 824×824、x=y=100、rx=185 でレビュー依頼の寸法と一致し、`render-icon.mjs` は四隅と各辺の余白の中央 4 点の透明を検証する。記録（判断 31〜37、逸脱、GATE-13／14 の target）は実装と一致し、実機確認は「未実施」と書かれている。新しい指摘は Minor 1 件（夏時間で `00:00:00` が存在しないゾーンのテスト欠落）だけで、前回の未解決 7 件はいずれも悪化していない。R-02（NFR2 未測定と SM1 の例外）は承認時に人が判断する必要が残る。禁止事項（秘密情報、新しい AWS API、テストからの AWS 接続、テレメトリ）に触れる変更はない。
