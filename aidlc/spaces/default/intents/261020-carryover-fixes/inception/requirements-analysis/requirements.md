# 要件：持ち越し Minor の一括修正（261020-carryover-fixes）

## 意図の分析

IB-11（Logs Insights 互換のクエリ、261017-insights-query）と parallel-fetch（261012）の作業で「次に回す」とした小さな直しを、1 本の短いブランチでまとめて片づける。新しい機能を足すのではなく、土台をそろえる作業である [desc]。

ねらいは次の 3 つ。

- **CI を信頼できる状態に戻す。** 時間に依存して落ちる結合テストがあり、Dependabot の PR #32 でも一度 CI を赤にした。たまたまの赤を見過ごす癖を付けないため、確実に通るようにする。
- **クエリの画面（Logs Insights の画面）の持ち方を正す。** キャッシュの読みすぎ、保持の増えすぎ、使われていない番号の受け渡しを直し、メモリと読み込みの無駄を減らす。
- **記述と画面の小さな食い違いを直す。** 古い説明、英語の複数形、読み上げ、固定値のテスト、リージョン名のない選択欄。

利用者に見える変更は、英語の単数・複数（FR4.1）、中断の知らせの読み上げ（FR4.2）、クエリの画面の進み具合の置き場所（FR4.4、Build and Test の目視で出た追加の依頼）、リージョンの選択欄の名前と絞り込み（FR5）だけである。この作業だけでは配らず、次の機能と合わせて配る（team.md Deployment、Practices Discovery Q2）。

種類：不具合の修正と内部の整理（bugfix）。範囲：ライブラリ（`crates/local-sights-core`）・アプリ（`src-tauri`）・画面（`src`）・手順書（`docs/releasing.md`）の 4 か所。複雑さ：標準。

## Functional Requirements

### FR1 不安定な結合テストの安定化（依頼 (1)）

対象：`crates/local-sights-core/tests/u5_cancel.rs` の `a_cancel_during_the_cache_lookup_completes_a_hit_and_cancels_a_miss`。

原因（コードの読み直しで確かめた）：キャッシュの照合は `spawn_blocking` の実スレッドで走る（`coordinator.rs` の `look_up`）。テストは止めた時計の上で `Duration::ZERO` のあとに中断を送るが、照合の答えが中断より先に返ると、取得は一覧（DescribeLogStreams）に進み、台本のない偽のゲートウェイが落ちる。

- **FR1.1** テストは、照合が答える前に中断が必ず届いている状態を、テストの道具で作ってから確かめる（Q1 A）。中断と照合の順番は、時刻や実スレッドの速さに依らず決まる。
- **FR1.2** 製品のコード（`coordinator.rs` などライブラリの本体）は変えない（Q1 A）。テストが確かめる振る舞い（NFR6.19：照合中に中断が来たとき、当たりは全部を表示して Completed、外れは一覧の前に中断して AWS を一度も呼ばない）は今のまま。
- **FR1.3** 直したテストは、同じテストを続けて何度流しても毎回通る。

受け入れ基準

- Given 照合が当たる場面、When 照合の答えより前に中断が届いている、Then 取得は Completed で、キャッシュの 2 件が表示され、AWS は呼ばれない。
- Given 照合が外れる場面、When 照合の答えより前に中断が届いている、Then 取得は Cancelled（一覧の前の中断）で、AWS の呼び出しの記録は空、表示は 0 件。
- Given 直したテスト、When 手元で 200 回続けて流す（`cargo test -p local-sights-core --test u5_cancel -- a_cancel_during_the_cache_lookup` を繰り返す）、Then 200 回とも通る。

### FR2 クエリの画面の保持の持ち方（依頼 (2)）

対象：Logs Insights の画面（クエリの画面）だけ。ログの画面の保持（EventTimeline）は取得のたびに入れ替わるため、対象外（Q3 での確認）。

#### FR2.1 キャッシュの一部を読むときの読みすぎを直す（Q2 A）

- **FR2.1.1** `LogCache::read_overlaps` は、求める範囲と重なる記録済みの範囲のうち、時刻が一番後ろのものの終わりまで読んだら、その先の行を読まない。
- **FR2.1.2** 読んだ部分については、今と同じ確かめ（見出しの形、行の順と位置、読んだ範囲ごとの件数が見出しと合うこと）を行う。読まない部分の件数は確かめない。
- **FR2.1.3** 重なる範囲がないときは、今と同じく見出しの行だけを読む。
- **FR2.1.4** 読んだ部分が壊れていれば、今と同じくファイルを消し、呼び出し側はその部分を AWS から取る。読めないファイル（権限など）は、今と同じく消さずに残す。
- **FR2.1.5** 返す結果（重なる範囲と、その範囲のイベント）は、ファイル全体を読んだときと同じになる。

受け入れ基準

- Given 3 つの記録済みの範囲を持つキャッシュのファイル、When 1 つ目の範囲とだけ重なる範囲を求める、Then 結果は 1 つ目の範囲のイベントだけで、2 つ目以降の行は読まれない（テストで読んだ行数を確かめる）。
- Given 重なる範囲の後ろの行が壊れたファイル、When 前の範囲とだけ重なる範囲を求める、Then 壊れた行を読まずに結果が返る（読まない部分の壊れは気づかない）。
- Given 重なる範囲の中の行が壊れたファイル、When 求める、Then 今と同じく Corrupt を返し、ファイルは消される。

#### FR2.2 選ばれなくなったロググループの保持を捨てる（Q3 A）

- **FR2.2.1** クエリの実行が始まるとき（`QueryStore::begin_run`）、その実行で選ばれていないロググループの保持（取得済みのイベントと記録済みの範囲）を捨てる。
- **FR2.2.2** 捨てたロググループを次の実行でもう一度選ぶと、今の取り足しの仕組みどおり、キャッシュ（有効なとき）か AWS から取り直す。
- **FR2.2.3** 選ばれ続けているロググループの保持は、今と同じく残し、足りない範囲だけを取り足す。
- **FR2.2.4** 接続を変えたときに全部を捨てる今の振る舞い（`discard_all`）は変えない。
- **FR2.2.5** 今の振る舞いを固定しているテスト（選ばれなくなったものも残ることを確かめるもの）は、新しい決まりに合わせて直す。

受け入れ基準

- Given ロググループ A と B で実行したあと、When A だけで実行する、Then B の保持は捨てられ、A の保持は残る。
- Given 上のあと、When もう一度 A と B で実行する、Then B は取り直され（キャッシュか AWS）、A は足りない範囲だけを取る。
- Given どのロググループの保持もある、When 接続を変える、Then 今と同じく全部が捨てられる。

#### FR2.3 使われていない `holdings_epoch` を取り除く（Q4 A）

- **FR2.3.1** クエリの結果（ライブラリの `QueryResult`、`ResultSummary` など）から `holdings_epoch` の項目を取り除く。
- **FR2.3.2** 行と行の詳細を取りに行く Tauri のコマンド（`get_query_rows`、`get_query_row_detail`）と、ライブラリ側の対応する関数の引数から `holdings_epoch` を取り除く。
- **FR2.3.3** 画面（`src/api.ts`、`QueryResultTable` など）の受け渡しから `holdings_epoch` を取り除く。
- **FR2.3.4** 行と行の詳細の中身・並び・件数は、今と変わらない。
- **FR2.3.5** QueryStore の中で世代を数える仕組みが、`holdings_epoch` の受け渡しとは別の目的（古い表示の判定など）でまだ要るなら、内部には残してよい。外に出る項目と引数だけを取り除く。

受け入れ基準

- Given 取り除いたあと、When クエリを実行して結果の行と詳細を取る、Then 取り除く前と同じ行と詳細が返る（既存のテストがすべて通る）。
- Given 取り除いたあと、When ライブラリ・アプリ・画面のコードを `holdings_epoch` で検索する、Then 外に出る項目と引数には残っていない。

### FR3 記述の古さを直す（依頼 (3)）

- **FR3.1** `crates/local-sights-core/src/query/plan.rs` の `DEFAULT_RESULT_LIMIT` の説明を、`has_row_limit` の決まり（`limit` のコマンドがあれば、`limit 0` も含めてその値で切る。`limit` がないときだけ既定の上限で切る）と合わせる。
- **FR3.2** 261017-insights-query の記録のうち、U2 の `code-summary.md` の部品の表に残る「`limit 0` は絞りなし」の記述を、今の決まり（`limit 0` は 0 行）に直す。U4 の確認用プログラム（`examples/compat_check.rs`）が `limit 0` を絞りなしとして扱う点は、比べに影響しないため直さず、その旨を説明に書く。
- **FR3.3** `src/elapsed.ts` の `nextAnchor` の説明に、取得の番号を「等しいかどうか」だけで比べるため、起動直後に古い `getSession` の応答が遅れて届くと新しい取得として受け取られ、経過時間が一度だけ戻ることがある、と書く（Q5 A）。振る舞いは変えない。parallel-fetch U4 の記録（`code-summary.md`）にも同じ点を残す。
- **FR3.4** `docs/releasing.md` の「次の版」の段落を、特定の番号を書かない形にする：公開済みの版（`v0.1.0`・`v0.1.1`・`v0.2.0`）の一覧と、次の版の番号はタグを打つ直前に決めること（team.md Deployment）。手順の中の例の番号（`0.2.0`、`v0.2.0`）は `X.Y.Z` のような置き場所にする（Q6 A）。

受け入れ基準

- Given 直したあと、When `DEFAULT_RESULT_LIMIT` の説明を読む、Then `limit 0` が 0 行であることと矛盾しない。
- Given 直したあと、When `docs/releasing.md` を `0.2.0` で検索する、Then 公開済みの版の一覧のほかには出てこない。

### FR4 画面の小物（依頼 (4)）

#### FR4.1 英語の単数・複数（Q7 A）

- **FR4.1.1** ストリームの数に結びついた英語の文は、決め手の数が 1 のとき単数形、それ以外（0 を含む）は複数形にする。対象は下の表の 6 か所（`src/i18n/messages.ts`）。名詞（`stream` / `streams`）に加え、数に合わせて変わる動詞（`was` / `were`）もそろえる。

  | 場所（今の英語の文） | 決め手の数 | 単数のときの例 | 複数のときの例 |
  |---|---|---|---|
  | 209 行「Listing streams… {count} streams selected so far」 | `count` | `Listing streams… 1 stream selected so far` | `Listing streams… 3 streams selected so far` |
  | 213 行「Fetching… {finished}/{planned} streams, {count} events so far」 | `planned`（分母） | `Fetching… 0/1 stream, 5 events so far` | `Fetching… 1/3 streams, 5 events so far` |
  | 217 行「Failed in {count} streams」 | `count` | `Failed in 1 stream` | `Failed in 2 streams` |
  | 236 行「Fetch cancelled. {n} streams were not finished, so some logs are missing.」 | `n`（名詞と動詞） | `Fetch cancelled. 1 stream was not finished, so some logs are missing.` | `Fetch cancelled. 2 streams were not finished, so some logs are missing.` |
  | 607 行「Fetching logs (log group {number} / {groups}): {done} / {total} streams, {events} events」 | `total`（分母） | `… 0 / 1 stream, 5 events` | `… 1 / 3 streams, 5 events` |
  | 627 行「Results are based on part of the logs. {count} streams could not be fetched.」 | `count` | `… 1 stream could not be fetched.` | `… 2 streams could not be fetched.` |

- **FR4.1.2** 数に結びつかない `streams` は変えない。209 行の先頭の「Listing streams…」と、603 行「… listing streams, {count} selected so far」（`{count}` の後ろに名詞がない）は対象外とする。
- **FR4.1.3** イベントの数（`{count} events`、`{events} events`）など、ストリーム以外の数の文は今回の対象外とする（A8）。
- **FR4.1.4** 日本語の文は変えない。
- **FR4.1.5** 単数・複数の選び分けは、文ごとに条件を書かず、文の定義（単数形と複数形の 2 つ、決め手の数の名前）と 1 か所の選び分けの仕組みで行う。仕組みと 6 か所の文はテストで確かめる。

受け入れ基準

- Given 英語の画面、When 上の表の各文の決め手の数が 1、Then 表の「単数のときの例」のとおりに出る。
- Given 英語の画面、When 決め手の数が 0 か 2 以上、Then 表の「複数のときの例」のとおりに出る（0 は複数形：`Failed in 0 streams`）。
- Given 英語の画面、When 603 行の文や「Listing streams…」の先頭を出す、Then 今と同じ文が出る。
- Given 日本語の画面、When 同じ場面、Then 今と同じ文が出る。

#### FR4.2 中断の知らせの読み上げ

- **FR4.2.1** 中断の知らせ（CancelNotice）は、読み上げの囲み（`role="status"`）が中身より先に画面にあり、中身が後から入ることで、画面の読み上げが確実に一度起きる作りにする。
- **FR4.2.2** 見た目（警告の色、文）は変えない。

受け入れ基準

- Given 取得中、When 中断する、Then 読み上げの囲みは中断の前から画面にあり、中断の知らせの文がその中に入る（テストで囲みと中身の順を確かめる）。
- Given 中断の知らせが出ていない、Then 読み上げの囲みは空で、何も読み上げない。

#### FR4.3 権限のテストの固定値（Q8 A）

- **FR4.3.1** `src-tauri/src/lib.rs` の権限のテスト（`the_manifest_the_capability_and_the_handler_list_the_same_commands`）は、コマンドの数を固定しない。3 か所（`build.rs` のマニフェスト、ハンドラ、`capabilities/default.json`）の一覧が同じであることと、どれも空でないことだけを確かめる。
- **FR4.3.2** 数の経緯のコメント（26 → 30 → 32）を消す。

受け入れ基準

- Given 3 か所の一覧が同じ、When テストを流す、Then 通る。
- Given どこか 1 か所にだけコマンドを足す（または消す）、When テストを流す、Then 落ちる。

#### FR4.4 クエリの画面の進み具合の置き場所（Build and Test の目視で出た追加の依頼、Q10 A）

今は、クエリの実行中の進み具合が結果の枠（Results）の中に出ている。「Fetching logs (log group 1 / 1): 37 / 744 streams, 719 events」のような文で、前回の結果（「Results of the previous run」）の上にある。しかし進み具合は［Run query］を押したことで出るもので、前回の結果とは別のまとまりである（開発者の目視、2026-10-21）。

- **FR4.4.1** 実行中の進み具合（`RunProgress`）は、クエリの枠（Log Analytics の入力の枠）の中、［Run query］の並び（`RunBar`）のすぐ下に出す。対象は、キャッシュの読み込み・準備・ストリームの列挙・取得・クエリの評価の各段階の文と、取得に失敗したロググループの数。結果の枠の中には出さない。
- **FR4.4.2** 結果の枠は、結果の見出しと件数、前回の結果（「Results of the previous run」の表示を含む）、結果がないときの文だけを持つ。実行中で前回の結果がないときは、結果の枠の中身は空にする（A9）。
- **FR4.4.3** 進み具合の文、読み上げ（`role="status"`）、出る条件（取得中と評価中だけ）、`data-testid="log-analytics-progress"` は変えない。置き場所だけを変える。
- **FR4.4.4** 実行していないとき（終わった・中断した・失敗した・まだ実行していない）は、クエリの枠の中に進み具合を出さない。

受け入れ基準

- Given クエリの実行中（取得中または評価中）、When 画面を見る、Then 進み具合はクエリの枠の中の［Run query］の並びの下にあり、結果の枠の中にはない。
- Given 前回の結果があるまま次の実行が始まった、When 画面を見る、Then 結果の枠には前回の結果（「Results of the previous run」）だけがあり、進み具合はクエリの枠の中にある。
- Given 実行が終わった（Done）、When 画面を見る、Then クエリの枠の中に進み具合はない。
- Given 前回の結果がないまま実行中、When 画面を見る、Then 結果の枠の中身は空で、進み具合はクエリの枠の中にある。

### FR5 リージョンの選択欄の名前と絞り込み（依頼 (5)）

- **FR5.1** 組み込みのリージョン（今の 34 個、`catalog/regions.rs` の `STANDARD_REGIONS`）ごとに、英語と日本語の名前を持つ表を足す。名前は AWS の公式の名前（コンソールの表記）に合わせる（例：`ap-northeast-1` は「Asia Pacific (Tokyo)」「アジアパシフィック (東京)」、`us-east-1` は「US East (N. Virginia)」「米国東部 (バージニア北部)」）。
- **FR5.1.1 表の置き場所** 名前の表は画面の側（TypeScript、`src/` の下）に、画面の言語ごとの文と同じく置く。ライブラリとアプリ（Rust）は今のままコードだけを扱い、`SessionView.regions` もコードの並びのまま変えない（Tauri のコマンドとイベントの形を変えない）。
- **FR5.2** 上の帯のリージョンの選択欄を、今の Cloudscape の `ButtonDropdown`（文字を打てない）から、文字を打って絞り込める Cloudscape の選択欄（`Select` の絞り込みなど）に替える。プロファイルとタイムゾーンの選択欄は替えない。
- **FR5.3** 候補の一覧は、それぞれのリージョンをコードと画面の言語の名前を並べて出す（Q9 A。例：日本語の画面で `ap-northeast-1 アジアパシフィック (東京)`）。並び順は今と同じくコードの順。今の接続のリージョンが分かる印（今のチェックの印にあたるもの）を付ける。
- **FR5.4** 選択欄で文字を打つと、候補が絞り込まれる。コードと、英語・日本語のどちらの名前にも一致する（大文字・小文字を区別しない部分一致）。
- **FR5.5** 表にないリージョン（プロファイルの既定のリージョンが組み込みの一覧にないとき。今の `region_options` が足すもの）は、名前なしでコードだけを出し、コードで絞り込める。
- **FR5.6** 絞り込みで候補がなくなったときは、そのことを画面の言語で出す。
- **FR5.7 閉じているときの表示** 選択欄を閉じているときは、選んでいるリージョンを候補と同じ形（コードと画面の言語の名前。表にないものはコードだけ）で出す。リージョンをまだ選んでいないときは、今と同じ「リージョンを選択」／「Choose a region」（`connection.region.placeholder`）を出す。
- **FR5.8 無効になる条件** 選択欄は、今と同じ条件で無効にする：接続を変えられないとき（`canChangeConnection` が偽）と、SSO のサインインの最中（261011-sso-login BR3.4）。無効のときは開けず、文字も打てない。
- **FR5.9** 選んだリージョンを接続に使う振る舞い（表示中のログがあるときの確認ダイアログを含む）は変えない。選択欄はキーボードだけで操作でき、画面の読み上げでは「リージョン」／「Region」（`connection.region.label`）と読まれる。
- **FR5.10 表とリージョンの一覧の一致** 名前の表のリージョンの集まりは `STANDARD_REGIONS` と一致する。画面のテスト（Vitest）が `crates/local-sights-core/src/catalog/regions.rs` の `STANDARD_REGIONS` を読み、名前の表と比べて、足りない・余るがないことを確かめる。

受け入れ基準

- Given 日本語の画面、When リージョンの選択欄を開く、Then `ap-northeast-1 アジアパシフィック (東京)` のようにコードの順で並び、今の接続のリージョンに印が付く。
- Given 英語の画面、When 開く、Then `ap-northeast-1 Asia Pacific (Tokyo)` のように並ぶ。
- Given どちらの言語の画面でも、When 「東京」「tokyo」「northeast」のどれかを打つ、Then `ap-northeast-1` が候補に残る。
- Given プロファイルの既定のリージョンが表にない、When 選択欄を開く、Then そのリージョンはコードだけで並び、コードで絞り込める。
- Given 「zzz」を打つ、Then 候補がないことが画面の言語で出る。
- Given `ap-northeast-1` を選んだ日本語の画面、When 選択欄を閉じている、Then `ap-northeast-1 アジアパシフィック (東京)` と出る。
- Given リージョンをまだ選んでいない、When 画面を開く、Then 選択欄に「リージョンを選択」（英語の画面では「Choose a region」）と出る。
- Given SSO のサインインの最中、または接続を変えられないとき、When 選択欄を押す、Then 開かず、文字も打てない。
- Given 名前の表から 1 つ消すか、表にないコードを足す、When 画面のテストを流す、Then 一致のテストが落ちる。

## Non-Functional Requirements

- **NFR1 既存のテストはすべて緑のまま** [desc]。Rust のテスト（`cargo test`、ライブラリとアプリ）は CI の Linux・macOS・Windows の各ランナーで、画面のテスト（Vitest）は Linux のランナーで、すべて通る。FR2.2.5 と FR4.3 で直すテスト以外は、期待値を変えない。
- **NFR2 狙ったテスト（bugfix のスコープの決まり、org.md Testing Posture）。** FR1〜FR5 の各直しに、その直しを確かめるテストを少なくとも 1 つ足すか直す。純粋なロジック（FR2.1 の読み止め、FR2.2 の捨て方、FR4.1 の単数・複数、FR5.4 の一致）はテストを先に書く（team.md Testing Posture）。
- **NFR3 カバレッジ。** 行カバレッジ（ライブラリは `cargo-llvm-cov`、画面は Vitest）は CI の Linux のランナーで 80% 以上を保つ（team.md Testing Posture）。
- **NFR4 速さ。** 今の速さのテスト（既定で無視される 9 件、`cargo test --release -p local-sights-core --lib -- --ignored`）がすべて通り、261002 の NFR1・NFR2（10 万件 10 秒以内、100 万件 100 秒以内）を保つ。FR2.1 の直しで、重なる範囲が前のほうにあるとき読む行が減る。
- **NFR5 呼ぶ API を増やさない。** CloudWatch Logs の読み取りの 3 つ（DescribeLogGroups・DescribeLogStreams・GetLogEvents）以外の API を呼ばない（project.md Forbidden）。リージョンの名前はアプリに組み込んだ表で持ち、ネットワークから取らない。
- **NFR6 2 つの言語。** 新しく足す文（FR5.6 の候補なし、リージョンの名前）は英語と日本語の両方を持ち、既存の「両方の言語がそろっている」テストに入る。
- **NFR7 書式と検査。** `cargo fmt --check`・`cargo clippy`（ワークスペース、3 OS）・`prettier --check`・ESLint・`tsc --noEmit`・`cargo-deny`・`npm audit --audit-level=high` が CI で通る（team.md Code Style）。テスト以外のコードで `unwrap()`／`expect()` を使わない。

## Constraints

- 呼べる AWS の API は読み取りの 3 つだけ、自動テストと CI から AWS につながない、認証情報を書かない、テレメトリを送らない（project.md Forbidden）。
- GUI に依存しないライブラリとアプリを分ける。OS ごとの処理は `cfg` の境界に閉じ込める（team.md Code Style）。
- 短命のブランチで 1 つの PR にし、CI がすべて通ってからスクワッシュマージする（team.md Way of Working）。
- この作業だけでは版を上げず、タグも打たない。次の版の番号はタグの直前に決める（team.md Deployment、Practices Discovery Q1・Q2）。

## Assumptions

- **A1 Windows 11 の手元の確認は省く。** team.md Walking Skeleton は、両方の OS に効く変更では macOS と Windows の両方で手元の確認を行うと決めているが、この作業だけの例外として省く [desc]（Practices Discovery Q5 A）。Windows は CI の `Tauri app (windows-latest)` のビルドとテストで確かめる。Build and Test の記録にもこの例外を書く。
- **A2 macOS の手元の確認は、GUI の目視に絞る。** 実際の AWS への取得の流れは変えないため、確認用プログラム（team.md Walking Skeleton の 2）は流さなくてよい。GUI を起動し、リージョンの選択欄（FR5）、英語の単数・複数（FR4.1）、中断の知らせ（FR4.2）、クエリの画面の実行（FR2）を目で確かめる。
- **A3 FR1 のテストの道具。** 照合を待たせる仕掛けなど、順番を固定する道具はテストの側（`tests/support/` など）に置く。製品のコードに「テストのための入口」を足す必要が出たら、Code Generation の計画で開発者の承認を得る。
- **A4 FR2.1 のファイルの形は変えない。** キャッシュのファイルの形（見出しの行と時刻順の行）は変えず、既存のファイルはそのまま読める。ログの画面の照合（`lookup`）の読み方も変えない。
- **A5 FR5.1 の名前の確かめ。** 34 個の名前（英語と日本語）は、Code Generation で AWS の公式の資料（リージョンの一覧のページの英語版と日本語版）と照らして確かめる。資料と食い違う名前は資料に合わせる。
- **A6 FR5.2 の部品。** リージョンの選択欄は、絞り込みのできる Cloudscape の部品に替える（FR5.4 の絞り込みに必要）。見た目の細部（上の帯の中の並びと幅）は今に近づけ、標準の部品だけで作る（project.md Corrections：見た目は標準部品のみ）。名前の表を画面の側に置くのは、名前が画面の言語ごとの表示の文で、Rust 側に置くとリージョンごとに 2 つの言語の文を Tauri の境界を越えて運ぶことになり、コマンドの形も変わるため（代わりの案：Rust の `catalog/regions.rs` に名前を持たせ `SessionView` で送る。境界とテストが重くなるため採らない）。
- **A7 FR5.4 の一致。** 日本語の名前の一致は、大文字・小文字の区別に加え、全角・半角の括弧の違いを区別しない程度にとどめる。読み仮名やローマ字の読みでは一致しない。
- **A9 FR4.4 の空の結果の枠。** 前回の結果がないまま実行しているあいだ、結果の枠の中身は空にする。今の「まだ実行していません」の文は実行中には合わず、進み具合はクエリの枠の中にあるため。見た目の細部（余白）は標準の部品の既定に任せる（project.md Corrections：見た目は標準部品のみ）。
- **A8 FR4.1 は数の文だけ。** 単数・複数を分けるのは、ストリームの数を含む英語の文に限る。ほかの数（イベントの数、ロググループの数など）の英語の文で同じ問題が見つかったら、同じ仕組みで直してよい（範囲を広げる場合は Code Generation の計画に書く）。

## Out of Scope

- 版を上げる PR、Release dry run、タグを打つこと、配ること（次の機能と合わせて行う）。
- 新しい機能（IB-09 書き出し、IB-10 JSON の整形、検索中の途中結果の表示など）。
- Windows 11 での手元の確認（A1）。
- キャッシュのファイルの形の変更と、ログの画面の照合（`lookup`）の変更（A4）。
- U4 の確認用プログラムが `limit 0` を絞りなしとして扱う点の直し（比べに影響しないため、説明だけ書く。FR3.2）。
- 経過時間の表示の番号の比べ方の変更（説明だけ足す。FR3.3、Q5 A）。
- 利用者のフォルダ名に日本語や空白があるときの確認（261015 の NFR5.b、報告があれば別の作業で直す）。
- ほかの言語（英語と日本語以外）。

## Open Questions

- **OQ1** FR1 の順番の固定に、製品のコードへのテスト用の入口が要るかどうかは、Code Generation の計画で決める（A3）。要るときは開発者の承認を得る。
- **OQ2** FR2.3.5：QueryStore の内部の世代の数え方を残すか消すかは、Code Generation でコードを読んで決める。外に出る項目と引数を取り除くことは決まっている。
- **OQ3** FR5.1 の日本語の名前の表記（例：「米国東部 (バージニア北部)」の括弧が全角か半角か）は、AWS の資料の表記に合わせる（A5）。

## Sources

- Initial description: [desc] `aidlc/spaces/default/intents/261020-carryover-fixes/project-description.json`（依頼の 5 項目と前提）
- 要件の確定の質問：[Q1]〜[Q10] `inception/requirements-analysis/requirements-analysis-questions.md`（Q10 は Build and Test の目視のあとに足した FR4.4 の扱い）
- 進め方の確認：`inception/practices-discovery/team-practices.md`（Way of Working・Walking Skeleton・Testing Posture・Deployment・Code Style）と Practices Discovery の面談（Q1 C・Q2 B・Q5 A）
- コードの知識ベース：`aidlc/spaces/default/codekb/local-sights/business-overview.md`（利用者と主な機能）、`architecture.md`（取得・クエリ・キャンセルの流れ）、`code-structure.md`（モジュールの配置）、`code-quality-assessment.md`（5 項目の技術的負債と行）
- 走査の結果：`inception/reverse-engineering/developer-scan.md` の Handoff Summary（5 項目のファイルと行、26 ではなく 32、名前の表がないこと）
- チームとプロジェクトの決まり：`aidlc/spaces/default/memory/team.md`、`project.md`（Forbidden、Corrections）、`org.md`（bugfix のスコープの決まり）
