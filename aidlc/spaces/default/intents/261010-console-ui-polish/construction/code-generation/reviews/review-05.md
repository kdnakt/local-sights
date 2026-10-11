## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-11T09:58:35Z
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
| R-11 | Minor | crates/local-sights-core/src/session.rs > `with_default_time_range`（末尾「Gate fix 16a」節のテスト 5 件） / code-summary.md 判断 33 | 夏時間で `00:00:00` が存在しない日（例：かつての America/Sao_Paulo）では、初期値の開始が `startNonexistentLocalTime` のエラーになると判断 33 に書かれている。この挙動は既存の `DateTimeInput::parse` の経路どおりで設計上は妥当だが、5 件のテストは夏時間の切り替え日として New York（`00:00:00` が存在する）しか扱っておらず、書かれた挙動が自動テストで固定されていない。起動直後に欄の下へ理由が出るので利用者には見える（隠れない）。 | `00:00:00` が存在しないゾーン（例：America/Sao_Paulo の 2018/11/04 付近）で、開始欄の `error()` が `NonexistentLocalTime` になり、終了欄は有効のままであるテストを 1 つ足す。足さない場合は、判断 33 の「テストなし」を一文で明記する。 | Unresolved |
| R-12 | Minor | src/styles.css > `.condition-area-frame` / `.condition-area`、aidlc/spaces/default/intents/261010-console-ui-polish/construction/code-generation/code-summary.md > 判断 46 | 枠の `height: 100%` は、親の `content-inner`（flex: 1 の項目）の「確定した高さ」に依存している。この解決は WebKit（WKWebView）で効く想定だが、jsdom は高さを計算しないため自動テストでは確かめられていない。効かない場合は枠が `min-height: 15rem` の 240px に潰れ、ログ一覧がほぼ見えなくなる（ステータス行自体は枠内に残るので「常に見える」要件は満たすが、一覧が使えない）。また、取得エラーのアラートが複数出る、条件行が折り返す、のどちらでも `.condition-area` 内でアラートや条件行は縮まない（flex-shrink が効かない）ので、合計が枠を超えると一覧が 0 になり、超えた分は列の外にはみ出して `content` のスクロールに回る。判断 46 の「約 470px 残る」は見込みで、アラート複数時の内訳（条件行＋アラート＋ステータス行）は書かれていない。 | 実機確認の表（Step 18c、すでに「未実施」と記録済み）に、(1) 1024×640 で 1,000 行以上を表示、(2) 取得エラーのアラート表示中、(3) 条件行が 2 段に折り返す幅、の 3 条件でステータス行とログ一覧の両方が見えることを明記する。確認で枠が潰れるようなら、アラート領域に `max-height` と `overflow: auto` を付けるか、枠の `min-height` を見直す。 | New |

> R-02 Reviewer note: NFR2 の 100 万行測定は未実施のまま（code-summary.md「性能測定」節に未実施と明記。SM1 の例外も承認時確認として残っている）。Step 17・18 は仮想スクロールの計測の仕組みを変えておらず、悪化なし。Step 18 は `.log-table-viewport` の高さを窓に合わせるだけで、実機測定は引き続き必要。

> R-04 Reviewer note: DatePicker のハイフン正規化は実機未確認のまま。Step 17 は日付欄の値の扱いを変えておらず、悪化なし。

> R-05 Reviewer note: TopBar の custom content を 1024px で確かめる実機確認は未実施のまま。Step 17・18 は TopBar に触れていない。

> R-06 Reviewer note: 配布物の増加。Step 17・18 は依存を足していない（Rust・依存に変更なし）ので悪化なし。

> R-07 Reviewer note: 起動時の SESSION_CHANGED 取りこぼしの窓。Step 17・18 は無関係で悪化なし。

> R-09 Reviewer note: PaneResizer の換算が padding を含む件。Step 18 の枠は右ペインの内側の変更で、PaneResizer には触れておらず悪化なし。

> R-10 Reviewer note: 20% での名前省略の実機確認は未実施のまま。Step 17・18 は無関係で悪化なし。

> R-11 Reviewer note: 00:00:00 が存在しないゾーンの初期値のテストなし。Step 17 の `todayInZone` はその初期値を扱わず無関係で悪化なし。

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| vitest（dateTime.test.ts、ConditionArea.test.tsx、App.test.tsx） | PASS：3 ファイル 74 テスト | 判断 43 の記録（dateTime 27、ConditionArea 22）と整合。TZ 固定（Asia/Tokyo）は `vi.stubEnv` で、実行環境の TZ に依存しない。 |
| tsc --noEmit | PASS（出力なし） | 型エラーなし。 |
| eslint src | PASS（出力なし） | 警告・エラーなし。 |

**観点ごとの確認結果**

- 今日の基準：`todayInZone(Date.now(), session.timeZone)` は Local で OS ゾーン、Utc で UTC の暦日。セルは Cloudscape がローカル時刻の `Date` で渡すため、`todayInZone(date.getTime(), "Local")` で暦日になり、`yyyy/mm/dd` 文字列どうしの比較で正しい（0 埋め固定幅、`isValidDateText` で不正値は制限しない）。
- 日付が変わったとき：`Date.now()` を呼び出しごとに読むので、開いたまま日付が変わっても次にカレンダーを描くときに新しい今日になる。判断 39 の記録と実装が一致。
- 判断 42（今日の強調はローカルのまま）は UTC 表示で深夜 0 時前後に 1 日ずれるが、手動確認の表に明記済みで、隠してはいない。
- 手入力の未来日は止めない（承認済み前提）。実装と判断 38 が一致。
- 記録の正直さ：Step 1・14a・16c・17c・18c の実機確認はいずれも「未実施」と書かれ、「合格」とは書かれていない。計画 17b の `i18nStrings` からの逸脱も判断 41 と逸脱表に記録されている。

### Summary

Step 17 は表示ゾーンの今日の判定、DatePicker が渡す Date の解釈、日付変更時の挙動、テストの TZ 固定のいずれも妥当で、実装と記録も一致している。Step 18 の absolute 重ね方式は Cloudscape の CSS から筋が通っているが、WKWebView での高さの解決は未確認なので、実機確認の条件を具体化することを Minor として勧める。前回までの指摘は Step 17・18 で悪化していない。
