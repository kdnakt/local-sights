# Code Summary — 画面を AWS マネジメントコンソールに寄せる

計画：`code-generation-plan.md`（ゼロ Unit）。テストの順序は Testing Contract のとおり、純粋なロジック（コアの書式、`src/dateTime.ts`）はテストを先に書き、画面の部品は実装してからテストを書いた。

## 変更したファイル

### コア（Rust、`crates/local-sights-core/`）

| ファイル | 変更 |
|---|---|
| `src/time_range.rs` | `INPUT_FORMAT` を `%Y/%m/%d %H:%M:%S`、`DISPLAY_FORMAT` を `%Y/%m/%d %H:%M:%S%.3f` に。`has_input_shape` の区切りを `/` に（ハイフンは `Format` エラー、FR2.4）。エラー文言とドキュメントを `yyyy/mm/dd hh:mm:ss` に。テストをスラッシュに書き換え、ハイフン拒否と「書いた文字列にハイフンがない」の 2 テストを追加 |
| `src/date_input.rs` | テストをスラッシュに書き換え、ハイフン入力が `Format` になるテストを追加 |
| `src/session.rs`、`src/request.rs`、`src/coordinator.rs`、`src/streams/mod.rs`、`examples/fetch_check.rs`、`tests/u1_fetch_flow.rs`、`tests/u3_fetch_flow.rs`、`tests/u5_filter_flow.rs`、`tests/u6_cache_flow.rs` | 旧書式の文字列（テスト・ドキュメント・`fetch_check` の説明文）をスラッシュに揃えた。ロジックの変更はない |

### 画面（TypeScript/React、`src/`）

| ファイル | 変更 |
|---|---|
| `package.json`、`package-lock.json` | `@cloudscape-design/components` 3.0.1396、`@cloudscape-design/global-styles` 1.0.71、`@cloudscape-design/design-tokens` 3.0.114 を追加（いずれも Apache-2.0）。`npm audit --audit-level=high`：0 件 |
| `main.tsx` | Cloudscape のグローバルスタイルを読み込み、`I18nProvider`（`all.en`／`all.ja` を `Locale` で選ぶ、FR4.6）で `App` を包み、`applyDesignTokens(document.body)` を呼ぶ |
| `designTokens.ts`（新規） | 自前描画が残る部分（ログ一覧・レイアウト）が読む `--ls-*` CSS 変数を Cloudscape のデザイントークンから `body` に設定する。独自の色は持たない |
| `hooks/useColorScheme.ts`（新規） | `prefers-color-scheme` の初期値と変化で `applyMode(Mode.Light|Dark)` を呼ぶ（FR4.5） |
| `hooks/useInitialFocus.ts`（新規） | Cloudscape の Modal がポータルで 1 コミット遅れて中身を描くため、部品が現れたコミットの後で一度だけ焦点を当てる callback ref |
| `styles.css` | システムカラーの規則を撤去。レイアウト（2 ペイン、最小幅 1024px、条件行の折り返し）と、ログ一覧の自前描画をデザイントークンの CSS 変数で書き直した |
| `dateTime.ts`（新規） | 純粋ロジック：`splitDateTime`／`joinDateTime`／`isoToSlashed`／`slashedToIso`、`completeTime`（開始 `00`・終了 `59` で補完）、`isMaskCompletionOf`、`isValidDateText`／`isValidTimeText`、`fieldOfValidationKey`（key → 欄の表）、`fieldReasons`（画面側の形式判定を優先、R-02）、`completionWouldEnableFetch`（R-01） |
| `components/TopBar.tsx`（新規） | 黒い `TopNavigation`。`ConnectionBar.tsx`・`TimeZoneToggle.tsx`（とテスト）を置き換え、削除 |
| `components/LogGroupPane.tsx` | `Container` + `Header` + `TextFilter` + `Table`（1 行選択、embedded）+ `Button`。一部失敗は表の上の `Alert type="warning"` |
| `components/ConditionArea.tsx`（新規） | `FetchForm.tsx`（とテスト）を置き換え、削除。`Container` + `Header`（選択中ロググループ、主ボタン [取得]）、絞り込み欄、開始・終了の `DatePicker` + `TimeInput`。欄ごとの理由、時刻の補完、Enter／[取得] の扱い。エラー表示・ログ一覧・ステータス行を `children` として同じ囲みに入れる |
| `components/LogFilterInput.tsx` | `TextFilter` に置き換え（文言 "Filter events"）。Enter で即時反映の振る舞いは維持。焦点を受け取る `inputRef` を追加（R-06） |
| `components/FetchErrorBanner.tsx` | `Alert type="error" dismissible`（見出し＝何が起きたか、本文＝次の行動・詳細）。閉じると同じジョブの間は出ない |
| `components/FailureList.tsx` | `Modal` + `Table`（列：ストリーム、何が起きたか／次の行動）+ footer の [閉じる] |
| `components/StatusLine.tsx` | `StatusIndicator`（loading／error／success／stopped／warning）+ `Button variant="inline-link"` |
| `components/ConfirmDialog.tsx`、`CloseConfirmDialog.tsx`、`SettingsDialog.tsx` | `Modal`（footer は `SpaceBetween` で右寄せ、主ボタンは右端）、`Checkbox`、`Button`。焦点の戻り・Escape・[Clear cache] の即時反映は維持 |
| `components/LogTable.tsx` | 空の状態の文言 `emptyText` を表の中に出す（FR2.2 の時刻列はコアの文字列のまま） |
| `components/LogTableCloudscape.tsx`（新規） | Cloudscape `Table`（expandableRows）で作り直した試作。`VITE_LOCAL_SIGHTS_LOG_TABLE=cloudscape` のときだけ `App` が使う |
| `dev/perfRows.ts`（新規）、`api.ts` | 性能測定の開発者向けハーネス。`VITE_LOCAL_SIGHTS_PERF_ROWS=<n>` のとき `getRows`／`rowPositions`／絞り込みをコアを通さず生成行で答える。`VITE_LOCAL_SIGHTS_PERF_GROW=1` で 10 万行ずつ増やす。既定では何もしない |
| `App.tsx` | `TopBar`、`ConditionArea`、`useColorScheme`、コマンドエラーの `Alert`、表の切り替えとハーネスの接続 |
| `i18n/messages.ts` | interaction-spec §9 の文言を追加・変更（下記）。`form.reasons.title`・`form.dateTime.placeholder` を削除。`error.what.*`／`error.next.*` は変えていない |
| `README.md` | Cloudscape・`yyyy/mm/dd`・カレンダー入力・時刻の補完・欄ごとの理由を記述。手動確認の項目と「Performance check of the log list」を追加 |

### Step 14（ゲートの修正依頼）で変更したファイル

| ファイル | 変更 |
|---|---|
| `src/styles.css` | 14a：`.condition-date` の幅を `11em` から `234px`（Cloudscape の DatePicker 自身の最大幅）に。14d：`.log-group-pane` の固定幅 `18rem` を `var(--ls-log-group-pane-width, 14rem)` に、つまみ `.pane-resizer` の規則を追加 |
| `src/components/ConditionArea.tsx` | 14a：カレンダーの幅の根拠をドキュメントコメントに記載（コードの変更なし） |
| `src/components/ConditionArea.test.tsx` | 14a：カレンダーが 7 列と月送りボタン 2 つを持ち、欄の外（ポータル）に開き、`.condition-date` の幅が DatePicker の `max-inline-size` と一致することを確かめるテストを 1 つ追加（両 CSS を `?raw` で読む） |
| `vitest.config.ts` | 14a：上の 2 つの `?raw` 読み込みだけ `test.css.include` で中身を返す（ほかの CSS は従来どおり空のモジュール） |
| `src/i18n/messages.ts` | 14b：`connection.profile.sdkDefault` を「SDK default」／「SDK デフォルト」に。14d：`pane.resizer`（つまみの aria-label）を追加。14h：`app.title` を `Local Sights` に |
| `src/i18n/messages.test.ts` | 14b・14d・14h の文言のテストを 2 つ追加 |
| `src/components/TopBar.tsx`、`TopBar.test.tsx` | 14b：文言の更新と「Profile: SDK default」のテストを 1 つ追加。14c：起動時の前提をドキュメントコメントに記載。14h：h1 の期待値を `Local Sights` に |
| `crates/local-sights-core/src/session.rs` | 14c：`with_catalog_and_time_zones` が `select_startup_profile()` を呼び、カタログに SDK デフォルトがあればそれを選び、既定リージョンがあれば一覧（Loading）を始める。アプリが走らせるための `startup_listing()` を追加。テスト：起動の 4 テストを新規、`fill_valid`・`unselected_reasons_…`・`profile_without_default_region_…`・`every_reason_is_listed_…` を新しい前提に合わせた |
| `crates/local-sights-core/tests/u2_log_group_listing.rs` | 14c：`AppSession` を包む `SessionSink` で、起動時の一覧がフェイクゲートウェイを通って（プロファイル名なし・リージョン付きで）ロググループを埋めることを確かめる統合テストを 1 つ追加 |
| `src-tauri/src/lib.rs` | 14c：`setup` で `startup_listing()` があれば `start_listing` を呼ぶ（通常の一覧と同じ経路） |
| `src/App.tsx`、`src/App.test.tsx` | 14c：起動時に画面が何も送らずコアの選択を映すテストを 1 つ追加。14d：`usePaneWidth` で幅を持ち `--ls-log-group-pane-width` を `.app-body` に設定、左ペインと右ペインの間に `PaneResizer` を置く。Tab 順のテストに「再読み込み → つまみ → 取得」を反映 |
| `src/hooks/usePaneWidth.ts`（新規） | 14d：既定 14rem・最小 10rem・最大 40rem・1rem 刻みの定数、`localStorage`（`local-sights.logGroupPaneWidth`）の読み書き（使えない環境では既定値）、`usePaneWidth` フック |
| `src/components/PaneResizer.tsx`（新規） | 14d：`role="separator"`・`aria-orientation="vertical"`・`aria-valuenow/min/max` のつまみ。ポインターのドラッグ（`setPointerCapture`）、←／→ で 1rem、Home／End で最小／最大 |
| `src/components/PaneResizer.test.tsx`（新規） | 14d：5 テスト（初期値、ドラッグ、矢印キー、最小・最大、`localStorage` の保存・復元と使えない環境） |
| `src-tauri/tauri.conf.json` | 14e：`csp`・`devCsp` に `font-src 'self' data:` を追加。14h：`productName` と `app.windows[0].title` を `Local Sights` に |
| `src/dev/perfRows.ts`、`src/dev/perfRows.test.ts`（新規） | 14f：行数の判定を `configuredPerfRowCount(env)` に切り出し、`env.DEV` が偽なら常に 0（本番ビルドでは無効）。テスト 4（本番で無効、変数なし・不正値・正常値、テスト実行中は無効、生成行の決定性） |
| `index.html` | 14h：`<title>` を `Local Sights` に |
| `src-tauri/icons/source/icon.svg`（新規）、`source/icon.png`（新規、1024×1024）、`icons/32x32.png`・`128x128.png`・`128x128@2x.png`・`icon.icns`・`icon.ico`・`icon.png`（作り直し） | 14i：濃紺の角丸タイルにログ行（1 行は Cloudscape の青 `#0972d3`）と虫眼鏡。`qlmanage -t -s 1024` で PNG に描き出し（`sips` で 1024×1024・アルファありを確認）、`tauri icon` で生成した一式のうち、リポジトリに元からある 6 ファイルだけを置き換えた |
| `README.md` | 14a〜14i：起動時の選択、ペインの幅、アプリ名、チェックリスト 4 項目（カレンダーの 7 列、CSP 違反なし、アプリ名とアイコン、つまみ）、性能ハーネスが本番ビルドで無効の一文 |

### Step 15（ゲートの修正依頼 2 回目）で変更したファイル

| ファイル | 変更 |
|---|---|
| `src/i18n/messages.ts` | 15a：`connection.profile.sdkDefault` を「Default」／「デフォルト」に。15b：`pane.collapse`（「Close the log group list」／「ロググループ一覧を閉じる」）と `pane.expand`（「Open the log group list」／「ロググループ一覧を開く」）を追加。15c：`form.selectedLogGroup.none` を「Log group: none」／「ロググループ: 未選択」に。15e：`logFilter.label`／`placeholder` を「Filter messages」／「メッセージを絞り込み」に |
| `src/i18n/messages.test.ts` | 15a・15b・15c・15e の期待を更新（見出しの文言と理由の文言が両言語で一致しないことも確かめる） |
| `src/hooks/usePaneWidth.ts` | 15b：単位を rem から「アプリ本体の幅に対する %」に（既定 25、最小 20、最大 50、刻み 1）。`localStorage` のキーを `local-sights.logGroupPaneWidthPercent` に変え、旧キーは読まない。閉じた状態（`local-sights.logGroupPaneCollapsed`）の読み書きと `collapsed`／`setCollapsed` を足し、戻り値を `{ width, collapsed, setWidth, setCollapsed }` に |
| `src/hooks/usePaneWidth.test.ts`（新規） | 15b：5 テスト（% の既定・最小・最大、保存と復元・旧キーを読まない、閉じた状態の保存、`localStorage` が使えない環境） |
| `src/components/PaneResizer.tsx` | 15b：ドラッグの px を親（`.app-body`）の `clientWidth`（なければ `window.innerWidth`）で % に換算。矢印キーは 1%、Home／End は 20%／50%。`aria-valuenow/min/max` は %、`aria-valuetext` は `25%` の形 |
| `src/components/PaneResizer.test.tsx` | 15b：% 刻みに書き換え、`window.innerWidth` への退避のテストを 1 つ追加（6） |
| `src/components/LogGroupPane.tsx` | 15b：`onCollapse`／`collapseButtonRef` を追加し、見出しの `actions` に `Button iconName="angle-left" variant="icon"`（`pane.collapse`）を置く |
| `src/components/LogGroupPane.test.tsx` | 15b：閉じるボタンは `onCollapse` があるときだけ出て、見出しと検索欄の間にあり、押すと呼ばれることを確かめるテストを 1 つ追加（12） |
| `src/App.tsx` | 15b：`usePaneWidth` の新しい戻り値を使い、`--ls-log-group-pane-width` を `%` で設定。閉じているときは `LogGroupPane` と `PaneResizer` を出さず、左端に `.log-group-pane-collapsed`（`angle-right` の `Button`、`pane.expand`）だけを出す。閉じたら開くボタンへ、開いたら閉じるボタンへ焦点を移す |
| `src/App.test.tsx` | 15a：「Profile: Default」。15b：「閉じる → 帯だけ → 開く → 元の幅」「閉じた状態が復元され、Tab は歯車 → 開くボタン → 取得」の 2 テストを追加し、Tab 順の期待に閉じるボタンを足し、`beforeEach` で `localStorage` を消す。15e：絞り込み欄の名前（23） |
| `src/styles.css` | 15b：`.log-group-pane` に `min-width: 0; overflow: hidden`、既定幅を `25%` に。`.log-group-pane-table table { table-layout: fixed }` で名前の列をペイン幅に収めて省略（…）。`.log-group-pane-collapsed`（幅 36px の縦の帯）を追加 |
| `src/components/ConditionArea.tsx` | 15c：見出しの未選択時の文言（コメント）。15d：絞り込み欄を `data-testid="fetch-form-filter"` で包み、上に `Box variant="awsui-key-label"`（`logFilter.label`）を置く |
| `src/components/ConditionArea.test.tsx` | 15c：見出しの期待を「Log group: none」／「ロググループ: 未選択」に変え、「Choose a log group. が 1 か所にしか出ない」テストを追加。15d：ラベルが開始・終了のラベルと同じ Cloudscape の variant で欄の前にあるテストを追加。15e：絞り込み欄の名前（18） |
| `src/components/LogFilterInput.tsx`、`LogFilterInput.test.tsx` | 15e：文言「Filter messages」／「メッセージを絞り込み」（コメントとテスト） |
| `src/components/TopBar.test.tsx` | 15a：「Default」「Profile: Default」 |
| `package.json`、`package-lock.json` | 15f：`@resvg/resvg-js` 2.6.2 を devDependency に追加（MPL-2.0）。スクリプト `icon:render` を追加。`npm audit --audit-level=high`：0 件 |
| `scripts/render-icon.mjs`（新規） | 15f：`icon.svg` を resvg で 1024×1024・背景透明に描き出し、四隅のアルファが 0・中央のアルファが 255 であることを確かめてから `source/icon.png` に書く。満たさなければ終了コード 1 |
| `src-tauri/icons/source/icon.png`、`icons/32x32.png`・`128x128.png`・`128x128@2x.png`・`icon.icns`・`icon.ico`・`icon.png` | 15f：透明な四隅で作り直し（`tauri icon` の生成物のうち `tauri.conf.json` が参照する 6 ファイルだけ、14i と同じ） |
| `README.md` | 15a〜15f：「Default」、ペインの %・閉じる／開く、Tab 順、見出し「Log group: none」と絞り込み欄「Filter messages」とラベルの揃え、「App icon」の節（`icon:render` と `tauri icon` の手順） |

### Step 16（ゲートの修正依頼 3 回目）で変更したファイル

| ファイル | 変更 |
|---|---|
| `crates/local-sights-core/src/time_range.rs` | 16a：`pub(crate) fn date_in_zone(instant, zone) -> Option<NaiveDate>`（そのゾーンでの暦日）と `pub(crate) fn format_day_bounds(date) -> Option<(String, String)>`（その日の `00:00:00` と `23:59:59` を `INPUT_FORMAT` で書く。年が 4 桁でなければ `None`）を追加。テスト 1（`day_bounds_are_written_in_the_input_format_for_the_zone_s_calendar_day`） |
| `crates/local-sights-core/src/session.rs` | 16a：ビルダー `with_default_time_range(now_ms: i64) -> Self` を追加：ローカルゾーンでの `now_ms` の暦日を今日、`NaiveDate::pred_opt()` を昨日とし、`format_day_bounds` の文字列を `DateTimeInput::parse` に通して `revalidate()`。既存の生成関数は変えていない。テスト 5（末尾「Gate fix 16a」節） |
| `src-tauri/src/lib.rs` | 16a：`current_epoch_ms() -> Option<i64>` を追加し、`AppState::load` で `Some` のときだけ `with_default_time_range` を呼ぶ（`unwrap`／`expect` なし、取れないときは空のまま） |
| `src/App.test.tsx` | 16a：起動時にコアの初期値が開始・終了の日付欄と時刻欄に分かれて出て、画面が `updateInput` を送り返さないテストを 1 つ追加（24）。画面のコードは変更なし |
| `src-tauri/icons/source/icon.svg` | 16b：板を `x=100 y=100 width=824 height=824 rx=185 ry=185` に縮め、中の図形（枠線・ログ行・虫眼鏡）を `<g transform="translate(100 100) scale(0.8046875)">` で包んだ。コメントに寸法の規則を記載 |
| `scripts/render-icon.mjs` | 16b：余白の帯の透明の検証（左右上下の各辺の余白の中央、`(50, 512)`・`(973, 512)`・`(512, 50)`・`(512, 973)`）を追加。定数 `MARGIN = 100`。コメントを更新 |
| `src-tauri/icons/source/icon.png`、`icons/32x32.png`・`128x128.png`・`128x128@2x.png`・`icon.icns`・`icon.ico`・`icon.png` | 16b：`npm run icon:render`（検証を通過）→ `tauri icon`（scratch に出力し、`tauri.conf.json` が参照する 6 ファイルだけコピー）で作り直し。`sips` で `source/icon.png` 1024×1024・`icon.png` 512×512・アルファありを確認 |
| `README.md` | 16a・16b：起動時の日付の初期値の説明（本文）とチェックリスト 2 項目、「App icon」の寸法（1024 のうち中央 824、余白 100、角 185）と検証内容 |

### Step 17（ゲートの修正依頼 4 回目）で変更したファイル

| ファイル | 変更 |
|---|---|
| `src/dateTime.ts` | 17a：`todayInZone(nowMs, zone: TimeZoneChoice): string`（`"Local"` は `getFullYear`／`getMonth`／`getDate`、`"Utc"` は `getUTC*` で `yyyy/mm/dd` を組む）と `isFutureDate(slashed, today): boolean`（両方が `isValidDateText` を通るときだけ文字列の大小で比べる。不正な文字列は未来と見なさない）を追加。`TimeZoneChoice` は `./api` から型だけ読み込む |
| `src/dateTime.test.ts` | 17a：`todayInZone / isFutureDate (gate fix 17a)` 節にテスト 5（Local と UTC で日付がずれる時刻、月・年またぎ、今日と過去は未来でない、明日以降は未来、不正な文字列は未来でない）。`vi.stubEnv("TZ", "Asia/Tokyo")` で Local を固定 |
| `src/components/ConditionArea.tsx` | 17b：開始・終了の両方の `DatePicker` に `isDateEnabled` を渡す。セルの `Date`（ローカルの 0 時）を `todayInZone(date.getTime(), "Local")` で暦日にし、`todayInZone(Date.now(), session.timeZone)` より後なら無効。部品のコメントに「手入力の未来日は対象外」を記載 |
| `src/components/ConditionArea.test.tsx` | 17b：`ConditionArea calendar limit (gate fix 17b)` 節にテスト 3（開始のカレンダーで明日以降が `aria-disabled="true"`・今日が有効で押せる、終了側も同じ、UTC 表示では UTC の今日が基準）。`vi.useFakeTimers({ toFake: ["Date"] })`・`vi.setSystemTime`・`vi.stubEnv("TZ", "Asia/Tokyo")` |
| `README.md` | 17c：本文に「カレンダーは表示中のタイムゾーンの今日より後を出さない、手入力は制限しない」、チェックリストに 1 項目 |

### Step 18（ゲートの修正依頼への追加：ステータス行を常に見える位置に）で変更したファイル

| ファイル | 変更 |
|---|---|
| `src/components/ConditionArea.tsx` | 18a：`Container` の中身を `div.condition-area-frame`（`data-testid="condition-area-frame"`）で包み、その中の `div.condition-area`（`data-testid="condition-area"`）に条件行・理由・子（アラート、ログ一覧、ステータス行）を置く。部品のコメントに原因と対処を記載 |
| `src/styles.css` | 18a：`.condition-area-frame { position: relative; height: 100%; min-height: 15rem }`、`.condition-area` を `height: 100%` から `position: absolute; inset: 0`（flex の列、`min-height: 0` は維持）に変更。`.status-line` に `flex: 0 0 auto`。開発用の試作 `.log-table-cloudscape` に `flex: 1; min-height: 0; overflow: auto`。原因をコメントに記載 |
| `src/App.test.tsx` | 18b：テスト 1（ログ一覧とステータス行が同じ列 `condition-area` の子で、ステータス行が最後の子。取得エラーのアラートが出ても同じ列の中で、ステータス行は最後のまま） |
| `src/components/ConditionArea.test.tsx` | 18b：テスト 1（子が枠の中の列の最後に入る。スタイルシートの `.condition-area-frame`・`.condition-area`・`.log-table`・`.status-line` の規則を `?raw` で読んで、縮める指定と縮まない指定を確かめる） |
| `README.md` | 18b：チェックリストに 1 項目（1,000 行以上・取得中・1024×640・エラーのアラート表示中・窓の高さを変えたときもステータス行が見え、ログ一覧だけがスクロールする） |

### テスト

Step 18 で追加：`App.test.tsx` +1（25）、`ConditionArea.test.tsx` +1（22）。

Step 17 で追加：`dateTime.test.ts` +5（27）、`ConditionArea.test.tsx` +3（21）。

Step 16 で追加：コア `session` +5（87）、`time_range` +1（20）、`App.test.tsx` +1（24）。

Step 15 で追加：`hooks/usePaneWidth.test.ts`（5、新規）、`PaneResizer.test.tsx` +1（6）、`LogGroupPane.test.tsx` +1（12）、`ConditionArea.test.tsx` +2（18）、`App.test.tsx` +2（23）。更新のみ：`LogFilterInput.test.tsx`（7）、`TopBar.test.tsx`（10）、`i18n/messages.test.ts`（19）。

Step 14 で追加：`ConditionArea.test.tsx` +1（16）、`TopBar.test.tsx` +1（10）、`i18n/messages.test.ts` +2（19）、`App.test.tsx` +1（21）、`components/PaneResizer.test.tsx`（5、新規）、`dev/perfRows.test.ts`（4、新規）、コア `session` +3（起動の 4 テスト新規、旧 `starts_with_nothing_selected_and_no_list` を置き換え）、`tests/u2_log_group_listing.rs` +1。

新規：`dateTime.test.ts`（22）、`hooks/useColorScheme.test.ts`（3）、`designTokens.test.ts`（2）、`components/TopBar.test.tsx`（9）、`components/ConditionArea.test.tsx`（15）、`components/LogTableCloudscape.test.tsx`（3）。更新：`LogGroupPane.test.tsx`（11）、`LogFilterInput.test.tsx`（7）、`FetchErrorBanner.test.tsx`（5）、`FailureList.test.tsx`（5）、`StatusLine.test.tsx`（16）、`ConfirmDialog.test.tsx`（5）、`CloseConfirmDialog.test.tsx`（4）、`SettingsDialog.test.tsx`（7）、`LogTable.test.tsx`（18、空の状態 2 件を追加）、`i18n/messages.test.ts`（17）、`App.test.tsx`（20）。

## 主な判断

1. **Red → Green の記録（Testing Contract の先行テスト）**
   - Step 3（コア）：テストをスラッシュに書き換えた直後の `cargo test -p local-sights-core time_range`：`FAILED. 5 passed; 14 failed`、`date_input`：`FAILED. 2 passed; 7 failed`。`INPUT_FORMAT`／`DISPLAY_FORMAT`／`has_input_shape`／文言を変えた後：`time_range` 19 passed、`date_input` 9 passed。
   - Step 4（画面側の純粋ロジック）：`npx vitest run src/dateTime.test.ts` は `Failed to resolve import "./dateTime"`（モジュール未作成）で落ちることを確認してから実装し、22 passed。
2. **上部ナビゲーションの中身は `ButtonDropdown` と `Button`**：`TopNavigation` の `utilities`（`menu-dropdown`／`button`）には `disabled` がなく、R-05 の有効・無効の表を満たせない。`TopNavigation` の custom content（`visualContext="top-navigation"`）の中に `ButtonDropdown` × 3 と歯車の `Button` を置いた。見た目は黒いバーで同じ、無効化は確実に動く。
3. **理由の割り当て（R-02）**：`fieldReasons` が日付欄・時刻欄の形式を先に判定し、その側のコアの `startFormat`／`endFormat` は隠す（画面がすでに理由を言っている）。日付が有効で時刻が空のときも隠す（補完で直る）。それ以外のコアの理由は表どおりの欄に出し、欄ごとに最初の 1 つだけ出す。選択・接続の理由は見出し行の直下に `StatusIndicator type="error"` で出す。
4. **空の時刻と [取得]（R-01、FR1.4／AS1 の置き換え）**：空の時刻は不正ではなく補う。コアが `canFetch=false` でも、理由が「有効な日付＋空の時刻」だけなら [取得] を押せる（`completionWouldEnableFetch`）。押下／Enter では先に `completeTime` を両方に適用してコアへ送り、コアが補完後の文字列を持ち `canFetch` になった時点で `onFetch` を呼ぶ（`pendingFetchRef`）。日付が空・不正なときは補完しない（画面案 画面 2「日付が空のときは補完しない」）。
5. **終了の `:59`（Q5）と Cloudscape の時刻マスク**：`TimeInput` は焦点が外れたとき・Enter のときに欠けた部分を `00` で自前補完して `onChange` を出す。`isMaskCompletionOf(前の値, 新しい値)` でそれを見分け、終了側は `completeTime(前の値, "end")`（`:59`）に置き換える。
6. **DatePicker の値**：Cloudscape の `DatePicker` は内部値が ISO（`yyyy-mm-dd`）、表示は `slashed`。画面側は `isoToSlashed`／`slashedToIso` で変換し、コアには常に `yyyy/mm/dd hh:mm:ss` を渡す（interaction-spec §8 の境界は変えていない）。`expandToViewport` でカレンダーを囲みの外に出す。
7. **ログ一覧（Step 10）**：測定を実行できないため、計画の安全側どおり既存の `LogTable.tsx`（自前の仮想スクロール）を既定のまま保ち、見出し・罫線・配色・余白・文字をデザイントークンで合わせた。Cloudscape 表の試作 `LogTableCloudscape.tsx` は環境変数で切り替え可能にし、測定後に入れ替えを判断する（下の「性能測定」）。
8. **デザイントークンの当て方**：`@cloudscape-design/design-tokens` の値は `var(--awsui-…, fallback)` の文字列で、暗モードの再定義は `applyMode` がクラスを付ける `body` にある。そこで `--ls-*` を `body` のインラインスタイルとして設定し（`designTokens.ts`）、`styles.css` は `var(--ls-*)` だけを読む。CSS に固定色はない。
9. **モーダルの初期焦点**：Cloudscape の `Modal` はポータルで 1 コミット遅れて中身を描き、FocusLock が最初の要素（×）に焦点を当てる。`useInitialFocus`（callback ref → state → effect）で、部品が現れた次のコミットで一度だけ焦点を当てる（Cancel／Keep fetching／Close／チェックボックス）。
10. **Escape**：Modal は自分で Escape を処理するため、ダイアログ側の `useEscapeKey` は外した（二重に閉じるとコアが `session.nothingPending` を返す）。コマンドエラーの Alert だけは従来どおり Escape でも閉じる。
11. **Rust ツールチェーン**：手元の既定 `rustc 1.90.0` は `Cargo.lock` の依存（aws-* が 1.94.1 以上を要求）を満たさないため、`cargo +stable`（1.99.0）で実行した。CI は `dtolnay/rust-toolchain@stable` のため影響はない。

### Step 14（ゲートの修正依頼）の判断

12. **14a カレンダーが切れる原因と対処**：`node_modules/@cloudscape-design/components/date-picker/index.js` は `Dropdown` を `minWidth="trigger"`・`maxWidth="trigger"` で開く。`dropdown/dropdown-fit-handler.js` の `getWidths` は `idealWidth = min(max(必要幅, minWidth), maxWidth)` なので、ドロップダウンの幅はトリガー（日付欄）の幅そのものになる。`expandToViewport` のときは `dropdown/internal.js` の `setDropdownPosition` が `target.style.inlineSize = position.inlineSize` を設定するため、欄が狭ければカレンダー（`calendar/styles.scoped.css` の `inline-size: 238px`）が `.calendar { overflow: auto }` の中で切れる。スクリーンショットの「日〜木まで」は `11em`（約 154px）の欄幅と一致する。候補のうち「`expandToViewport` を外す」は、`Container fitHeight` の中身（`.content-wrapper-fit-height { overflow: hidden }`、`.content-fit-height { overflow: auto }`）がドロップダウンを切るため採れない。「最小幅を Cloudscape の設定で確保する」は DatePicker に公開プロパティがない。よって**日付欄の幅を広げる**を採り、値は DatePicker 自身の上限 `.date-picker-container { max-inline-size: 234px }` と同じ 234px にした（それ以上は Cloudscape が許さない）。テストは「7 列と月送りボタンが DOM にある」「ドロップダウンが欄の外（ポータル）にある」「`.condition-date` の幅が DatePicker の `max-inline-size` と等しい」を確かめる。実機での見え方は手動確認に残す（下の表）。なお 234px のドロップダウンに 238px のカレンダーが入る計算にはならないが（枠線 2px×2 を含めると 8px 不足）、これは Cloudscape 自身の DatePicker と同じ条件であり、実機で右端がわずかに切れる場合はこの計算のとおりで、Cloudscape 側の問題として記録する。
13. **14a の jsdom の制約**：vitest は CSS を空のモジュールとして扱うため、`?raw` 読み込みでも中身が空になる。`vitest.config.ts` の `test.css.include` に上の 2 ファイルの `?raw` だけを足し、ほかの CSS の扱いは変えていない（`@types/node` を足して `fs` で読む案は依存が増えるため見送った）。
14. **14b 文言**：「SDK default」／「SDK デフォルト」。上部バーは「Profile: SDK default」／「プロファイル: SDK デフォルト」。
15. **14c 起動時の選択はコアの責務**：`AppSession::with_catalog_and_time_zones` が、カタログに SDK デフォルトがあれば `ConnectionState::request_profile(SdkDefault, 既定リージョン, has_logs=false)` を適用する。接続の変更ではないので `connection_generation` は 0 のまま、ログの破棄も起きない。既定リージョンがあれば一覧（Loading）が始まり、アプリ側は `startup_listing()`（Loading の間だけ `Some`）を `setup` で受け取って通常の `start_listing` に渡す。`ConnectionState::new()` は「何も選ばない」のまま（`connection.rs` のテストは変えていない）。画面は起動時に何も送らない（`App.test.tsx` で `selectProfile`／`selectRegion`／`reloadLogGroups` が呼ばれないことを確認）。MVP の決定「起動時は何も選ばず、前回の選択も覚えない」（U2 Q2）のうち、**「何も選ばず」はこの指示で置き換え**、**「前回の選択は覚えない」は維持**する（`startup_selection_is_not_remembered_between_sessions` で確認）。
16. **14c Red → Green の記録**：テストを先に書いた直後の `cargo +stable test -p local-sights-core session`：`startup_listing` 未定義のコンパイルエラー。アクセサ `startup_listing()` だけ足した状態（選択の実装なし）で再実行：`FAILED. 77 passed; 5 failed`（`starts_idle_with_reasons_and_cannot_fetch`、`starts_with_the_sdk_default_profile_and_its_region_and_lists_its_log_groups`、`starts_with_the_sdk_default_profile_and_no_region_when_it_has_none`、`startup_selection_is_not_remembered_between_sessions`、`unselected_reasons_and_shared_reasons_are_both_listed`）。`--test u2_log_group_listing`：`FAILED. 9 passed; 1 failed`。`select_startup_profile()` を入れた後：`fill_valid` が「プロファイルが未選択なら dev を選ぶ」だったため dev を選ばなくなり 29 件が `UnknownLogGroup` で落ちた → ヘルパーを「dev でなければ dev を選ぶ」に直し、`every_reason_is_listed_in_the_order_of_the_conditions` の期待から `selection.profileRequired` を外して `82 passed`、u2 `10 passed`。
17. **14d 幅の保持先**：幅は秘密ではないため `localStorage`（キー `local-sights.logGroupPaneWidth`、rem の数値）に覚える。読めない・書けない環境（プライベートウィンドウ等）では既定値で動き、例外は握りつぶして画面は止めない。つまみは Tab 順で「再読み込み」の次、「取得」の前に入る（キーボードで幅を変えられるため焦点を受ける必要がある）。
18. **14e CSP**：`font-src 'self' data:` を `csp`・`devCsp` に追加。Cloudscape のグローバルスタイルがフォントを `data:` で埋め込むため。外部への通信先は増えない。
19. **14f ハーネスの歯止め**：`configuredPerfRowCount(env)` が `env.DEV` でなければ 0 を返す。Vite は `vite build` で `import.meta.env.DEV` を `false` に置き換えるため、本番ビルドでは変数を付けてビルドしても行は生成されない。
20. **14h 名前の範囲**：`productName`・ウィンドウタイトル・`app.title`・`index.html` の `<title>`・README の見出しを `Local Sights` に。`identifier`（`dev.local-sights.app`）、Cargo のパッケージ名、npm のパッケージ名は `local-sights` のまま（設定ファイル・キャッシュの場所が変わらない）。
21. **14i アイコン**：SVG（1024×1024、余白約 96px、文字なし）→ `qlmanage -t -s 1024 -o <dir> icon.svg`（`icon.svg.png`、`sips` で 1024×1024・アルファありを確認）→ `source/icon.png` に保存 → `tauri icon source/icon.png -o <scratch>` → 生成物のうち `32x32.png`・`128x128.png`・`128x128@2x.png`・`icon.icns`・`icon.ico`・`icon.png` だけを `src-tauri/icons/` に置き換えた（`android/`・`ios/`・`Square*Logo.png`・`StoreLogo.png` は `tauri.conf.json` が参照せず、リポジトリにも元からないため取り込まない）。

### Step 15（ゲートの修正依頼 2 回目）の判断

22. **15a 文言**：「Default」／「デフォルト」。上部バーは「Profile: Default」／「プロファイル: デフォルト」。SDK の既定の認証チェーンを指す説明は README の本文に残した。
23. **15b 狭くならない原因**：`.log-group-pane` は `flex: 0 0 <幅>` だけで `min-width` が既定の `auto` のため、flex アイテムの最小幅が中身の min-content 幅（`white-space: nowrap` のロググループ名の最長）になり、つまみで指定した幅より狭くならなかった（スクリーンショットの約 40% は最長の名前の幅）。`min-width: 0; overflow: hidden` で中身より狭くできるようにした。さらに Cloudscape の `Table` は `table-layout: auto` で名前の列が中身の幅に広がり表の中に横スクロールが出るため、`.log-group-pane-table table { table-layout: fixed }` を当てて、選択列（Cloudscape 側で 40px 固定）以外の幅を名前の列に渡し、既存の `.log-group-pane-name` の省略（…）が効くようにした。全名は従来どおり `title` に残る。
24. **15b 幅の単位**：rem から「アプリ本体（`.app-body`）の幅に対する %」に変えた。ドラッグの px は、つまみの親の `clientWidth`（jsdom のように 0 なら `window.innerWidth`、それも 0 なら 1024）で % に換算する。`clientWidth` は `.app-body` のパディングを含むため、厳密にはパディング分だけ換算が粗いが、ドラッグの追従感にしか影響しない。保存先のキーを `local-sights.logGroupPaneWidthPercent` に変え、旧キー（rem の数値）は読まない（14 を % と誤読して 20% に丸められるのを避ける）。
25. **15b 閉じる状態**：見出しの `actions` の「<」ボタンで閉じ、閉じた間は `LogGroupPane` と `PaneResizer` を描かず、左端に幅 36px の縦の帯（`.log-group-pane-collapsed`）と「>」ボタンだけを置く。幅は閉じても保持され、開くと同じ幅に戻る。閉じた状態は `local-sights.logGroupPaneCollapsed`（`"true"`／`"false"`）に覚え、`"true"` 以外は開いているとみなす。焦点：押したボタンが消えるため、閉じたら「>」へ、開いたら「<」へ焦点を移す（`paneFocusRef` と `useEffect`）。Tab 順は「歯車 → < → 検索 → 表 → 再読み込み → つまみ → 取得」、閉じているときは「歯車 → > → 取得」。
26. **15c 重複の解消**：見出しの未選択時の文言を案内文からただの状態表示「Log group: none」／「ロググループ: 未選択」に変え、「Choose a log group.」／「ロググループを選んでください。」は見出し直下の理由行（コア由来の `selection.logGroupRequired`）だけが言う。文言表のテストで見出しと理由が両言語で一致しないことも確かめる。
27. **15d 高さの揃え**：絞り込み欄の上に、開始・終了と同じ `Box variant="awsui-key-label"`（文言は `logFilter.label`）を置いた。`condition-row` は `align-items: flex-start` のままで、3 つの欄がすべて「ラベル 1 行 + 入力欄」の構造になるため入力欄の上端が揃う。TextFilter の aria-label も同じ文言のため、支援技術には同じ名前が 2 回聞こえるが（ラベルは装飾、名前は aria-label）、見た目の揃えを優先した。
28. **15e 文言**：「Filter messages」／「メッセージを絞り込み」（ラベル・プレースホルダー・README）。
29. **15f アイコンの四隅**：`qlmanage` は透明部分を白で塗るため（14i の PNG の四隅は `(255,255,255,255)` だった）、`@resvg/resvg-js` で描き出す `scripts/render-icon.mjs` に替えた。`background` を指定しないので SVG が塗らない部分は透明のまま。スクリプトは四隅のアルファが 0・中央のアルファが 255 でなければ PNG を書かずに終了コード 1 を返す。描き出し後の `source/icon.png` と `tauri icon` の生成物（`icon.png` 512px、`128x128.png`、`32x32.png`）の四隅はすべてアルファ 0、中央は 255 であることを確認した。`@resvg/resvg-js` は MPL-2.0（配布物には入らない開発時のみの依存）。`npm audit --audit-level=high` 0 件、`cargo-deny` は Rust の依存に変更がないため影響なし。
30. **15 Dock 名**：`tauri dev` は Cargo のバイナリ名で起動するため Dock に「Local Sights」と出ない。`tauri build` の `.app`（`productName`）では「Local Sights」になる。コードの変更はなく、手動確認の表に残した。

### Step 16（ゲートの修正依頼 3 回目）の判断

31. **16a 昨日の判定は暦日ベース**：`now_ms` をローカルゾーンに変換した暦日（`date_in_zone`）を「今日」、その `NaiveDate::pred_opt()` を「昨日」とする。瞬間から 24 時間を引くのではないので、夏時間の切り替え日（23 時間・25 時間の日）でも「昨日 00:00:00〜今日 23:59:59」が暦どおりになる（テストで America/New_York の 2024/03/10 と 2024/11/03 を確認：開始の瞬間は EST、終了の瞬間は EDT、またその逆）。文字列は `INPUT_FORMAT`（`yyyy/mm/dd hh:mm:ss`）で組み、`DateTimeInput::parse` に通すので、入力欄に打ち込んだ場合とまったく同じ経路で瞬間と誤りが決まる。
32. **16a 時刻の初期値**：開始 `00:00:00`・終了 `23:59:59`（利用者が確認済み。画面の空の時刻の補完規則と同じ値）。終了は従来どおりその秒の 999 ミリ秒まで含む（テストで `range.end_instant()` を確認）。
33. **16a 埋めない場合**：`now_ms` に暦日がない（chrono の範囲外）か、ローカルゾーンでの年が 4 桁でない（U4 R-05 と同じ境界）ときは埋めず、従来どおり空のまま（起動直後に誤りを見せない）。アプリ側は `SystemTime::now()` が UNIX_EPOCH より前、または i64 に収まらないときに埋めない（`current_epoch_ms() -> Option<i64>`）。なお 00:00:00 が夏時間で存在しないゾーン（例：かつての America/Sao_Paulo）では埋めたうえでコアの `startNonexistentLocalTime` が時刻欄の直下に出る（既存の経路どおり。隠さない）。
34. **16a 既存の生成関数は変えない**：`with_catalog_and_time_zones` などは空の入力のまま。初期値はビルダー `with_default_time_range` を呼んだときだけ入る。こうして空の入力を前提にする既存テストの前提を壊さず、テスト側も「`now` を与えない生成は空」を 1 つ確かめている。
35. **16a 画面側は無変更**：コアが持つ `startInput.text`／`endInput.text` を既存の `splitDateTime` の経路で日付欄・時刻欄に分けて出すだけ。起動時に画面は `updateInput` も `startFetch` も送らない（`App.test.tsx` で確認）。
36. **16a Red → Green の記録**：テストを先に書いた直後の `cargo +stable test -p local-sights-core session`：コンパイルエラー 8 件（`with_default_time_range` 未定義の E0599 ×5、`date_in_zone`／`format_day_bounds` 未定義の E0425 ×3）。実装後：`session` 87 passed（82 → 87）、`time_range` 20 passed（19 → 20）、コア全体 358 passed。`App.test.tsx` は画面を変えない実装後テストとして追加し 24 passed。
37. **16b アイコンの寸法**：Apple のアプリアイコンの規則（1024px のうち中央 824×824、上下左右 100px の透明な余白、角の半径約 185px）に合わせた。従来の 1024px 全面の板は Dock で他のアプリより約 24% 大きく見えていた。中の図形は `scale(0.8046875)`（= 824 / 1024）で同じ比率に縮めたので構図は変わらない。`render-icon.mjs` の中央の検証座標（512, 512）は板の中なので変更不要。余白の検証は計画の `(50, 512)` 1 点ではなく、各辺の余白の中央 4 点にした（1 点では左の余白しか確かめないため）。`tauri icon` の生成物は 14i・15f と同じく 6 ファイルだけ置き換えた。

### Step 17（ゲートの修正依頼 4 回目）の判断

38. **17b 手入力の未来日は対象外**：制限するのはカレンダーだけで、日付欄に未来の日付を打った場合は理由も出さず、取得もそのまま行える（承認済みの前提。利用者の依頼は「カレンダーで」であり、未来の範囲を取得しても CloudWatch は 0 件を返すだけで害がない）。Cloudscape も「無効にした日もキーボードでは入力できる」としている（`calendar/interfaces.d.ts` の `isDateEnabled` の説明）。
39. **17b 「今日」の基準**：表示中のタイムゾーン（`session.timeZone`）の今日。Local は OS のタイムゾーンの暦日、UTC は UTC の暦日なので、深夜 0 時の前後では 1 日ずれる（例：日本時間 3/16 01:00 は UTC では 3/15 で、UTC 表示なら 3/16 が無効）。時刻欄の入力と同じゾーンで日付を決めるため、この基準にした。`Date.now()` は `isDateEnabled` の呼び出しのたび（カレンダーが日を描くとき）に読むので、アプリを開いたまま日付が変わっても、次にカレンダーを開けば新しい今日が基準になる。
40. **17b セルの日付の読み方**：Cloudscape が `isDateEnabled` に渡す `Date` はローカルの日付（`calendar/grid/use-calendar-grid-rows.js` が呼ぶ `internal/utils/date-time/calendar.js` の `getCalendarMonthWithSixRows`、中身は `mnth` の `getCalendarMonth` でローカル時刻の `Date` を作る）なので、その `getFullYear`／`getMonth`／`getDate` が暦日になる。これは `todayInZone(date.getTime(), "Local")` と同じなので、組み立ての処理を 1 つにまとめた。UTC 表示でもセルは「暦日」として読むので、UTC の今日との比較は日付の文字列どうしで正しい。
41. **17b 無効な日の読み上げ**：Cloudscape の `DatePicker`／`Calendar` の `i18nStrings` には無効な日を読み上げる文言がない（`todayAriaLabel`、月・年の送りの aria-label などだけ）。無効な日のセル（`td`）には Cloudscape が `aria-disabled="true"` を付け、`onClick` と `tabIndex` を付けない（`calendar/grid/index.js`）ため、支援技術は「利用不可」として扱う。`dateDisabledReason`（無効の理由をツールチップと読み上げで出し、そのセルにも焦点が当たるようになる）という別のプロパティはあるが、新しい文言と焦点の振る舞いの変更を伴い計画にないため使っていない（必要なら次の作業で足せる）。`todayAriaLabel` は既存の設定がない（`I18nProvider` の `all.ja`／`all.en` の既定のまま）ため触れていない。
42. **17b 「今日」の強調との関係**：Cloudscape のカレンダーが今日のセルに付ける強調（`aria-current="date"`）は、常に OS のタイムゾーンの今日（`new Date()`）で決まる。UTC 表示で深夜 0 時の前後のときは、強調される日（ローカルの今日）と選べる最後の日（UTC の今日）が 1 日ずれる。Cloudscape に強調の日付を渡すプロパティはないため、そのままにした（手動確認の表に記載）。
43. **17a Red → Green の記録**：テストを先に書いた直後の `npx vitest run src/dateTime.test.ts`：`5 failed | 22 passed (27)`（`todayInZone is not a function`、`isFutureDate is not a function`）。実装後：`27 passed`。部品のテスト（17b）は実装後に書き、`ConditionArea.test.tsx` 21 passed。テストの TZ は `vi.stubEnv("TZ", "Asia/Tokyo")` で固定し、外側の TZ を `America/Los_Angeles` にして実行しても 2 ファイル 48 passed であることを確かめた（手元の TZ に頼らず通る。CI は UTC）。

### Step 18（ステータス行を常に見える位置に）の判断

44. **18a 原因**：計画の見込み（「`Container` の内側が flex の列になっておらず高さが決まらない」）は半分正しかった。`ConditionArea` はすでに `fitHeight` を使っており、Cloudscape の DOM（`container/internal.js`、`styles.scoped.css`）では、根（`fit-height`：flex の列、`block-size: 100%`）→ `content-wrapper`（flex の列、`block-size: 100%`、`overflow: hidden`）→ `content`（`fit-height` で `flex: 1; overflow: auto; display: flex; flex-direction: column`）→ `content-inner`（`flex: 1`、余白つき）→ 子、の順に並ぶ（`BuiltInErrorBoundary` は DOM を出さない）。根から `content` までは窓の高さに収まるが、`content-inner` は flex の項目で `min-height` が既定の `auto` のため、中身の高さより縮まない。中身の `.condition-area { height: 100% }` は、その高さを決める計算（最小内容の高さ）では割合が効かず中身の高さで数えられ、ログ一覧の自前の仮想スクロールが持つ全行分の高さ（`.log-table-content` の `height`）がそのまま `content-inner` を押し広げていた。結果として `content`（`overflow: auto`）が条件エリア全体をスクロールし、ステータス行はその最下部＝窓の外にあった。
45. **18a 対処：公開 API では足りず、自前の囲みにした**：`fitHeight` はすでに使っている。`Container` の `style` プロパティは `content` の余白（`paddingBlock`／`paddingInline`）しか受け取らず（`container/style.js` の `getContentStyles`）、`content-inner` に `min-height: 0` を当てる公開の手段がない。内部のクラス名（`awsui_content-inner_…` はビルドごとのハッシュつき）には依存しないことにし、自前の囲み `.condition-area-frame`（`position: relative; height: 100%`）の上に列 `.condition-area` を `position: absolute; inset: 0` で重ねた。絶対配置の中身は `content-inner` の高さの計算に入らないので、`content-inner` は `content` の残りの高さ（flex で決まった確定の高さ）に収まり、枠の `height: 100%` はその高さになる。列の中ではログ一覧（`.log-table { flex: 1; min-height: 0 }`、中の `.log-table-viewport` が `overflow-y: auto`）だけが残りの高さに縮んでスクロールし、条件行・アラート・ステータス行（`flex: 0 0 auto`）は自分の高さを保つ。`contain: size` で中身を高さの計算から外す案もあったが、絶対配置のほうが WKWebView を含めて挙動が長く安定しているため選ばなかった。
46. **18a 枠の最小の高さ（15rem）**：窓がとても低い場合に列が潰れきらないよう、枠に `min-height: 15rem`（240px）を付けた。最小の窓 1024×640 では、上部バー 48px と本体の余白を引いても枠には約 470px が残る見込みで、この下限には届かない。下限を下回る場合は従来どおり `content` がスクロールする（はみ出しを切り捨てない）。
47. **18a 仮想スクロールへの影響**：`LogTable` は `.log-table-viewport` の `clientHeight` を `ResizeObserver` で測り、描く行の範囲を決めている（`LogTable.tsx` の `setMeasuredSize`）。これまでは見えている高さではなく、伸びたビューポートの高さを測っていた可能性が高い。修正後はビューポートの高さが窓に合わせて決まり、窓の大小で `ResizeObserver` が再計測するので、描く行は見えている分（＋前後の余白）になる。計測の仕組みそのものは変えていない（`LogTable.test.tsx` 18 件は変更なしで緑）。
48. **18a 試作の表**：開発用の Cloudscape 表の試作（`VITE_LOCAL_SIGHTS_LOG_TABLE=cloudscape`）は同じ列に入るため、`.log-table-cloudscape` にも同じ縮める指定と `overflow: auto` を当て、試作を使った性能確認でもステータス行が押し出されないようにした。

## 性能測定（NFR2、R-03）

**未実施**。測定は Tauri 実機（開発者の Mac）で行う必要があり、この環境では実行できない。手順は README の「Performance check of the log list」に書いた。

| 測る項目 | 合格 | 結果 |
|---|---|---|
| 絞り込み入力から結果表示まで（100 万行） | 100 秒以内 | 未測定 |
| ホイール 1 回・PageDown・End・行展開から画面が変わるまで | 1 秒以内 | 未測定 |
| 10 万行ずつ増える間の入力欄への入力の反映 | 1 秒以内 | 未測定 |
| メモリで落ちないこと | 落ちない | 未測定 |

**合否の扱い（計画からの逸脱の記録）**：計画は「測ってから決める」だったが、測れないため、安全側の選択肢（既存の仮想スクロールにトークンを当てる）を既定にし、Cloudscape 表の試作を切り替え式で残した。測定の結果すべて合格なら `App.tsx` の `LOG_TABLE_VARIANT` の既定を `cloudscape` に変える（`src/dev/perfRows.ts`）だけで入れ替わる。SM1 の例外（ログ一覧が Cloudscape 部品でない）は承認時に確認いただきたい。

## 手動確認が必要

次は実行していない。開発者が `npm run tauri dev` で確認する（README の「Console-style screen」チェックリストに同じ項目を置いた）。

| 項目 | 確認の仕方 | 期待 |
|---|---|---|
| AS4：Cloudscape の部品が WKWebView で描画される | 起動して TopNavigation・DatePicker（カレンダー）・TimeInput・Modal を開く | すべて描画・操作できる |
| R-07：DatePicker に `2024-03-01` を打つ | 開始の日付欄に入力 | jsdom での解析では、Cloudscape のマスクは `-` を区切り文字として受け取り `2024/03/01` に正規化する（`generateMaskArgs` の `inputSeparators: ['-', '.', ' ']`）。実機でも同じなら、ハイフンは「形式の誤り」ではなく「自動で直る」振る舞いになる。結果をここに追記する |
| 性能測定（上記） | README の手順 | 4 項目の数値 |
| FR4.5：OS の明暗の切り替え | OS を切り替える | 再起動なしで全体が切り替わる（自前描画のログ一覧も） |
| FR4.6：日本語環境でカレンダーの月名・曜日 | OS 言語を日本語にして起動 | 日本語で出る |
| NFR7：1024×640 | ウィンドウを最小にする | 2 ペインと条件行が崩れない（折り返しは想定内） |
| K1〜K8（accessibility-checklist） | キーボードだけで一通り | 操作できる |
| 14a：カレンダーの幅 | 開始・終了の日付欄のカレンダーボタンを押す | 曜日の 7 列すべてと「＜」「＞」の月送りボタンが見え、右端が切れない（切れる場合は判断 12 の 8px の計算どおりか、幅を測って記録する） |
| 14e：CSP 違反なし | `npm run tauri dev` で起動し WebView のインスペクタを開く | 起動時・カレンダー／ドロップダウン／ダイアログを開いたときにコンソールに CSP 違反が出ない |
| 14h：アプリ名 | `tauri build` の `.app` を起動する | Dock・ウィンドウタイトル・ウィンドウ切替（Cmd+Tab）に「Local Sights」と出る（`tauri dev` の Dock 名は Cargo のバイナリ名、判断 30） |
| 14i：アイコン | 起動し、Finder でバンドルを見る | Dock と Finder に新しいアイコン（濃紺のタイル・ログ行・虫眼鏡）が出る（Dock のキャッシュが残る場合は再起動で更新） |
| 14c：起動時の選択 | 既定リージョンのある SDK 設定（`AWS_REGION` または `~/.aws/config` の `[default]`）で起動する | 上部バーが「Profile: SDK default」とそのリージョンになり、クリックなしでロググループ一覧が読み込まれる。既定リージョンがなければリージョンは未選択で一覧は「リージョンを選んでください」 |
| 14d・15b：ペインの幅 | 2 ペインの間のつまみをドラッグ、焦点を当てて ←／→ | 幅が窓の 20%〜50% の間で変わり（既定 25%）、20% まで狭めると長い名前が「…」で省略されホバーで全名が出る。再起動後も同じ幅で開く |
| 15b：左ペインを閉じる・開く | 見出しの「<」を押す、帯の「>」を押す | 閉じると左端に細い帯と「>」だけが残り、ログ一覧が残りの幅を使い、焦点は「>」にある。「>」で元の幅に戻り、焦点は「<」にある。閉じたまま終了すると次回も閉じている |
| 15c・15d・15e：条件エリアの文言と揃え | ロググループ未選択で起動する | 見出しは「Log group: none」、その下に「Choose a log group.」が 1 回だけ出る。絞り込み欄のラベルは「Filter messages」で、入力欄の上端が開始・終了の入力欄と同じ高さ |
| 15f：アイコンの四隅が透明 | `npm run tauri build` した `.app` を Finder と Dock で見る（`tauri dev` でも Dock に出る） | 角丸の外側が白くなく、背景が透けて見える |
| 15：`tauri build` の .app で Dock 名 | `npm run tauri build` して `.app` を起動する | Dock・Cmd+Tab に「Local Sights」と出る（`tauri dev` では Cargo のバイナリ名のまま） |
| 16a：起動時に昨日〜今日が入っている | 起動する（深夜 0 時の前後にも試す） | 開始の日付欄が昨日・時刻欄が `00:00:00`、終了の日付欄が今日・時刻欄が `23:59:59`（OS のタイムゾーンの暦日）。UTC に切り替えると両欄が書き換わる。ロググループを選ぶだけで [取得] が押せる |
| 16b：Dock で他のアプリと同じ大きさ | `npm run tauri build` した `.app` を Dock に置く（`tauri dev` でも Dock に出る） | 他のアプリのアイコンと同じ大きさに見える（板は 1024px のうち中央 824px、余白 100px）。Dock のキャッシュが残る場合は再起動で更新 |
| 17b：カレンダーで明日以降が選べない | 開始・終了の日付欄のカレンダーを開き、明日・来月の日を押す。上部バーで UTC に切り替えて同じことをする（深夜 0 時の前後にも試す） | 明日以降の日は薄く表示され、押しても欄の値が変わらない。今日と過去の日は選べる。UTC 表示では UTC の今日が最後の選べる日（日本時間 0〜9 時は前日）。今日の強調はローカルの今日のまま（判断 42）。日付欄に未来の日付を手で打つと受け付けられ、取得もできる |
| 18a：ステータス行が常に見える | 1,000 行以上を取得する（取得中にも見る）。窓を最小の 1024×640 にし、高さを伸縮させる。取得エラーのアラートが出た状態（権限のないプロファイル等）でも見る。ログ一覧をホイール・PageDown・End でスクロールする | ステータス行（「取得中… n/m ストリーム、これまでに N 件」「絞り込み後 N 件 / 全 M 件」など）が常に右の枠の下端に見える。スクロールするのはログ一覧だけで、条件行とステータス行は動かない。仮想スクロールの行が欠けたり、最後の行に届かなかったりしない |

自動テストで確かめた範囲：jsdom で Cloudscape の実物を描画し、DatePicker のカレンダーから日を選ぶ、時刻の補完、欄ごとの理由、Tab の順序（DOM 順）、Modal の焦点と Escape（`keyCode` 付きの keydown）を検証した。**jsdom の制約**：`@testing-library/user-event` の `{Escape}`／`{Tab}` は `keyCode` を付けないため、Cloudscape の Modal（Escape）とマスク入力（Tab）には届かない。テストでは Escape を `fireEvent.keyDown(…, {keyCode: 27})` で送り、マスク入力からの Tab 順序は DOM 順で確かめた。実機のブラウザーでは `keyCode` が付くため問題ない。

## 維持した振る舞い（FR4.4、R-04）

| 振る舞い | 実装 | 確かめたテスト |
|---|---|---|
| 行の展開・閉じる、複数展開、全文のコピー | `LogTable.tsx`（変更なし） | `LogTable.test.tsx` 既存 |
| キーボードでの行選択（↑↓・PageUp／Down・Home／End・Enter／Space） | `LogTable.tsx`（変更なし） | `LogTable.test.tsx` 既存 |
| 取得中の操作ロック（接続・ロググループ・日付・時刻・[取得] は無効、絞り込み欄とタイムゾーンは有効） | `TopBar.tsx`、`LogGroupPane.tsx`、`ConditionArea.tsx` | `TopBar.test.tsx`「disables the connection and settings while fetching」、`LogGroupPane.test.tsx`「cannot select while locked」、`ConditionArea.test.tsx`「disables the date, time and Fetch while fetching」 |
| 接続変更の確認ダイアログ（表示中のログがあるときだけ、Cancel に焦点、Escape＝Cancel） | `ConfirmDialog.tsx`、`App.tsx` | `ConfirmDialog.test.tsx`、`App.test.tsx`「asks before a connection change」 |
| 終了の確認ダイアログ（Keep fetching に焦点、Escape＝Keep、焦点の戻り） | `CloseConfirmDialog.tsx` | `CloseConfirmDialog.test.tsx`、`App.test.tsx`「asks before closing while fetching」 |
| 設定ダイアログ（チェックボックスに焦点、[Clear cache] の即時反映、閉じたら歯車に焦点が戻る） | `SettingsDialog.tsx` | `SettingsDialog.test.tsx`、`App.test.tsx`「opens the settings dialog」 |
| ロググループの絞り込み（検索欄、取得中も使える、確認待ちは無効） | `LogGroupPane.tsx` | `LogGroupPane.test.tsx` |
| ログの絞り込み（0.3 秒後に送る、Enter で即時、拒否後の再送、ダイアログ後の再送） | `LogFilterInput.tsx` | `LogFilterInput.test.tsx`、`App.test.tsx` |
| タイムゾーン切替で入力の瞬間を保ち文字列だけ書き換わる | `App.tsx`（`key` による再マウント）、`ConditionArea.tsx` | `App.test.tsx`「switches the time zone」 |
| 失敗したストリームの一覧（ステータス行のボタン、Close に焦点、Escape、焦点の戻り） | `FailureList.tsx`、`StatusLine.tsx` | `FailureList.test.tsx`、`App.test.tsx` |
| エラーの中身（種類の判定・文言・安全な詳細）は不変 | `errorText.ts`、`ErrorMessage.tsx`（変更なし） | `errorText.test.ts`、`ErrorMessage.test.tsx`、`messages.test.ts`（`error.what.*`／`error.next.*` の逐語一致） |
| 列と並び（時刻・ストリーム・メッセージ） | `LogTable.tsx`、`LogTableCloudscape.tsx` | `LogTable.test.tsx`、`LogTableCloudscape.test.tsx` |
| Tab 順（Step 14d・15b）：画面の順のまま。「再読み込み → つまみ → 取得」（14d）に、ペインの見出しの「<」が歯車の次に入る（15b）。閉じているときは「歯車 → > → 取得」 | `App.tsx`、`LogGroupPane.tsx`、`PaneResizer.tsx` | `App.test.tsx`「moves through the screen with Tab …」「starts with the pane closed …」 |
| 起動時の選択（Step 14c）：MVP の「起動時は何も選ばない」はこの変更で置き換え（SDK デフォルトのプロファイルと、あればその既定リージョンを選んで一覧を始める）。「前回の選択は覚えない」は維持（セッションはカタログからだけ作られ、前のセッションの選択を読まない） | `session.rs`（`select_startup_profile`、`startup_listing`）、`src-tauri/src/lib.rs`（`setup`） | `session.rs`「starts_with_the_sdk_default_profile_…」×2、「a_catalog_without_the_sdk_default_…」、「startup_selection_is_not_remembered_between_sessions」、`u2_log_group_listing.rs`「startup_selects_the_sdk_default_…」、`App.test.tsx`「shows at startup the SDK default profile …」 |
| 起動時の日付（Step 16a）：入力欄の初期値は「空」から「昨日 00:00:00〜今日 23:59:59（ローカルの暦日）」に置き換え。入力の編集・時刻の補完・タイムゾーン切替・欄ごとの理由の経路はそのまま（初期値も同じ `DateTimeInput::parse` と `revalidate` を通る）。前回の入力は覚えない | `session.rs`（`with_default_time_range`）、`time_range.rs`（`date_in_zone`、`format_day_bounds`）、`src-tauri/src/lib.rs`（`AppState::load`、`current_epoch_ms`） | `session.rs`「starts_with_yesterday_to_today_…」「the_default_range_follows_the_local_zone_not_utc」「yesterday_crosses_a_month_and_a_year_boundary_…」「the_default_range_is_whole_calendar_days_across_a_daylight_saving_change」「without_a_now_the_inputs_start_empty_…」、`time_range.rs`「day_bounds_are_written_…」、`App.test.tsx`「shows at startup the default range the core filled in …」 |

## 文言の変更（interaction-spec §9）

追加：`form.start.date`／`form.start.time`／`form.end.date`／`form.end.time`（aria-label）、`form.date.placeholder`／`form.time.placeholder`、`form.date.required`、`form.date.format`、`form.time.format`、`logGroups.column.name`、`logGroups.selection`、`logGroups.filter.clear`、`logFilter.clear`、`table.empty`、`table.noMatches`、`failures.column.stream`、`failures.column.reason`。
Step 15 の変更：`connection.profile.sdkDefault`（Default／デフォルト）、`form.selectedLogGroup.none`（Log group: none／ロググループ: 未選択）、`logFilter.label`／`placeholder`（Filter messages／メッセージを絞り込み）。追加：`pane.collapse`、`pane.expand`。

Step 13 までの変更：`form.selectedLogGroup.none`（Select a log group／ロググループを選んでください）、`form.start.label`／`form.end.label` の日本語（開始（{zone}）／終了（{zone}））、`validation.startFormat`／`endFormat`（`yyyy/mm/dd hh:mm:ss`）、`validation.rangeOrder`（End must be after start.／終了は開始より後にしてください。）、`logGroups.filter.label`／`placeholder`（Find log groups／ロググループを検索）、`logFilter.label`／`placeholder`（Filter events／イベントを絞り込み）。
削除：`form.reasons.title`、`form.dateTime.placeholder`。

## 計画からの逸脱

| 計画 | 実際 | 理由 |
|---|---|---|
| Step 1：Tauri 実機での描画確認と DatePicker のハイフン確認 | 未実施。jsdom でのコード解析と描画確認で代替し、実機確認を README と上の表に残した | GUI 実機が必要 |
| Step 2：`cargo test -p local-sights-core time_range date_input` | `time_range` と `date_input` を別々に実行した | `cargo test` のフィルタは 1 つしか受け取らない |
| Step 6：`TopNavigation` の `utilities`（`menu-dropdown`／`button`） | custom content の中の `ButtonDropdown`／`Button` | `utilities` に `disabled` がない（R-05 の表を満たせない） |
| Step 6：タイムゾーンは「通常・取得中に有効、確認待ち・設定表示中に無効」 | 計画の表どおり実装（MVP では常に有効だった） | 計画の R-05 表に従った |
| Step 7：失敗は表の空の状態にエラー文言 | 失敗（Partial）は常に表の上の warning Alert に「一覧は途中までです」＋3 行を出し、表の空の状態は 0 件／該当なしの文言だけ | コアは「一部失敗」と「全部失敗」を区別しない（どちらも `Partial`＋`failure`）ため、1 か所に出すほうが読みやすい。取れた行はそのまま表に残る |
| Step 8：ハイフン入力・`2024/02/30` で日付欄の直下に理由（タイプ操作で） | タイプでは Cloudscape のマスクが `-` を区切りに正規化し、日の上限も月に合わせて丸めるため再現できない。不完全な日付（`2024/03`）のタイプと、コアが持つハイフン／不在日の文字列（旧データ）で理由が出ることをテストした。`isValidDateText` 単体でハイフンと `2024/02/30` の拒否は確認した | DatePicker の仕様（R-07 の実機確認で裏を取る） |
| Step 8：時刻欄の blur を `user.tab()` で起こす | `focus()` で別の要素に焦点を移して blur させた。Tab の順序は DOM 順で検証 | user-event の Tab は `keyCode` を持たず、マスク入力が飲み込む |
| Step 9：ダイアログの Escape | Modal 自身の処理に任せ、`useEscapeKey` は外した。テストでは `keyCode: 27` 付きの keydown を送る | 二重処理の回避、user-event の制約 |
| Step 10：測ってから決める | 既定は既存の仮想スクロール＋トークン。試作は環境変数で切り替え | 測定は実機のみ可能（指示どおり） |
| Step 11：`validation.rangeOrder` の文言 | §9 どおり「End must be after start.」に変更（旧「The start must be before the end.」） | 画面案の文言表に従った |
| Step 12：`cargo` は既定ツールチェーンで実行 | `cargo +stable` で実行 | 手元の既定 1.90.0 が `Cargo.lock` の MSRV（1.94.1）未満 |
| Step 5：App.test の更新 | Step 9 までの部品が揃ってからまとめて更新した | 途中の状態では成立しないため |
| Step 14a：実機確認で合否を決める | 実機は動かせないため、原因の特定と幅の変更、jsdom で確かめられる範囲のテストまで。見え方は手動確認の表と README に残した | GUI 実機が必要 |
| Step 14a：テストで CSS を読む方法 | `vitest.config.ts` の `test.css.include` に 2 ファイルの `?raw` を追加した | vitest は CSS を空にするため（`@types/node` の追加は避けた） |
| Step 14c：`tests/u2_log_group_listing.rs` の「起動の前提」を直す | 同ファイルに起動の前提を置くテストはなかったため、直す代わりに起動時の一覧がゲートウェイを通る統合テストを 1 つ足した | 既存テストは `run_listing` の単体の振る舞いだけを扱っていた |
| Step 14c：既存テストの期待の変更 | `every_reason_is_listed_in_the_order_of_the_conditions` と `unselected_reasons_and_shared_reasons_are_both_listed` から `selection.profileRequired` を外し、`fill_valid` と `profile_without_default_region_…` を新しい前提に合わせた | 起動時にプロファイルが選ばれるため。品質目標の緩和ではなく、承認された振る舞いの変更に合わせたもの |
| Step 14d：Tab 順 | 「再読み込み → つまみ → 取得」になり、`App.test.tsx` の Tab 順の期待につまみを足した | つまみはキーボードで操作するため焦点を受ける |
| Step 14d：`localStorage` が使えない環境のテスト | `window.localStorage` のゲッターが例外を投げる状態で既定値になることを確かめた | 計画の「使えない環境では初期値」をそのまま検証 |
| Step 14f：`vi.stubEnv("DEV", false)` | 行数の判定を `configuredPerfRowCount(env)` として `env` を引数に取り、テストでは環境オブジェクトを渡した | モジュール読み込み時に評価される定数を `stubEnv` だけで切り替えるには `vi.resetModules` と動的 import が必要で、関数化のほうが読みやすい |
| Step 14i：`tauri icon` の生成物 | `src-tauri/icons/` に元からある 6 ファイルだけ置き換え、`android/`・`ios/`・Windows ストア用の PNG は取り込まなかった | `tauri.conf.json` の `bundle.icon` が参照しないため |
| Step 14g：`npm audit` | Node 22 の `npm` で実行（`/Users/kdnakt/.nvm/versions/node/v22.21.1/bin/npm`） | シェルの既定 Node 16 では npm 10 が動かない |
| Step 15b：`usePaneWidth` の戻り値 | `[width, setWidth]` のタプルから `{ width, collapsed, setWidth, setCollapsed }` に変えた | 閉じた状態を同じフックで持つため（計画の「`usePaneWidth`（または隣のフック）」の前者） |
| Step 15b：Tab 順 | 歯車の次にペインの「<」ボタンが入り、`App.test.tsx` の Tab 順の期待に足した | 計画どおり見出しの `actions` に置いたため、DOM 順で検索欄の前になる |
| Step 15b：名前の省略 | `.log-group-pane { min-width: 0; overflow: hidden }` に加えて `.log-group-pane-table table { table-layout: fixed }` を当てた | `min-width: 0` だけでは Cloudscape の表（`table-layout: auto`）が表の中で横に伸び、省略（…）が効かないため |
| Step 15b：つまみの換算 | 親の `clientWidth`、なければ `window.innerWidth`、それも 0 なら 1024 | jsdom はレイアウトを持たないため、テストでは親の `clientWidth` を定義して 10px＝1% にした |
| Step 15b：閉じる・開くの焦点 | 閉じたら「>」へ、開いたら「<」へ焦点を移す | 計画にはないが、押したボタンが消えると焦点が body に落ちるため（キーボード操作の継続、K 系） |
| Step 15d：ラベルと aria-label | ラベルの `Box` と TextFilter の aria-label が同じ文言 | 見た目の揃えを優先。装飾のラベルを aria-label に結びつける仕組みが TextFilter にないため |
| Step 15f：`tauri icon` の生成物 | 14i と同じく 6 ファイルだけ置き換えた | `tauri.conf.json` の `bundle.icon` が参照しないため |
| Step 15f：README の手順 | README に「App icon」の節を新設した | 従来の README にアイコンの手順はなく（14i は code-summary だけに記録）、「手順を更新する」は新設で満たした |
| Step 16a：`format_input_in_zone` の書式で文字列を作る | 書式定数 `INPUT_FORMAT` を使う `pub(crate) fn format_day_bounds` と `date_in_zone` を `time_range.rs` に足し、`session.rs` はそれを呼ぶ | 定数が `time_range` の中に閉じているため。瞬間ではなく暦日から書くので、`format_input_in_zone`（瞬間 → 文字列）そのものは使えない |
| Step 16a：テストは session に 5 つ | session に 5 つ、加えて time_range に 1 つ（`day_bounds_are_written_…`）を足した | 新しい crate 内関数の年の境界（10000 年で `None`）を直接確かめるため（team.md「ライブラリ側の関数にはテスト」） |
| Step 16b：余白の検証は `(50, 512)` の 1 点 | 各辺の余白の中央 4 点（`(50, 512)` を含む） | 1 点では左の余白しか確かめないため |
| Step 16b：`npm run tauri icon <png>`（`src-tauri/icons/` に直接出力し、不要物を削除） | `-o <scratch>` に出力し、6 ファイルだけ `cp` した | README に書いた手順と同じ。削除より複製のほうが取り違えがない（`git status --short src-tauri/icons` で 6 ファイルの変更だけを確認） |
| Step 16c：Tauri 実機の確認（起動時の日付、Dock の大きさ） | 未実施。手動確認の表と README のチェックリストに残した | GUI 実機が必要 |
| Step 17a：TZ の固定（計画に記載なし） | `dateTime.test.ts`・`ConditionArea.test.tsx` の該当節で `vi.stubEnv("TZ", "Asia/Tokyo")`／`vi.unstubAllEnvs()` を使った | 「Local」の今日は OS のタイムゾーンで決まるため、固定しないと手元（日本）と CI（UTC）で結果が変わる。`process.env` を直接使うと `@types/node` が必要になり依存が増える（判断 13 と同じ方針） |
| Step 17b：無効な日を読み上げる `i18nStrings` を渡す | 渡していない | Cloudscape の `i18nStrings` に該当する文言がない。セルの `aria-disabled="true"` で支援技術に伝わる（判断 41） |
| Step 17b：今日の基準は「描画時の `Date.now()`」 | `isDateEnabled` の呼び出しのたびに `Date.now()` を読む | カレンダーが日を描くときに読むので実質は同じで、開いたまま日付が変わっても次に開いたときに正しい（判断 39） |
| Step 17b：テストは 3 つ | 3 つ。開始側のテストで、無効な日を押しても値が変わらず `onChange` が呼ばれないこと、今日を押すと選べることも確かめた | 「選べない」を `aria-disabled` の属性だけでなく操作でも確かめるため |
| Step 17c：Tauri 実機の確認（カレンダーで明日以降が選べない） | 未実施。手動確認の表と README のチェックリストに残した | GUI 実機が必要 |
| Step 18a：`ConditionArea` の内側を `display:flex; flex-direction:column; flex:1; min-height:0` で窓の高さに収める | 列の指定はそのまま、`flex: 1` の代わりに枠（`position: relative; height: 100%`）の上へ `position: absolute; inset: 0` で重ねた | 縮まないのは Cloudscape の `content-inner`（公開 API で指定できない）で、その子に `flex: 1` を当てても `content-inner` は flex の列ではないため効かない（判断 44・45） |
| Step 18a：自前の囲みの名前 `.condition-area-body` | `.condition-area-frame`（既存の列 `.condition-area` はそのまま残し、その外側の枠にした） | 新しい要素は列の「中身」ではなく、列を重ねる「枠」のため。列のクラス名を変えずに済む |
| Step 18a：計画に記載のない変更 | 枠の `min-height: 15rem`、`.status-line { flex: 0 0 auto }`、試作 `.log-table-cloudscape` の縮める指定 | とても低い窓での潰れ防止（判断 46）、ステータス行を縮ませない、試作の性能確認でも同じ見え方にする（判断 48） |
| Step 18b：テスト 2 つの置き場所 | 「同じ列の最後の子」（取得エラーのアラート表示中も含む）は `App.test.tsx`、「縮める指定のクラス」は `ConditionArea.test.tsx` でスタイルシートの規則まで読んで確かめた | 列の組み立ては `App` と `ConditionArea` にまたがるため。クラスが付くだけでは規則の中身を保証しないため、14a と同じ `?raw` の読み方を使った |
| Step 18c：`cargo +stable fmt`・`clippy` | 実行していない（計画の 18c の一覧にもない） | Rust に変更がないため |
| Step 18c：Tauri 実機の確認（ステータス行が常に見える） | 未実施。手動確認の表と README のチェックリストに残した | GUI 実機が必要。jsdom は高さを計算しない |

## テストの数と実行結果

- 追加・更新したテスト：コア +2（`time_range` 19、`date_input` 9）、画面：`dateTime` 22、`useColorScheme` 3、`designTokens` 2、部品 9+11+15+7+5+5+16+5+4+7+4（`ErrorMessage` 既存）+18+3、文言 17、App 20。回帰テスト：(1) `ConditionArea.test.tsx`「regression (FR3.1): the list of reasons is gone」、(2) `FetchErrorBanner.test.tsx`「regression (FR3.2): the three-line alert closes with its button」、(3) `time_range.rs`「rejects_hyphen_separated_dates」「written_texts_use_slashes_and_no_hyphen」。
- 変更前の基準：vitest 21 ファイル 171 テスト、cargo 346 passed / 5 ignored。

Step 13 までの実行（2026-10-11、参考）：vitest 24 ファイル 210 テスト、cargo 348 passed / 5 ignored。

Step 14 の後の実行（2026-10-11、参考）：vitest 26 ファイル 224 テスト、cargo 352 passed / 5 ignored。

Step 15 の後の実行（2026-10-11、参考）：vitest 27 ファイル 235 テスト、cargo 352 passed / 5 ignored。

Step 16 の後の実行（2026-10-11、参考）：vitest 27 ファイル 236 テスト、cargo 358 passed / 5 ignored。

Step 17 の後の実行（2026-10-11、参考）：vitest 27 ファイル 244 テスト、cargo 358 passed / 5 ignored。

最終実行（2026-10-11、Step 18 の後）：

```
$ npx vitest run
 Test Files  27 passed (27)
      Tests  246 passed (246)

$ cargo +stable test -p local-sights-core
test result: ok. 316 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out   (unittests src/lib.rs)
test result: ok. 11 passed; 0 failed   (tests/u1_fetch_flow.rs)
test result: ok. 10 passed; 0 failed   (tests/u2_log_group_listing.rs)
test result: ok. 14 passed; 0 failed   (tests/u3_fetch_flow.rs)
test result: ok. 4 passed; 0 failed    (tests/u5_filter_flow.rs)
test result: ok. 3 passed; 0 failed    (tests/u6_cache_flow.rs)
合計 358 passed / 0 failed / 5 ignored
```

そのほか（Step 18 の後）：`npx tsc --noEmit` OK、`npx eslint .` 0 件、`npx prettier --write .` のあと `--check` OK、`npx vite build` OK、`npm audit --audit-level=high` 0 件。Step 18 は画面の CSS・部品・テストだけの変更で、Rust と依存に変更はない。Step 18 の部品のテストは実装の後に書いた（Testing Contract：GUI の層は実装してからテスト）。追加した 2 つのテストのうち `ConditionArea.test.tsx` のものは、変更前の構造（枠がない）では `condition-area-frame` が見つからず落ちる。

そのほか（Step 17 の後）：`npx tsc --noEmit` OK、`npx eslint .` 0 件、`npx prettier --write .` のあと `--check` OK、`cargo +stable fmt --all --check` OK、`cargo +stable clippy -p local-sights-core --all-targets` 警告 0、`npx vite build` OK、`npm audit --audit-level=high` 0 件。Step 17 は Rust と依存に変更がないため、`cargo-deny` と Tauri 側の clippy には影響なし（Step 16 の後の `cargo +stable clippy --workspace --all-targets` 警告 0 のまま）。

## 追跡（traceability.json）

`GATE-*` はコード生成ゲートでの利用者の修正依頼の ID（`GATE-1`〜`GATE-6` は 1 回目＝Step 14、`GATE-7`〜`GATE-12` は 2 回目＝Step 15、`GATE-13`・`GATE-14` は 3 回目＝Step 16、`GATE-15` は 4 回目＝Step 17、`GATE-16` はその追加＝Step 18）、`REV-*` は計画レビューの指摘の ID。いずれも上流の要件 ID ではなく、対応した実装ファイルかテストファイルを `target` にしている。

## 配布物の大きさ（`npx vite build`、要件レビュー R-05）

| | 変更前 | Step 13 の後 | Step 14 の後 | Step 15 の後 | Step 16 の後 | Step 17 の後 | Step 18 の後 |
|---|---|---|---|---|---|---|---|
| JS | 271.20 kB（gzip 83.96 kB） | 911.18 kB（gzip 249.84 kB） | 913.06 kB（gzip 250.54 kB） | 914.51 kB（gzip 250.92 kB） | 914.51 kB（gzip 250.92 kB、画面のコードは変更なし） | 914.88 kB（gzip 251.06 kB） | 915.01 kB（gzip 251.07 kB） |
| CSS | 3.65 kB（gzip 1.15 kB） | 916.28 kB（gzip 212.34 kB） | 916.62 kB（gzip 212.43 kB） | 916.95 kB（gzip 212.48 kB） | 916.95 kB（gzip 212.48 kB） | 916.95 kB（gzip 212.48 kB） | 917.10 kB（gzip 212.52 kB） |
| `dist/` 合計 | 約 0.3 MB | 約 1.8 MB | 約 1.8 MB | 約 1.8 MB | 約 1.8 MB | 約 1.8 MB | 約 1.8 MB |

Cloudscape の部品・グローバルスタイルと、英・日の文言束（`all.en`／`all.ja`）を静的に取り込んだ分。ローカルアプリのため配信コストはなく、起動は WKWebView のローカル読み込み。Vite の 500 kB 警告は出るが、要件に大きさの上限はない。必要なら `importMessages(locale)` による文言束の遅延読み込みで JS を減らせる（今回は見送り）。
