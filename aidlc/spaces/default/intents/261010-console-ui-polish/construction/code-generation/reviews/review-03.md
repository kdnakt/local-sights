## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-11T08:22:19Z
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
| R-09 | Minor | src/components/PaneResizer.tsx > containerWidthInPixels、src/styles.css > .app-body / .log-group-pane | ペインの幅（`flex-basis` の %）は `.app-body` の内容領域（左右の padding を除く）が基準だが、ドラッグの換算は親要素の `clientWidth`（padding を含む）で割っている。padding は `--ls-space-m` の左右ぶんで、窓幅 1200px なら約 3% の差になり、ドラッグ中にカーソルとハンドルが少しずつずれる。jsdom では padding が無いため、テストでは捕まらない。 | 換算の分母を `clientWidth` から左右の padding を引いた値にする（`getComputedStyle` の `paddingLeft`／`paddingRight`）。直さない場合は実機のドラッグ確認で許容範囲かを見て、code-summary.md に残す。 | New |
| R-10 | Minor | src/styles.css > .log-group-pane-table table、src/components/LogGroupPane.tsx > name 列の `.log-group-pane-name` | `min-width: 0` と `table-layout: fixed` と省略記号でペインが 20% まで縮む設計は、原因（`white-space: nowrap` の min-content 幅）に合っている。ただし Cloudscape の Table は選択列の幅やセルの内側の要素を自前の CSS で決めるため、実際の Tauri 画面で名前が切られるか（横スクロールが出ないか）は jsdom では確認できない。 | 実機で、最小の 20% まで縮めたときに名前が省略記号で切られ、横スクロールが出ないことを手動確認の項目に含める。出る場合は `.log-group-pane-table` の `overflow-x: hidden` か列幅の指定で対処する。 | New |

> R-02 Reviewer note: NFR2 の 100 万行の測定は依然として手元作業で、SM1 の例外（ログ一覧は自前）の受け入れも承認時の判断のまま。Step 15 では変化なし。

> R-04 Reviewer note: R-07 の実機確認（DatePicker のハイフン入力）は未実施。Step 15 で ConditionArea を変えたが、この点は変わらない。

> R-05 Reviewer note: 1024×640 での上部ナビゲーションの実機確認は未実施。

> R-06 Reviewer note: 起動時間の体感確認は未実施。

> R-07 Reviewer note: 起動直後の `SESSION_CHANGED` 取りこぼしの余地は残る（Step 15 は触れていない）。実機で認証なし／切断状態の起動を確認する。

> R-08 Reviewer note: code-summary.md に「GATE-* は修正依頼の ID、REV-* は計画レビューの指摘の ID で、要件 ID ではない」との説明が入った。GATE-7〜GATE-12 の追加も確認した。

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| vitest run | PASS: 27 files, 235 tests | 画面側の既存の動作と Step 15 のテストが通る |
| tsc --noEmit | PASS（出力なし） | 型の整合に問題なし |
| eslint . | PASS（出力なし） | リント指摘なし |
| cargo test -p local-sights-core | PASS（失敗・エラーの出力なし） | コア側に回帰なし |
| cargo clippy -p local-sights-core --all-targets | PASS（警告なしで完了） | コアに指摘なし |
| cargo fmt --all --check | PASS（出力なし） | 書式の差なし |
| sips icon.png | 1024x1024、hasAlpha: yes | アルファありの PNG。四隅の透明は render-icon.mjs が書き出し前に検証している（四隅 alpha 0、中央 alpha 255） |

### Summary

6 件の修正依頼（プロファイル名「デフォルト」、ペインの 20%〜50% と完全に閉じる帯、プロンプトの重複解消、ラベルの揃え、「メッセージを絞り込み」、四隅透明のアイコン生成）は Step 15 のとおり実装・検証されており、コアと UI の境界、認証情報・テレメトリ・新しい AWS API の使用に問題はない。残るのは実機でしか確かめられない小さな点（ドラッグの換算の padding、Cloudscape 表内の省略記号）と、承認時に人が決める NFR2 の例外（R-02）である。
