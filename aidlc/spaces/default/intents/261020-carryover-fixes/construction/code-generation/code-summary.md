# Code Generation の記録（261020-carryover-fixes）

作業単位はない（bugfix のスコープ）。承認済みの計画 `code-generation-plan.md` の 38 の手順をすべて終えた。入力は `inception/requirements-analysis/requirements.md` と、コードの知識ベース `aidlc/spaces/default/codekb/local-sights/`。

## 作ったもの・変えたもの

| 要件 | ファイル | 変えたこと |
|---|---|---|
| FR1 | `crates/local-sights-core/tests/u5_cancel.rs` | 取得の前に中断を送る補助関数 `fetch_already_stopped` を足し、`a_cancel_during_the_cache_lookup_completes_a_hit_and_cancels_a_miss` をそれに替えた。製品のコードは変えていない（OQ1） |
| FR2.1 | `crates/local-sights-core/src/cache/plan.rs` | 純粋な関数 `last_overlapping_range` と `EventCounter::verify_through` を足した |
| FR2.1 | `crates/local-sights-core/src/cache/mod.rs` | `EventSelection::Through { last }` を足し、`read_overlaps` は重なる一番後ろの範囲の終わりで読むのをやめ、そこまでの範囲の件数を確かめる。`lookup`・`write` の読み方とファイルの形は変えていない（A4） |
| FR2.2 | `crates/local-sights-core/src/query/store.rs` | `begin_run` で、その実行で選ばれていないロググループの保持（イベントと範囲）を捨てる。冒頭の説明と `Holding::Committed` の説明を直した |
| FR2.3 | `crates/local-sights-core/src/query/evaluator.rs`、`query/runner.rs`、`insights_session.rs`、`session.rs` | `QueryResult`・`ResultSummary`・`QueryRowWindow`・`QueryRowDetail` の `holdings_epoch` と、`QueryResult::new`／`from_output`・`display_rows`・`display_row_detail`・`display_query_rows`・`display_query_row_detail` の引数を取り除いた。`QueryStore` の内部の `epoch` は残した（OQ2） |
| FR2.3、FR4.3 | `src-tauri/src/lib.rs` | `get_query_rows`・`get_query_row_detail` と補助関数の引数を取り除いた。権限のテストは数の固定をやめ、比べを `same_commands` に切り出した |
| FR2.3 | `src/api.ts`、`src/components/QueryResultTable.tsx`、`src/test/fixtures.ts` | 型と呼び出しから `holdingsEpoch` を取り除き、古い応答は `runId` だけで見分ける |
| FR3.1 | `crates/local-sights-core/src/query/plan.rs` | `DEFAULT_RESULT_LIMIT` の説明を `has_row_limit` の決まり（`limit 0` も含めて `limit` の値で切る）に合わせた |
| FR3.2 | `crates/local-sights-core/examples/compat_check.rs`、`aidlc/spaces/default/intents/261017-insights-query/construction/u2-query-language/code-generation/code-summary.md` | U4 の説明に、今のアプリは `limit 0` で 0 行で、両側 0 行のため比べに影響しないことを足した。U2 の部品の表を「`limit 0` は 0 行、Loop-back 3 の p08」に直した |
| FR3.3 | `src/elapsed.ts`、`aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/code-generation/code-summary.md` | `nextAnchor` の説明と U4 の記録に、番号を等しいかどうかだけで比べるため経過時間が一度だけ戻ることがある点を書いた。振る舞いは変えていない |
| FR3.4 | `docs/releasing.md` | 「次の版」の段落を、公開済みの版の一覧とタグの直前に番号を決めることに替え、例の番号を `X.Y.Z`／`vX.Y.Z` にした |
| FR4.1 | `src/i18n/messages.ts` | `englishSingular`（6 つのキー → 決め手の数の名前と単数形の文）を足し、`translate` の中の 1 か所（`templateOf`）で、英語で決め手の数がちょうど 1 のときだけ単数形を使う。`messages` の形は変えていない |
| FR4.2 | `src/components/CancelNotice.tsx`、`src/App.tsx`、`src/styles.css` | `role="status"` の囲みを常に置き、知らせがあるときだけ中身を入れる。空の囲みは `.cancel-notice:empty { position: absolute; }` で列の並びから外し、余白を出さない |
| FR5 | `src/i18n/regions.ts`（新規）、`src/components/TopBar.tsx`、`src/App.tsx`、`src/i18n/messages.ts`、`src/styles.css` | 34 個の名前の表 `REGION_NAMES`、`regionLabel`、`filterRegionOptions` を足した。リージョンの選択欄を Cloudscape の `Select`（`filteringType="manual"`）に替え、`TopBar` に画面の言語 `locale` を渡す。新しい文 `connection.region.noMatch`・`connection.region.filterPlaceholder`。選択欄の幅は `.top-bar-region` |

テストのファイル：`cache/plan.rs`・`cache/mod.rs`・`query/store.rs`・`query/runner.rs`・`query/evaluator.rs`・`insights_session.rs`・`session.rs`・`tests/u5_cancel.rs` の中のテスト、`src-tauri/src/lib.rs` のテスト、`src/i18n/messages.test.ts`、`src/i18n/regions.test.ts`（新規）、`src/components/TopBar.test.tsx`、`CancelNotice.test.tsx`、`QueryResultTable.test.tsx`、`LogAnalyticsPage.test.tsx`、`StatusLine.test.tsx`、`src/App.test.tsx`。

## 主な実装の判断

- **FR1（OQ1）**：`run_fetch_with_cache` は照合（`look_up`）が答えるまで中断を見ない。そのため取得の前に `StopHandle::stop` を呼んでおくことは、製品のコードから見て「照合が答える前に中断が届いている」状態と同じになる。テストの道具だけで順番が決まり、時計や実スレッドの速さに依らない。製品のコードへのテスト用の入口は要らなかった。
- **FR2.1 の読み止め**：重なる記録済みの範囲のうち一番後ろのもの（ファイルの時刻順で最後）の終わりを過ぎた行で読むのをやめる。読み止めた行も、読んだ行と同じく形と順の確かめを通す（今の `Range` の読み方と同じ）。件数は、その範囲までのすべての範囲について見出しと比べる。結果は今と同じく `overlap_ranges` の部分だけを残すため、全部読んだときと同じになる（テストで比べた）。
- **FR2.2**：`begin_run` の最初に `groups.retain(|key, _| keys.contains(key))` で選ばれていないものを捨てる。選ばれ続けるものは今のまま（staging だけ捨てる）。`discard_all` は変えていない。
- **FR2.3（OQ2）**：`QueryStore` の `epoch` は `view_epoch`・`copy_rows`・`view_len`・`build_view`・`evaluate_step` で前の実行の表示用の保持を使わないために要るため残した。外に出る項目と引数だけを取り除いた。画面は接続を変えると結果が消えるため、同じ `runId` が別の保持を指すことはない。
- **FR4.1**：単数形は英語の別の表に置き、`translate` の 1 か所で選ぶ。決め手の数は `Number(...) === 1` で見るため、文字列の `"1"` も単数になる。0 は複数形。複数形の文（`were`・`could not`）は今のままでそろっている。
- **FR4.2**：囲みが中身と一緒に現れる作りでは読み上げが起きない読み上げソフトがあるため、囲みを先に置く。空の囲みは flex の列の `gap` で隙間を作るため、`:empty` のときだけ `position: absolute` にして並びから外した。要素そのものは画面に残る。
- **FR5 の部品**：Cloudscape の自動の絞り込みは全角・半角の括弧の違いを吸収しないため、`filteringType="manual"` で自分の `filterRegionOptions` を使う。Cloudscape は打った文字を少し遅らせて `onLoadItems` で知らせるため、その文字を `TopBar` の状態に持ち、候補を作り直す。手動の絞り込みでは候補なしのとき `empty` が出るため、`empty` と `noMatch` の両方に `connection.region.noMatch` を渡した。今の接続のリージョンは `selectedOption` で、Select の標準の選択の印が付く。
- **FR5.10（R-08）**：`regions.test.ts` は `regions.rs` を `?raw` で読み、`pub const STANDARD_REGIONS: &[&str] = &[` から次の `];` までの `"…"` を取り出す。取り出せないとき・0 件のときは例外で落とす（そのこと自体もテストした）。

### リージョンの名前の資料（Step 29、A5、OQ3）

AWS の公式の資料「AWS Regions」の英語版 <https://docs.aws.amazon.com/global-infrastructure/latest/regions/aws-regions.html> と日本語版 <https://docs.aws.amazon.com/ja_jp/global-infrastructure/latest/regions/aws-regions.html>（2026-10-21 に AWS のドキュメントの読み取りの道具で読んだ）の「Available AWS Regions」の表で、`STANDARD_REGIONS` の 34 個すべての英語と日本語の名前を確かめた。資料の表も 34 個で、`STANDARD_REGIONS` と過不足なく一致した。確かめられなかった名前はない。括弧はどちらの版も半角で、前に半角の空白がある（例：「アジアパシフィック (東京)」「米国東部 (バージニア北部)」）。日本語版のページには「機械翻訳により提供」の注記があるが、表の名前はそのまま使った。

## テスト

### Red（テスト先行の手順で、実装の前に落ちたこと）

| 手順 | 落ち方 |
|---|---|
| Step 6（FR2.1 の判断） | コンパイルが通らない：`error[E0425]: cannot find function last_overlapping_range` が 8 件、`error[E0599]: no method named verify_through found for struct EventCounter` が 5 件 |
| Step 10（FR2.2） | 2 件が落ちた：`a_log_group_no_longer_chosen_is_discarded_at_the_next_run`（`left: (1, 0)`、`right: (0, 0)`：B の保持が残る）、`a_log_group_chosen_again_is_fetched_again_while_the_others_keep_their_ranges`（`left: [200..299]`、`right: [100..299]`：B を取り直さない） |
| Step 22（FR4.1） | 2 件が落ちた：`Expected: "Listing streams… 1 stream selected so far"`／`Received: "Listing streams… 1 streams selected so far"`、`Expected: "Failed in 1 stream"`／`Received: "Failed in 1 streams"`。複数形・日本語・対象外の文の 3 件は今のままで通った（変えないことの確かめ） |
| Step 30（FR5.4、FR5.10） | `Failed to resolve import "./regions"`（まだないモジュール） |

### 200 回の結果（FR1.3）

`for i in $(seq 1 200); do cargo test -q -p local-sights-core --test u5_cancel -- a_cancel_during_the_cache_lookup || break; done` を流し、200 回とも通った（`passed runs: 200`）。

### 流したコマンドと結果

| コマンド | 結果 |
|---|---|
| 変更前の基準：`cargo test --workspace`、`npx vitest run` | すべて通過。ライブラリの単体テスト 637 件（無視 9 件）、アプリ 31 件、Vitest 41 ファイル 469 件 |
| `cargo test -p local-sights-core --lib cache::` | 57 件通過（基準 46 件＋ 11 件） |
| `cargo test -p local-sights-core --lib query::store`、`query::runner`、`query::` | 通過（`query::` 137 件） |
| `cargo test -p local-sights-core --test u5_cancel` | 11 件通過 |
| `cargo test -p local-sights --lib tests::the_manifest_the_capability_and_the_handler_list_the_same_commands`、`tests::same_commands` | 1 件、3 件通過 |
| `cargo test --workspace` | 762 件通過、失敗 0、無視 9（ライブラリの単体テストは 650 件、アプリは 34 件） |
| `cargo test --release -p local-sights-core --lib -- --ignored`（NFR4） | 9 件すべて通過（9.54 秒） |
| `npx vitest run src/i18n src/components/TopBar.test.tsx src/components/CancelNotice.test.tsx src/components/QueryResultTable.test.tsx src/App.test.tsx` ほか触った画面のテスト | 通過 |
| `npm run test:coverage` | 42 ファイル 496 件通過。行カバレッジ 90.01%（閾値 80%、変えていない） |
| `cargo fmt --all --check`、`cargo clippy --workspace --all-targets` | 合格、警告 0 |
| `npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit` | 合格 |
| テスト以外のコードの `unwrap()`／`expect()` | 足していない（差分の追加の行で見つかったものはすべてテストの中） |
| 呼ぶ API（NFR5、Step 37） | 追加の行に AWS の SDK の呼び出し・ネットワークへの取得はない。依存関係の設定（`Cargo.toml`・`Cargo.lock`・`package.json`・`package-lock.json`）は変えていない。リージョンの名前は組み込みの表 |

足したテストは、Rust がライブラリ 13 件（`cache/plan.rs` 7、`cache/mod.rs` 4、`query/store.rs` 1 件を直し 1 件を足す、`query/runner.rs` 1）とアプリ 3 件（`same_commands`）、画面が 27 件（`messages.test.ts` 6、`regions.test.ts` 10、`TopBar.test.tsx` 8、`CancelNotice.test.tsx` 2、`App.test.tsx` 1）。

## 計画との違い

1. **既存のテストの期待値を NFR1 の 2 か所（FR2.2.5・FR4.3）のほかでも直した。** どれも、この作業で直す振る舞いそのものを固定していたもの。
   - FR4.1：`src/components/StatusLine.test.tsx` の 2 か所（「Failed in 1 streams」→「Failed in 1 stream」）と `src/components/LogAnalyticsPage.test.tsx` の 1 か所（「1 streams could not be fetched」→「1 stream could not be fetched」）。数が 1 のときの今の英語の誤りを期待していた。
   - FR4.2：`src/App.test.tsx` の「知らせが消えると `cancel-notice` の要素がなくなる」を「同じ要素が残って中身が空になる」に替えた。別のテストの待ち合わせ（`findByTestId("cancel-notice")`）は、囲みが最初からあるため `cancel-notice-text` を待つ形にした。
   - FR2.3：`src-tauri/src/lib.rs` の「`holdingsEpoch` が送り返される」の確かめを「送られない」に替え、「コピーは世代に依らない」の確かめ（世代の引数がなくなった）を消した。`QueryResultTable.test.tsx` の「別の世代の応答を捨てる」は「別の `runId` の応答を捨てる」にした（Step 16 の古い `runId` のテスト）。
   - FR4.3：数の固定とともに、`cancel_fetch` が入っていることの確かめも消し、要件の「同じであることと、どれも空でないことだけ」にそろえた。
2. **計画の「触るもの」にないファイルも直した。** `src/components/LogAnalyticsPage.test.tsx`（`getQueryRows` の引数、FR2.3）、`src/components/StatusLine.test.tsx`（上の 1）、`src/App.tsx`（`TopBar` に `locale` を渡す）、`src/styles.css` の `.cancel-notice:empty`（FR4.2 の余白）。
3. **`filterRegionOptions` は言語の引数を持たない。** 計画は `filterRegionOptions(codes, text, locale)` としていたが、FR5.4 でどちらの言語の画面でも両方の名前に一致させるため、言語を使う場所がない。使わない引数を置かず `filterRegionOptions(codes, text)` にした。
4. **`TopBar` に `locale` を足した。** 名前を画面の言語で出すため。呼び出し側は `App` だけ。
5. **`connection.region.filterPlaceholder`（「Find a region」／「リージョンを検索」）も足した**（計画では「必要なら」）。
6. **FR2.1 の「読んだ行数を確かめる」は、読まない部分の行を壊しても気づかないことで確かめた**（計画の Step 9 のとおり）。行を数える入口は製品のコードに足していない。
7. **キーボードの操作のテストは `fireEvent.keyDown` に `keyCode` を付けた。** Cloudscape はキーを `keyCode` で見るが、user-event のキーには `keyCode` がない（App のテストの注記と同じ事情）。
8. **この環境だけの準備。** `node_modules` がなかったため `npm ci` を流した。Tauri のアプリのビルドに要る GTK・WebKit の開発用ライブラリがなかったため、CI（`.github/workflows/ci.yml`）と同じ `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev` を `apt-get` で入れた。リポジトリの設定は変えていない。

## 残っていること

- **ライブラリの行カバレッジは手元で測れていない。** この環境に `cargo-llvm-cov` が入っていない。足した関数（`last_overlapping_range`、`verify_through`）はテストで通っている。CI の Linux のランナーの `cargo-llvm-cov`（80%）で確かめる。
- **Windows の手元の確認は省く（A1）。** macOS の手元の確認（A2：リージョンの選択欄、英語の単数・複数、中断の知らせ、クエリの画面の実行の目視）は Build and Test で行う。
- `.cancel-notice:empty` で囲みを並びから外したときの見た目（上の余白が今と同じであること）は jsdom では測れないため、macOS の目視で確かめる。

## Assumptions & Open Questions

None.
