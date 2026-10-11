## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-10T16:03:04Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | src-tauri/tauri.conf.json > app.security.csp / devCsp | CSP に `font-src` がなく `default-src 'self'` が効く。一方 `dist/assets/*.css` の Cloudscape の Open Sans は `url(data:font/woff2;base64,...)` で埋め込まれている。WKWebView では data: のフォントが CSP で拒否され、フォントが読めずに既定フォントへ落ちる、またはコンソールに違反が出る可能性が高い。jsdom のテストでは検出できず、手動確認の表にも CSP の項目がない | `npm run tauri dev` と `tauri build` の実機で、フォント読み込みの CSP 違反が出ないか確認する。出るなら CSP に `font-src 'self' data:` を足す（秘密情報・外部通信は増えない）。手動確認の表と README のチェックリストに「CSP 違反なし」を加える | New |
| R-02 | Major | aidlc/spaces/default/intents/261010-console-ui-polish/construction/code-generation/code-summary.md > 性能測定（NFR2、R-03） / traceability.json NFR2 | NFR2 の 100 万行の測定が未実施（4 項目すべて「未測定」）で、traceability では Deferred。ログ一覧は Cloudscape 部品ではなく自前の仮想スクロールのまま（SM1 の例外）。判断は安全側で一貫し、例外も明示されているが、NFR2 が満たされたことの根拠はまだない。また試作の切り替えは Vite のビルド時環境変数のため、測定のたびに dev の再起動が要る | 承認時に SM1 の例外（ログ一覧は自前）を人が明示的に受け入れるか決める。測定は承認後の手元作業として期限と記録先（code-summary.md の表）を決めておく。測定が通った場合に `LOG_TABLE_VARIANT` の既定を切り替える手順をフォローアップにする | New |
| R-03 | Minor | src/dev/perfRows.ts > ROW_COUNT / GROWING / LOG_TABLE_VARIANT、src/api.ts（getRows・rowPositions・setFilter の分岐）、src/App.tsx > usePerfRows | 性能ハーネスは既定では不活性（環境変数が未設定なら `enabled=false`、`.env` ファイルもない）。ただしコードは本番バンドルに常に入り、`VITE_LOCAL_SIGHTS_PERF_ROWS` がシェルに残った状態で `tauri build` すると、配布物が生成行を表示する形で意図せず有効になりうる。ビルド時の歯止めがない | `import.meta.env.DEV` でも条件づけ、本番ビルドでは常に無効かつツリーシェイクで除去されるようにする。難しければ README に「本番ビルド前にこれらの変数を unset する」と書く | New |
| R-04 | Minor | crates/local-sights-core/src/time_range.rs、src/components/ConditionArea.test.tsx、code-summary.md > 計画からの逸脱（Step 8）、手動確認 R-07 | DatePicker にハイフン入力するとマスクが `/` に正規化する（jsdom での解析のみ）。このためコアのハイフン拒否（FR2.4）は画面の入力経路から到達しにくく、「ハイフンで日付欄の直下に理由」の画面テストは入力ではなくコアが持つ旧文字列で代替している。実機未確認。要件側の期待（拒否か自動の直しか）が code-summary に「どちらでもよい」と明記されていない | 実機で R-07 を確認し、結果を code-summary.md に追記する。ハイフンが自動で直る挙動を採用するなら、要件 FR2.4 との対応（拒否は貼り付けなどコアに文字列が届く経路で効く）を一文で明記する | New |
| R-05 | Minor | src/components/TopBar.tsx（custom content）、code-summary.md 判断 2 | `TopNavigation` の `utilities` に disabled がないため、`ButtonDropdown`／`Button` を custom content に置いた。計画 Step 6 の `menu-dropdown`／`button` からの逸脱だが記録され、R-05 の有効・無効の表は実装・テストされている。コンソールとの見た目・キーボード挙動（overflow での折りたたみなど）の差は実機では未確認で、最小幅 1024px で上部ナビゲーションが崩れないかは NFR7 の手動確認に含まれる | NFR7 の手動確認（1024×640）で、上部ナビゲーションの 4 つの操作部品が溢れず操作できることを確かめる | New |
| R-06 | Minor | code-summary.md > 配布物の大きさ | JS は 271 kB から 911 kB、CSS は 3.65 kB から 916 kB に増えた（`dist/` 約 1.8 MB。文言束とフォントの埋め込みが主因）。報告はされており、ローカルアプリで上限要件もないため問題ではない。起動時間への影響は実測がない | 必要になった時点で文言束の遅延読み込みを検討する。手動確認の起動時に体感の遅さがないか一言確認する | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| vitest run | PASS: 24 ファイル 210 テスト | code-summary の記録と一致 |
| tsc --noEmit | PASS（出力なし） | 型エラーなし |
| eslint . | PASS（出力なし） | 指摘なし |
| cargo +stable test -p local-sights-core | PASS（`FAILED`／`error` の行なし。時刻・日付入力の書式テストを含む） | コアの書式変更とスラッシュ化の回帰が通る |
| cargo +stable clippy -p local-sights-core --all-targets | PASS（警告なし） | 問題なし |
| cargo +stable fmt --all --check | PASS（終了コード 0） | 問題なし |
| traceability.json の target 存在確認 | PASS（欠落ファイルなし） | NFR2 と NFR7 の Deferred は理由が記載され、実機作業として妥当 |

### Summary

実装は計画どおりで、スイートは全て緑。コアの変更は書式のみで、コアと画面の境界は 1 欄 1 テキストのまま（`splitDateTime`／`joinDateTime`）、`dateTime.ts` の key と欄の表・上部ナビゲーションの有効・無効の表・時刻補完（`:00`／`:59`、空は `00:00:00`／`23:59:59`）も実装されている。主な懸念は、実機でしか見えない CSP と埋め込みフォントの食い違い（Major）と、未測定の NFR2 を人が SM1 の例外として承認するかの判断（Major）で、いずれも承認前後の手元確認で解消できる範囲のため READY とした。
