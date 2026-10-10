# Code Summary — U4 時間範囲とタイムゾーン（u4-time-range）

計画：`code-generation-plan.md`（Plan Approval 済み）。単位テストの手順：`unit-test-instructions.md`。U4 が書いたファイルの一覧は `source-manifest.json`、ルールと要件の対応は `traceability.json`。

## 今回の作業（2026-10-10、既存のコードと計画の照合）

U4 のコードは以前の作業で作られており、その上に U5〜U7 の機能が足されている。ツールを 2.11.0 に上げた後、U4 の計画承認がもう一度求められた。利用者は計画を承認し、U1〜U3 と同じく既存のコードを計画と照合する形で進めた。

- 今回作った・変えた・消したアプリのソース：なし（変更は計画ファイルのチェックボックスだけ）。
- `source-manifest.json` のパス 32 件と、`traceability.json` の `OK` の対象 22 件は、すべて今も存在する。どちらも前回のものを引き続き使う。
- テストを先に書く順序（Step 3・4）は最初のビルドで行われた（下の「テスト先行の証拠（Red）」）。今回は失敗の実行を作り直さず、既存のテストの中身と実行で確かめた。
- 計画の 20 項目のうち次の 3 項目はチェックを付けていない（この環境で計画に書かれたコマンドを実行できないため）：
  - Step 1 の 1 つ目：依存（`chrono-tz` 0.10・`iana-time-zone` 0.1）は入っており、テレメトリ系の依存もない。ただし `cargo-deny` が入っておらず `deny.toml` もないため、`cargo deny check licenses` は実行していない。代わりに配布元の Cargo.toml でライセンスを手で確かめた：chrono-tz 0.10.4（MIT OR Apache-2.0）、phf・phf_shared 0.12.1（MIT）、siphasher 1.0.4（MIT OR Apache-2.0）、iana-time-zone 0.1.65（MIT OR Apache-2.0）。
  - Step 7 の 2 つ目（`cargo build -p local-sights`）：実行して `gdk-sys` の組み立てで失敗した（`gdk-3.0` がない）。
  - Step 9 の 2 つ目（`cargo clippy --workspace --all-targets`・`cargo build -p local-sights` を含む検査）：Tauri の Linux 用の前提ライブラリがないため。

### 手順ごとの結果

| 手順 | 結果 | 計画の文言との違い |
|------|------|--------------------|
| Step 1 骨組みと設定 | 2 つ目は満たしていた。1 つ目は未チェック（上のとおり） | なし |
| Step 2 テストの実行環境 | 満たしていた | なし |
| Step 3・4 純粋なロジック | 満たしていた | なし。time_range・date_input・request・session のテストの中身で計画の観点がすべてあることを確かめた。R-09 の 2 つの例外は `date_input.rs` のモジュールの doc コメントにある |
| Step 5 OS のタイムゾーン | 満たしていた | なし（テスト 4 件、OS の設定に依存しない） |
| Step 6 AppSession | 満たしている（後の単位による違いあり） | 入力の書き換えを拒むのは、取得中に加えて確認待ちと設定ダイアログが開いている間（U6、`ensure_can_change`）。`can_fetch` にも設定ダイアログの条件が加わった（U6）。`DisplayRowWindow` に `filtered`・`allCount`・`resultVersion` が加わった（U5）。`select_time_zone` は設定ダイアログが開いていても受け付ける |
| Step 7 Tauri のつなぎ | 1 つ目は満たしている（後の単位の形）。2 つ目は未チェック | `get_rows` は EventTimeline ではなく LogView から行を取る（U5。ロックの順はセッション → LogView）。capabilities には U6・U7 のコマンドの権限も並ぶ |
| Step 8 画面 | 満たしている（後の単位による違いあり） | FetchForm は絞り込みの入力も持ち、設定ダイアログが開いている間も入力を無効にする（U5・U6）。`useRowWindow` は filterId と resultVersion にも反応する（U5）。計画では「取得中も押せる」を `TimeZoneToggle.test.tsx` に置くことになっていたが、そのファイルにはなく、同じ観点を `ConnectionBar.test.tsx` の "holds the time zone switch, usable while the connection is locked"（取得中）で確かめている。TimeZoneToggle にはそもそも無効にする手段がない |
| Step 9 ビルドと環境 | 1 つ目は満たしていた。2 つ目は未チェック | README の「Time zones (U4)」に夏時間の存在しない日時も入っている |
| Step 10 doc コメントと記録 | 満たしている | コアは `#![warn(missing_docs)]` で警告 0 件。画面側の U4 の公開物にも JSDoc がある。記録はこのファイル |

### ルールの文言といまのコードの違い

`traceability.json` では次の項目も `OK` のままにしている。どれも U4 の内容は保たれており、後の単位が条件を足したもの：

| ID | いまのコードとの違い | そのまま成り立つ部分 |
|----|----------------------|----------------------|
| BR2.1 | [Fetch] を押せない条件に、設定ダイアログが開いている間（U6）が加わった（理由の文言は足していないので R-04 の方針は保たれる） | ほかの条件、理由の並び、誤りがあるとき RangeOrder を出さない |
| BR1.5 | 書き換えを拒むのは取得中に加えて、確認待ち（U2:BR2.6）と設定ダイアログが開いている間（U6） | 同じ文字列なら解釈し直さない |
| BR3.4 | 行は EventTimeline の RowWindow を直接ではなく、U5 の LogView（絞り込みの結果を含む）から取る | AppSession が displayTime を合成し、画面は変換しない |
| BR3.1 | 切替の後も、U7 で開いた展開の行が開いたまま残る（足された振る舞い） | 切り替えても取り直さず、表示だけを描き直す |

### テストと検査の結果（今回）

| コマンド | 結果 |
|----------|------|
| `cargo test -p local-sights-core --lib -- time_range:: time_zone:: date_input:: request:: session::` | 124 件成功 |
| `cargo test -p local-sights-core`（全体） | lib 305 件成功（5 件 ignore）、結合テスト u1 11・u2 9・u3 14・u5 4・u6 3 件成功 |
| `npx vitest run`（U4 の 5 ファイル） | 69 件成功 |
| `npx vitest run`（全体） | 21 ファイル 171 件成功 |
| `cargo fmt --all --check` | 成功 |
| `cargo clippy -p local-sights-core --all-targets` | 成功・警告 0 件（`--workspace` は src-tauri を組み立てられないため実行できない） |
| `npx tsc --noEmit`・`npx prettier --check .`・`npx eslint .` | 成功 |
| `npm audit` | 脆弱性 0 件 |
| `cargo build -p local-sights` | このコンテナでは失敗（`gdk-3.0` がない） |
| `cargo deny check licenses` | 未実行（`cargo-deny` が入っていない） |

### 気づいた点（直していない）

- `deny.toml` と CI の設定がまだないため、team.md の「CI で `cargo-deny` を行う」はまだ満たされていない（CI Pipeline ステージの範囲）。

## 最初のビルドの記録

### 作ったもの・変えたもの

| 場所 | 中身 |
|------|------|
| `Cargo.toml`・`crates/local-sights-core/Cargo.toml` | `chrono-tz`（0.10、既定の機能なし＋`std`）と `iana-time-zone`（0.1、U3 までも chrono 経由でビルドに入っていた）を追加。ほかの依存は足していない |
| `crates/local-sights-core/src/time_range.rs` | 選んだタイムゾーンの壁時計の時刻としての解釈 `parse_in_zone`（存在しない日時は `DateTimeParseError::NonexistentLocalTime`、2 回現れる日時は早い方、BR1.2）、入力欄用の文字列化 `format_input_in_zone`（4 桁の年に表せなければ `None`、BR1.4）、一覧用の文字列化 `format_display_in_zone`（`.mmm` 付き、その瞬間のオフセット、表せなければ数値、BR3.1）。U1 の `parse_utc_seconds`・`format_utc_millis` は確認用プログラムのために残し、形の確認を共通の関数にまとめた |
| `crates/local-sights-core/src/date_input.rs`（新規） | DateTimeInput（text・instant・error）と DateTimeInputError（Format・NonexistentLocalTime）。`parse`（BR1.2）、`edit`（同じ文字列は解釈し直さない、BR1.5・R-06）、`switch_zone`（瞬間を保って文字列を作り直す／瞬間を持たない入力と 4 桁の年に表せない瞬間は文字列を残して解釈し直す、BR1.4・R-02・R-05）。entities の「instant は変わらない」の 2 つの例外（R-09）をモジュールの doc コメントに書いた |
| `crates/local-sights-core/src/time_zone.rs`（新規） | TimeZoneChoice（Local・Utc、既定は Local、BR1.1）と TimeZoneContext（ローカルとして使う `chrono_tz::Tz`、テストで差し替える境界、BR3.4）。`from_name`・`resolve`（名前がない・空・知らない名前は UTC にし、理由を返す）・`detect`（起動時に `iana-time-zone` で OS の名前を読む）。診断の文言はタイムゾーンの名前だけを含む |
| `crates/local-sights-core/src/request.rs` | ValidationError に StartNonexistentLocalTime・EndNonexistentLocalTime を追加（BR2.2）。`ALL` を条件の並びの順に並べ直した。画面用の `validate_session_input`（DateTimeInput の瞬間から範囲と順序、BR1.6、誤りがあれば RangeOrder を出さない、BR2.1）。確認用プログラム用の `validate_fetch_input`（UTC の文字列、BR3.5）は残し、両方が同じ検証（U1:BR1.8）を通る |
| `crates/local-sights-core/src/session.rs` | SessionState の FetchInput を、タイムゾーン（time_zone）と開始・終了の DateTimeInput に置き換えた。`with_catalog_and_time_zones`、`update_input`（いまのタイムゾーンで解釈、同じ文字列なら何もしない）、`select_time_zone`（取得中・確認待ちでも受け付け、phase を変えない、BR1.4）、`display_rows`（EventTimeline の行に displayTime を付けた DisplayRowWindow を返す、R-08）。SessionView に `timeZone`・`startInput`・`endInput` |
| `crates/local-sights-core/examples/fetch_check.rs` | 新しい 2 つの理由の英語の説明を足しただけ（コンパイルのため）。引数と出力は UTC のまま（BR3.5） |
| `src-tauri/` | 起動時に `TimeZoneContext::detect()` で OS のタイムゾーンを読み、使えなければ診断ログ（標準エラー）に名前だけを出して UTC にする。コマンド `select_time_zone` を追加（`build.rs` の権限の一覧、`capabilities/default.json`、生成された `permissions/autogenerated/select_time_zone.toml`）。`get_rows` はセッション → 保持ログの順にロックし、AppSession が displayTime を付けた行を返す |
| `src/` | `api.ts`（TimeZoneChoice・DateTimeInput・DisplayRow、SessionView の変更、`selectTimeZone`）、`components/TimeZoneToggle.tsx`（新規、標準のラジオボタン 2 つ、無効にしない）、`ConnectionBar.tsx`（上部バーに置く）、`FetchForm.tsx`（ラベルと理由の文言にいまのタイムゾーン、コアの文字列を表示）、`LogTable.tsx`（見出しにタイムゾーン、各行は displayTime をそのまま表示）、`hooks/useRowWindow.ts`（timeZone の変化で取り寄せ直す）、`App.tsx`（切替の転送、タイムゾーンが変わったら FetchForm を作り直す）、`i18n/messages.ts`（`{zone}` 付きの文言、存在しない日時の文言、タイムゾーンの名前、英日）、`format.ts`（`formatUtcMillis` を削除）、`styles.css`（上部バーでの並べ方だけ） |
| `README.md` | 状態の説明、入力とタイムゾーンの説明、U4 の手元の確認項目、確認用プログラムは UTC のままであること、テストは OS のタイムゾーンに依存しないこと |

### 主な判断

- タイムゾーンの変換はすべてライブラリが持つ（BR3.4）。画面は数値の時刻を文字列にしない。`formatUtcMillis` を消し、ログの行は `get_rows` で displayTime 付きで受け取る。
- displayTime の合成は AppSession が行い、EventTimeline には新しい依存を足さない（R-08）。切替では timelineVersion を変えず、画面は SessionView の `timeZone` の変化を合図に表示範囲の行を取り寄せ直す（`useRowWindow` の依存に timeZone を足した）。位置を保つ処理（`find_row_position`）は timelineVersion だけに反応するため、切替では動かない。
- 2 回現れる日時は `LocalResult::Ambiguous` の小さい方の瞬間。存在しない日時は `LocalResult::None` で、形式の誤りとは別の理由にする。
- 入力欄と一覧の「表せる範囲」は、どちらも選んだタイムゾーンで 0000〜9999 年。一覧は U1 と同じく数値をそのまま出す。U1 の画面の `formatUtcMillis` も 4 桁の年でない場合は数値にしていたので、見え方は変わらない。
- 空の入力は、U1 と同じく形式の誤り（StartFormat・EndFormat）として [Fetch] を押せない理由に入る。
- [Fetch] を押せない理由の並びは、選択の理由（プロファイル → リージョン → ロググループ）、開始の入力、終了の入力、順序の順。取得中と確認待ちは新しい文言を足さず、`validation_errors` は空のまま `can_fetch` だけが false になる（R-04）。
- `select_time_zone` は失敗しないため、Tauri のコマンドは何も返さず `session-changed` で新しい状態を送る。同じタイムゾーンを選んだときは何もしない（遅い方の瞬間を保つため）。
- 画面の入力欄は打鍵中の下書きを持つため、タイムゾーンが変わったら `key` を変えて FetchForm を作り直し、コアが作り直した文字列を取り込む。切替の操作中は焦点がラジオボタンにあるため、入力中の文字は失われない。
- `AppSession::new()`・`with_catalog()` のローカルは UTC（テストの既定。U1〜U3 のテストはそのまま UTC の値で通る）。アプリは `with_catalog_and_time_zones` で OS のタイムゾーンを渡す。
- タイムゾーンの切替は標準のラジオボタン（`fieldset` と `legend`）。Tab で入り、矢印キーとスペースで選べるのはブラウザの標準の動き（BR3.3、NFR13）。
- 秘密情報：診断ログに出すのは「OS のタイムゾーンが読めない」またはタイムゾーンの名前（Debug 形式でエスケープ）だけ。U4 は AWS の API を新しく呼ばない。

### テストの結果（最初のビルド）

| コマンド | 結果 |
|----------|------|
| `cargo test --workspace` | 成功（lib 203 件成功・2 件 ignored、u1_fetch_flow 11、u2_log_group_listing 9、u3_fetch_flow 14） |
| `cargo test -p local-sights-core --lib -- time_range:: time_zone:: date_input:: request:: session::`（U4 の単位のコマンド） | 97 件成功 |
| `npx vitest run` | 12 ファイル 91 件成功 |
| `npx vitest run src/components/TimeZoneToggle.test.tsx src/components/LogTable.test.tsx src/components/FetchForm.test.tsx src/App.test.tsx src/i18n/messages.test.ts`（U4 の単位のコマンド） | 5 ファイル 45 件成功 |
| `cargo fmt --check`・`cargo clippy --workspace --all-targets` | 成功・警告 0 |
| `npx tsc --noEmit`・`npx prettier --check .`・`npx eslint .` | 成功 |
| `npm audit` | 脆弱性 0 件 |
| `cargo build -p local-sights` | 成功 |

U4 で足したテスト：time_range +8、date_input 9、time_zone 4、request +5、session +12（[Fetch] の条件 5、切替と表示の行 7）、画面側 TimeZoneToggle 5、ConnectionBar +1、LogTable +2、FetchForm +3、App +1、messages +2。U3 の時点の lib 165（+ignored 2）・Vitest 77 から、lib 203（+ignored 2）・Vitest 91。U4 で意図して書き直した既存のテスト：`session.rs` の 3 か所（入力の取り出しを DateTimeInput に、SessionView の JSON の形）、`LogTable.test.tsx`（コアの displayTime を出す、見出し「Time (Local)」、`formatUtcMillis` の確認を削除）、`App.test.tsx`（日本語の見出し「時刻（ローカル）」）、`ConnectionBar.test.tsx`（新しい props）、`src/test/fixtures.ts`（SessionView の新しい形、行に displayTime）。

### テスト先行の証拠（Red）

Testing Contract の `ordering` のとおり、純粋なロジック（選んだタイムゾーンでの解釈と文字列化、入力欄の切替と書き換え、瞬間からの範囲と理由、[Fetch] の理由の並び）はテストを先に書き、実行して失敗を確かめてから実装した。

1. `time_range.rs`・`date_input.rs`・`request.rs`・`session.rs` にテストだけを書いた状態（`date_input.rs`・`time_zone.rs` はモジュールの doc コメントだけ）で `cargo test -p local-sights-core --lib -- time_range:: time_zone:: date_input:: request:: session::` を実行 → コンパイルの失敗 `error: could not compile local-sights-core (lib test) due to 74 previous errors`。主な出力：`cannot find function parse_in_zone`（7）、`cannot find function format_input_in_zone`（7）、`cannot find function format_display_in_zone`（6）、`cannot find function validate_session_input`（6）、`cannot find type DateTimeInput`（13）、`cannot find type DateTimeInputError`（3）、`cannot find type DateTimeParseError`（2）、`no variant ... StartNonexistentLocalTime`（3）・`EndNonexistentLocalTime`（4）、`no associated function ... with_catalog_and_time_zones`（1）、`unresolved import crate::time_zone::TimeZoneContext`（1）。
2. 実装の後、同じコマンドで 86 件成功（Step 4 の時点。直したのは U1 のテストの SessionView の JSON の形の 1 件だけで、意図した変更）。

OS のタイムゾーンの読み取り（`time_zone.rs` の `resolve`・`detect`）、AppSession のつなぎ（切替・表示の行）、Tauri、画面は、実装してからテストを書いて実行した。Tauri には自動テストを置かず、`cargo build -p local-sights` で組み立てを確かめた。

### 計画との違い（最初のビルド）

1. Red はコンパイルの失敗として記録した（U3 のような `todo!()` の骨組みは作らず、まだない関数・型を呼ぶテストを先に書いた）。
2. Step 3 の `session.rs` のテストがローカルのタイムゾーン（New York）を必要とするため、Step 4 で TimeZoneChoice と TimeZoneContext の値の部分（`new`・`utc`・`local`・`zone`）だけを先に作った。OS の名前の読み取り（`from_name`・`resolve`・`detect`）は Step 5 で実装してからテストを書いた。
3. 確認用プログラム（`examples/fetch_check.rs`）は「変えない」計画だったが、ValidationError に種類を足したため、`describe` の `match` に 2 行を足さないとコンパイルできなかった。引数・出力・動作は UTC のまま変わらない（UTC は存在しない日時を持たないため、この 2 つの理由は出ない）。
4. `validate_fetch_input` は残したうえで、`validate_session_input` と共通の内部関数にまとめた（UTC の文字列を DateTimeInput にしてから同じ検証を通す）。`FetchInput` は確認用プログラムと結合テストのために残した。
5. Step 7 の対象に `src-tauri/build.rs` を足した（Tauri のコマンドの権限の一覧に `select_time_zone` を入れないと、権限が生成されない）。
6. Step 8 で、計画のテストの一覧にない `ConnectionBar.test.tsx` を新しい props に合わせて直し、「接続先を変えられない間も切替は使える」テストを 1 件足した。`styles.css` に上部バーでの並べ方だけを足した。
7. テストの件数は、`date_input.rs` が 9 件（目安 7〜8）、`session.rs` が 12 件（目安 6〜7）で目安より多い。少なくはしていない。
8. 作業中にディスクが一杯になり `cargo build` が失敗したため、ビルドの一時ファイル `target/debug/incremental`（約 4.2 GB、作り直せるキャッシュ）を消し、以後は `CARGO_INCREMENTAL=0` でビルドした。コードの変更ではない。

## まだ確かめていないこと・未解決

- `cargo build -p local-sights`・`cargo clippy --workspace --all-targets`・`cargo deny check licenses` は、Tauri の前提ライブラリと `cargo-deny` がある手元か CI で確かめる（2026-10-10 の照合ではこのコンテナで実行できなかった）。確かめたら計画の Step 1 の 1 つ目、Step 7・Step 9 の 2 つ目にチェックを付ける。

- 画面での見た目と操作の確認（切替で入力欄と一覧の時刻が変わる、取得中の切替、夏時間のある地域での存在しない日時の表示）は、開発者本人の手元で行う（`README.md` の「Time zones (U4)」）。
- OS のタイムゾーンは起動時に 1 回だけ読む。アプリの実行中に OS のタイムゾーンを変えた場合は、再起動するまで反映されない（BR1.1 の「起動のたびに」の範囲内だが、利用者向けの説明は README だけ）。
- 夏時間の終わりで 2 回現れる時刻の遅い方の瞬間は、ローカルでは早い方と同じ文字列で表示され、画面の上では区別できない（R-06 で決めたとおり瞬間は保たれる）。
- リポジトリにはまだ `deny.toml` と CI の設定がないため、`cargo-deny` は実行していない。追加したクレートのライセンスは手で確かめた：`chrono-tz` MIT OR Apache-2.0、`phf`・`phf_shared` MIT、`siphasher` MIT OR Apache-2.0（`iana-time-zone` MIT OR Apache-2.0 は U3 までもビルドに入っていた）。
- 作業環境のディスクの空きが少ない（作業の終わりで約 2.4 GB）。
