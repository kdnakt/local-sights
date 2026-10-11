## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-11T07:15:20Z
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
| R-07 | Minor | src-tauri/src/lib.rs > setup（`startup_listing` から `start_listing` を呼ぶ箇所）、src/App.tsx > 最初の `useEffect`（`onSessionChanged` の登録と `getSession` の呼び出し） | 起動時の一覧取得（14c）は、画面の読み込みより前に始まる。画面側は `getSession()` の取得と `onSessionChanged` の登録を並行して始めるため、取得が非常に速く終わった場合（例：認証失敗がすぐ返る）に、スナップショットの後かつリスナー登録の前に出た `SESSION_CHANGED` を取りこぼす余地がある。取りこぼすと一覧が Loading のままになりうる。窓は狭く、普通の API 往復では起きにくい | 実機で認証情報なし／ネットワーク切断の状態で起動し、一覧が Loading のまま残らないか確認する。残るなら、リスナーの登録が終わってから `getSession()` を呼ぶ（順に実行する）か、登録後にもう一度 `getSession()` で再同期する | New |
| R-08 | Minor | aidlc/spaces/default/intents/261010-console-ui-polish/construction/code-generation/traceability.json > GATE-1〜GATE-6、REV-R01、REV-R03 | これらは要求 ID ではなくゲートの修正依頼とレビュー指摘の ID である。`status` と `target` は他の要求と同じ形式で記録され、整合している（検証ツールの合格）。現状は要求 ID と区別がつかない | 許容できる。code-summary.md か traceability の説明に「GATE-*／REV-* はゲートの修正依頼・レビュー指摘の追跡 ID で、要求 ID ではない」と一文添えるとよい | New |

> R-01 Reviewer note: `tauri.conf.json` の `csp` と `devCsp` の両方に `font-src 'self' data:` が入った。外部通信先は増えていない。手動確認の表（code-summary.md）と README のチェックリストに「CSP 違反なし」が加わった。実機での違反ゼロの確認だけが残る（手動項目）

> R-02 Reviewer note: NFR2 の 100 万行の測定はまだ未実施で、traceability では Deferred のまま。ログ一覧は自前の仮想スクロールで、SM1 の例外を承認時に人が明示的に受け入れる必要がある。コードに変更はなく、判断は安全側で一貫している。計画の未チェックの箱（性能測定）が残っているのはこのため

> R-03 Reviewer note: `perfRows.ts` の `configuredPerfRowCount(env)` が `env.DEV` でなければ 0 を返す。本番ビルドでは `import.meta.env.DEV` が偽に置き換わるため、環境変数が残っていても生成行は出ない。テスト 4 件あり、vitest は全件通過

> R-04 Reviewer note: DatePicker へのハイフン入力の実機確認（R-07）が未了。計画の未チェックの箱に残っている。結果と FR2.4 との対応を code-summary.md に書く作業は実機確認後

> R-05 Reviewer note: 上部ナビゲーションの 4 つの操作部品が 1024×640 で溢れないかは実機未確認（手動項目）。実装の逸脱は記録済みで、コードの問題ではない

> R-06 Reviewer note: 配布物の増加（JS 約 911 kB、CSS 約 916 kB）に変化はなく、起動時間の体感確認は手動項目のまま。問題ではなく、確認の一言だけ

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| vitest run | PASS（26 ファイル、224 テスト） | 14a〜14i のテスト（ConditionArea、TopBar、PaneResizer、perfRows、messages、App）を含め全件通過 |
| tsc --noEmit | PASS（出力なし） | 型の問題なし |
| eslint . | PASS（出力なし） | リントの問題なし |
| cargo test -p local-sights-core | PASS（310 件ほか各結合テストが全件通過、5 件は無視指定） | 14c の起動時選択（テスト先行）と u2_log_group_listing の前提変更を含め通過 |
| cargo clippy -p local-sights-core --all-targets | PASS（警告なし） | 問題なし |
| cargo fmt --all --check | PASS（出力なし） | 整形済み |
| sips（source/icon.png） | 1024×1024 | 14i のソース画像が要件どおり存在 |

**6 件の依頼の実装確認**

- 14a：日付欄の幅を 234px にする CSS の修正が入り、ConditionArea のテストがある。見え方の合否は実機確認（手動項目）。
- 14b：`connection.profile.sdkDefault` は `SDK default`／`SDK デフォルト`。
- 14c：`select_startup_profile` がセッション生成時に SDK デフォルトを選び、`ConnectionState::new()` は変えていない。接続の変更として扱わないため `connection_generation` は 0 のまま、破棄される表示中のログもない。`startup_listing()` で一覧を `lib.rs` から実行する。コア（GUI に依存しない層）とアプリの境界は保たれている。前回の選択は読まない（Q2 の後半）ことと、表示中のログがあるときだけの変更確認は維持されている。
- 14d：`PaneResizer` と `usePaneWidth`（既定 14rem、最小 10rem、最大 40rem、1rem 刻み、`localStorage` 失敗時は既定値）。値の範囲は丸められる。
- 14h：`productName`、ウィンドウタイトル、`app.title` が `Local Sights`。`identifier` は変えていない。
- 14i：`source/icon.svg` と 1024×1024 の `icon.png` があり、アイコン一式が再生成されている。

**その他**

- 認証情報の書き込み、テレメトリ、新しい AWS API の呼び出しは、差分に見当たらない。`localStorage` に入るのはペイン幅の数値だけ。
- 計画の未チェックの箱は 2 つ（行 93 の実機確認、行 168 の性能測定）。3 つ目は合否の箱（行 173）で、性能測定に従属する。いずれも手動項目だけで、自動化できる作業の取りこぼしはない。

### Summary

6 件の依頼はいずれも計画どおりに実装され、自動検証は全件通過した。R-01 と R-03 は解消。R-02（NFR2 の未測定と、ログ一覧が自前の仮想スクロールである SM1 の例外）は承認時に人が受け入れるかを決める必要がある。残りは実機確認の手動項目で、新規は起動時一覧の取りこぼしの余地と ID 区分の注記のみで、いずれも Minor。
