# Test Results — Logs Insights 互換のクエリ（IB-11）

流した日：2026-10-18。場所：開発用のコンテナ（Linux x86_64）。ブランチ `claude/zen-hamilton-gs7xnj` のコミット `d130f53`（U1〜U4 のコードをすべて含む）。

## 1. ビルド

| コマンド | 結果 |
|---|---|
| `npm run check:version` | 成功 |
| `cargo fmt --all --check` | 成功 |
| `cargo clippy --workspace --all-targets` | 成功（警告 0） |
| `cargo build -p local-sights` | 成功 |
| `npx tsc --noEmit` | 成功 |
| `npx eslint .` | 成功 |
| `npx prettier --check .` | 成功 |
| `npx vite build` | 成功 |
| `npm audit --audit-level=high` | 成功（脆弱性 0） |
| `cargo deny check` | 成功（advisories ok、bans ok、licenses ok、sources ok。重複した crate の警告だけで、どれも今の main と同じ。この作業で Rust の依存関係は増えていない） |

## 2. テスト

各作業単位の `unit-test-instructions.md` のコマンドは、すべて下の全体のコマンドに含まれる（同じテストを 2 回数えない）。

| コマンド | 合計 | 通った | 落ちた | 飛ばした |
|---|---|---|---|---|
| `cargo llvm-cov -p local-sights-core --fail-under-lines 80`（単体） | 631 | 622 | 0 | 9（速さのテスト。下の 3 で別に流した） |
| 同上（結合 `tests/u1_*`〜`u6_*`、9 ファイル） | 78 | 78 | 0 | 0 |
| `cargo test -p local-sights-core --doc` | 0 | 0 | 0 | 0 |
| `cargo test -p local-sights-core --examples`（compat_check 42、fetch_check 7） | 49 | 49 | 0 | 0 |
| `cargo test -p local-sights`（アプリ） | 31 | 31 | 0 | 0 |
| `npm run test:coverage`（Vitest、41 ファイル） | 469 | 469 | 0 | 0 |
| `bash -n scripts/compat/insights-export.sh`（U4） | — | 成功 | — | — |

落ちたテストはない。U2 で 1 回だけ見えた `tests/u5_cancel.rs` の落ち方は、今回は起きていない。

## 3. カバレッジ

| 対象 | 道具 | 行カバレッジ | 下限 |
|---|---|---|---|
| 中核ライブラリ | cargo-llvm-cov 0.9.1（Linux） | 96.52%（35,619 行のうち 1,238 行が未実行の領域） | 80% |
| 画面 | Vitest のカバレッジ | 89.88%（1,759 行のうち 1,581 行） | 80% |

カバレッジを外す印は、この作業で足していない。

## 4. 速さとメモリ（release ビルド、`--test-threads=1`）

`cargo test --release -p local-sights-core --lib -- --ignored --nocapture --test-threads=1`：9 件すべて通った。

| 測り | 結果 |
|---|---|
| ログの画面：10 万件の絞り込み | 2.3 ms |
| ログの画面：100 万件の絞り込み | 30.2 ms |
| ログの画面：100 万件にページを追加 | 97.0 ms |
| タイムライン：100 万件の行と位置 | 22 µs／11 µs |
| タイムライン：重なる 1 万件を 100 万件に足す | 99.5 ms |
| クエリ 10 万件：filter だけ／JSON の filter／JSON の sort／fields・filter・sort・limit | 17.0 ms／44.9 ms／87.2 ms／93.6 ms |
| クエリ 100 万件：同じ 4 種類 | 262 ms／644 ms／1.75 s／758 ms |
| 100 万件の JSON の sort の最も長い 1 区切り（[Cancel] の遅れの上限） | 927〜965 ms。3 区切りで止めると 185 ms で結果を出さずに止まった |
| 2 つの画面に 100 万件ずつ、50 個のロググループ | 範囲の記録 574 ms、評価の並び 154 ms、前の結果 10 万行 2.77 s、JSON の sort 2.44 s（最も長い 1 歩 1.14 s） |
| ピークのメモリ（Linux の VmHWM） | 始め 451.7 MiB（同じ実行の前のテストの分を含む）→ 2 つの画面に 100 万件ずつ 545.0 MiB → 前の結果と sort のあと 672.0 MiB。落ちずに結果が出た |

## 5. Target Verification Matrix

人が決めた NFR1 の数値は、2026-10-18 の答え（C：10 万件 10 秒以内、100 万件 100 秒以内）。

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| T-NFR1-100K | requirements.md NFR1、Build and Test の答え | 10 万件の評価が 10 秒以内（4 種類のクエリのいちばん遅いもの） | 93.6 ms | 上の 4（`four_queries_over_one_hundred_thousand_events`） | build-and-test | Met |
| T-NFR1-1M | requirements.md NFR1、Build and Test の答え | 100 万件の評価が 100 秒以内 | 1.75 s（1 つのロググループ）、2.44 s（50 個） | 上の 4（`four_queries_over_one_million_events`、`memory::…`） | build-and-test | Met |
| T-NFR2 | requirements.md NFR2 | 100 万件でも落ちずに結果が出る。評価中も画面が反応する（評価は区切りごとに止められる） | 落ちずに結果。最も長い 1 区切り 0.97 s、[Cancel] で止まる | 上の 4（`memory::…`、`a_cancel_during_a_json_sort…`） | build-and-test | Met |
| T-BASE-SPEED | team.md Testing Posture「既存の速さのテスト」、C5 | ログの画面の 10 万件 10 秒・100 万件 100 秒ほか、今の速さのテストが通る | 5 件とも通った（2.3 ms・30.2 ms ほか） | 上の 4 | build-and-test | Met |
| T-NFR8-CORE | requirements.md NFR8、各計画の Testing Contract | ライブラリの行カバレッジ 80% 以上 | 96.52% | 上の 3 | build-and-test | Met |
| T-NFR8-UI | requirements.md NFR8、各計画の Testing Contract | 画面の行カバレッジ 80% 以上 | 89.88% | 上の 3 | build-and-test | Met |
| T-NFR8-OS | requirements.md NFR8・NFR6、team.md Testing Posture | Rust のテストが Linux・macOS・Windows の各ランナーで通る | 3 つの OS の `Tauri app` ジョブがすべて成功（ライブラリとアプリのテスト、workspace の clippy、アプリのビルド） | PR kdnakt/local-sights#34 の CI（コミット `4114fb2`）、上の 8 | build-and-test | Met |
| T-C5 | requirements.md C5 | 既存のテストがすべて通る（ログの画面の振る舞いを変えない） | すべて通った（Linux） | 上の 2 | build-and-test | Met |
| T-STYLE | team.md Code Style | fmt・clippy（deny）・eslint・prettier・tsc が通る | すべて通った | 上の 1 | build-and-test | Met |
| T-DEPS | team.md Code Style | `npm audit --audit-level=high` と `cargo deny check` が通る | どちらも通った | 上の 1 | build-and-test | Met |
| T-NFR5 | requirements.md NFR5、project.md Forbidden | 呼ぶ AWS API は読み取りの 3 つだけ | 新しい API はない。偽のゲートウェイのテストと、スクリプトのテスト（`logs start-query`・`logs get-query-results` だけ）が通った | 上の 2、security-test-instructions.md | build-and-test | Met |
| T-NFR12 | requirements.md NFR12、project.md Forbidden | 外部に何も送らない | 足した行に外部への送信はない（一致したのは関数の名前 `finish_fetch`・`report_fetch` だけ） | security-test-instructions.md の差分の確かめ | build-and-test | Met |
| T-SM1 | requirements.md SM1・NFR3、U4 BR3.3 | 互換を確かめる 20 本のクエリで、コンソールとアプリの結果が同じ（終了コード 0） | 23 本すべて Same（3 回目、2026-10-19） | 下の 15（`insights-export.sh` → `compat_check`） | 開発者の手元。この計画に後の段はない | Met |
| T-WS-MAC | team.md Walking Skeleton の 2・3、U1〜U3 code-summary | macOS で、実際の AWS で取得してクエリの結果が出る。取り足し・キャッシュ・接続の切り替え・ログの画面の振る舞いが前と同じ | 開発者の Mac でひととおり確かめて問題なし（複数のロググループを含む） | 下の 9（開発者の報告、2026-10-18） | 開発者の手元 | Met |
| T-WS-WIN | team.md Walking Skeleton の 2・3 | Windows 11（CI で作ったインストーラで入れたアプリ）で同じことができる | まだ確かめていない | 同上 | 開発者の手元 | Unverified |
| T-ACE | U2 code-generation の review R-04、FR9.2 | 本物の Ace で、色分け・補完の候補・エラーの下線と行の横の印が出る | 開発者の Mac で、補完と色分けに問題なし。ひととおりの確認で問題なし | 下の 9（開発者の報告、2026-10-18） | 開発者の手元 | Met |

## 6. 失敗の扱い

- 落ちたコマンドと `Not Met` の目標はない。
- `Unverified` が残っている（T-SM1、T-WS-WIN。T-WS-MAC と T-ACE は下の 9 で Met）。当初は 4 つ（T-SM1、T-WS-MAC、T-WS-WIN、T-ACE）。どれもコードの誤りではなく、このコンテナでは流せない確かめ（本人の AWS 認証情報、本物の GUI）。T-NFR8-OS は PR の CI で Met になった（下の 8）。コードを直して変わるものではないため、Code Generation に戻る直しはない。
- この計画（refactor）には Build and Test の後の段がないため、これらを引き受ける段もない。開発者の判断を仰ぐ（下の 7）。

## 7. 開発者の判断

2026-10-18：開発者の答えは「PR作ってCI結果みよう」。PR を作り、macOS・Windows の CI の結果で T-NFR8-OS を確かめる。結果が出たらこの表を更新する。手元の確かめ（T-SM1、T-WS-MAC、T-WS-WIN、T-ACE）は、まだ残っている。

2026-10-18：CI の結果（下の 8）を見たあと、残り 4 つの扱いについて、開発者の答えは「Abort」（ここで止め、手元で確かめたあとに続きから再開する）。Build and Test の承認は開いていない。再開したら、手元の確かめの結果でこの表を更新する。U4 のコードのレビューの R-01〜R-05 も残っている。

## 8. PR の CI（kdnakt/local-sights#34、コミット `4114fb2`）

| ジョブ | 結果 |
|---|---|
| Rust library (fmt, clippy, test) | success |
| Front end (tsc, eslint, prettier, vitest, build) | success |
| cargo-deny | success |
| Tauri app (ubuntu-latest) | success |
| Tauri app (macos-latest) | success |
| Tauri app (windows-latest) | success |
| GitGuardian Security Checks | success |

## 9. 手元の確かめ（開発者の Mac）

- 2026-10-18：Node.js が古く `npm run tauri dev` が Vite（rolldown）の起動で落ちた（`node:util` に `styleText` がない）。Node.js 22.12 以上が要る（CI は 22）。この作業の変更が原因ではない。
- 2026-10-18：Node.js を上げたあと、macOS で簡単な `filter @message` のクエリが動いた。T-WS-MAC の一部。複数のロググループ・取り足し・キャッシュ・接続の切り替え・ログの画面・本物の Ace は、まだ確かめていない。
- 2026-10-18：開発者の気づき：コンソールの Logs Insights は検索中に取得できた結果を少しずつ表示するが、このアプリは取得と評価がすべて終わってから結果を出す。要件（FR7）と設計にない振る舞いの違いで、誤りではない。開発者の答えは「次の版に回す」。IB-11 には入れず、次の版の改善の候補とする。
- 2026-10-18：開発者の報告：Mac の確かめはひととおり問題なし。複数のロググループで実行し、補完と色分けも問題なかった。T-WS-MAC と T-ACE を Met にした。
- 2026-10-18：開発者の気づき：接続のリージョンを選ぶ欄に、リージョンの名前（東京、バージニア北部など）が出ず、検索もできない。この作業で変えた部分ではない（接続の選び方は前からの画面）。開発者の答えで、今後の改善の候補として積む。
- 2026-10-18：開発者の答え：Windows 11 の確かめ（T-WS-WIN）は最後にまとめて行う。

## Loop-Back Log

### Loop-back 1 — 2026-10-18T12:45:00Z

- **Diagnosis**：コンソールとの互換の確かめ（T-SM1）を信頼して流すには、U4 の `scripts/compat/insights-export.sh` の欠陥を先に直す必要がある。start-query の失敗を理由を区別せずにすべて `status: Error`（Logs Insights がクエリを断った）として書くため、認証切れ・アクセスの拒否・スロットリングでも BR2.4 の断り方の比べに使われ、BothRejected（互換）や OnlyInsightsRejected（違い）と誤って判定しうる（U4 のコードのレビュー R-01、Major）。
- **Root-cause stage**：code-generation（u4-compat-check）
- **Planned fix**：U4 のレビューの R-01〜R-05 を直す。R-01：`MalformedQueryException` のときだけ `Error` とし、それ以外の失敗は `Unknown`（NotCompared、終了コード 2 の側）として書く。偽の `aws` で認証切れ相当の失敗のテストを足す。R-02：実行の初めに OUT の中の前回の `*.json` を消す（または settings.json と食い違う書き出しを NotCompared にする）。R-03：start-query に `--limit 10000` を渡す。R-04：CLI の標準エラーをクエリの ID と分ける。R-05：U4 の code-summary の書式の記述を直す。
- **Estimated impact**：手間 小（スクリプト 1 つ、Rust のテスト 3 件ほど、記録の直し。計画の承認とレビューをもう一度行う）。費用 0。危うさ 低い（アプリ本体と中核ライブラリは変えない）。開発者が「U4 の指摘を直す」を選んだ（2026-10-18）。

## 10. Loop-back 1 のあとの流し直し（2026-10-18、コミット `2e92b19` 以降）

U4 の Code Generation で R-01〜R-05 を直し（コミット `b6c04de`）、U4 のコードのレビュー（2 回目の試み、READY）と作業単位の確かめ（`cargo test --workspace && npx vitest run`、成功）を経て Build and Test に戻った。変わったのは確認用プログラム（例）とスクリプトだけ。

| コマンド | 結果 |
|---|---|
| `cargo test -p local-sights-core --examples` | compat_check 48 件（42 → 48）、fetch_check 7 件、すべて通過 |
| `cargo test -p local-sights-core` | 単体 622・結合 78 ほか、すべて通過 |
| `cargo fmt --all --check` | 通過 |
| `cargo clippy --workspace --all-targets` | 警告・エラーなし |
| `bash -n scripts/compat/insights-export.sh` | 通過 |

ほかの目標（速さ、カバレッジ、画面のテスト、依存関係）は、変わっていないコードについてのもので、上の 1〜8 の結果のまま。残りは T-SM1（コンソールとの互換、開発者の手元）と T-WS-WIN（Windows 11、最後にまとめて）。

## 11. コンソールとの互換の確かめ（T-SM1、開発者の Mac、2026-10-18）

条件：CloudTrail のロググループ、期間 1 時間（START=1767435900、END=1767439500）、イベント 992 件、ストリーム 4 本。スクリプトは開発者が jq 1.6 に合わせて直した版（`5ff5140`）。書き出しは 20 本すべて Complete。

結果：Same 13、Different 6、OnlyAppRejected 1、NotCompared 0（終了コード 1、違いあり）。ログの中身は記録しない。

| クエリ | 結果 | 原因の見立て |
|---|---|---|
| q01-doc-latest（limit 20）、q02-doc-like（limit 20）、q09-sort-natural（limit 200）、q10-sort-missing（limit 50） | Different（Logs Insights だけが 992・68・992・992 行） | スクリプトの誤り。Loop-back 1 の R-03 の直しで start-query に必ず `--limit 10000` を渡すようにしたため、クエリの中の `limit` より API の上限が優先され、Logs Insights がすべての行を返した。アプリの結果は正しい（20・20・200・50 行） |
| p03-text-order（`eventName > "M"`） | Different（Logs Insights 0、アプリ 136） | アプリの仮の決まり（U2：数字でない文字列どうしは Unicode の順で比べる）が違った。コンソールでは、数字でない文字列の大小の比べは合わない |
| p04-bool（`readOnly = "true"`） | Different（件数は 830 で同じ。値の書き方が違う） | コンソールは JSON の真偽値を `1` と出し、アプリは `true` と出す |
| p05-bool-literal（`readOnly = true`） | OnlyAppRejected | コンソールは引用符のない `true` を項目の名前として読むとみられ、0 行を返した。アプリは項目どうしの比べとして対応外にした |
| q03〜q08、q11、p01、p02、p06〜p09 | Same | — |

### Loop-back 2 — 2026-10-19T00:00:00Z

- **Diagnosis**：コンソールとの互換の確かめ（上の 11）で違いが 7 本出た。(1) q01・q02・q09・q10 はスクリプトの誤り：start-query に必ず `--limit 10000` を渡すため、クエリの中の `limit` より優先され、Logs Insights がすべての行を返した。(2) p03：数字でない文字列どうしの大小の比べ（`eventName > "M"`）を、アプリは Unicode の順で比べたが、コンソールは 0 行。(3) p04：JSON の真偽値を、コンソールは `1` と出し、アプリは `true` と出す。(4) p05：引用符のない `true` を、コンソールは項目の名前として読み（項目どうしの比べ）0 行を返すが、アプリは対応外のエラーにする。
- **Root-cause stage**：code-generation（u4-compat-check：(1)、u2-query-language：(2)〜(4)）
- **Planned fix**：(1) スクリプトは、クエリに `limit` がないときだけ `--limit 10000` を渡す。(2) `<`・`<=`・`>`・`>=` で両辺が数として読めない文字列のときは合わない（コンソールに合わせる）。(3) JSON の真偽値は結果の一覧と比べの文字で `1`／`0` と出す（開発者の答え「コンソールに合わせる」。false が 0 になるかは次の互換の確かめで確かめる）。(4) 引用符のない言葉を比べの右辺に書いたときは項目の名前として読み、項目どうしの比べに対応する（開発者の答え「項目どうしの比べに対応する」）。
- **Estimated impact**：手間 中（U2 の評価と読み取りの変更とテスト、U4 のスクリプトの小さな変更、それぞれ計画の承認とレビューをもう一度）。費用 0。危うさ 中（クエリの評価を変えるため、既存のテストを全部通し、互換の確かめをもう一度流す）。開発者が (3)・(4) の扱いを選んだ（2026-10-19）。

## 12. Loop-back 2 のあとの流し直し（2026-10-19、コミット `881b802`）

U2 の Code Generation で (2)〜(4) を直し（`fe4910a`、レビュー READY、作業単位の確かめは成功）、U4 の Code Generation で (1) と比べの道具を直した（`881b802`、レビュー READY、作業単位の確かめは成功）。U4 では、クエリの組に p10（項目どうしの `=`）、p11（右辺の項目がないときの `!=`）、p12（数字でない文字列と数字だけの文字列の大小）を足し、23 本にした。

| コマンド | 結果 |
|---|---|
| `cargo test --workspace` | 743 件が通過、9 件は既定で無視（速さのテスト）。中核ライブラリの単体は 634 件（622 → 634） |
| `cargo test -p local-sights-core --examples` | compat_check 51 件（48 → 51）、fetch_check 7 件、すべて通過 |
| `cargo test --release -p local-sights-core --lib -- --ignored` | 9 件とも通過（U2 の開発で流した。10 万件で最も遅い JSON の sort が 147 ms、100 万件で 1.44 s。NFR1 の 10 秒・100 秒以内のまま） |
| `npx vitest run` | 41 ファイル・469 件が通過 |
| `cargo fmt --all --check`、`npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit` | 通過 |
| `cargo clippy --workspace --all-targets` | 警告・エラーなし |
| `bash -n scripts/compat/insights-export.sh` | 通過 |

レビューの Minor で、次の互換の確かめと手元で見るもの（直していない。project.md Corrections のとおり、レビューを記録したあとの指摘は持ち越す）：

- U2：数字でない文字列と数字だけの文字列の大小は仮の決まり（Unicode の順）のまま → p12 で確かめる。右辺の項目がないときは `!=` だけが合う仮の決まり → p11 で確かめる。`@timestamp > now` のような形は、今は項目 `now` として読む（前は対応外の関数のエラー）。
- U4：`parse` の正規表現の中の `limit` はコマンドとみなす（`--limit 10000` を渡さない側に倒れる。今の組に `parse` はない）。limit の判定は macOS の bash 3.2 と awk ではまだ流していない → 手元の互換の確かめが初めての実行になる。

残りは T-SM1（コンソールとの互換、23 本でもう一度、開発者の Mac）と T-WS-WIN（Windows 11、最後にまとめて）。

## 13. コンソールとの互換の確かめ・2 回目（T-SM1、開発者の Mac、2026-10-19、コミット `b088e2b`）

条件：11 と同じ（CloudTrail のロググループ、期間 1 時間 START=1767435900・END=1767439500）。イベント 992 件、ストリーム 4 本、失敗したストリーム 0。書き出しは 23 本すべて Complete（Error 0、そのほか 0）。書き出しのフォルダはリポジトリの外の新しい空のフォルダ。

結果：Same 20、Different 3、BothRejected 0、OnlyAppRejected 0、OnlyInsightsRejected 0、Skipped 0、NotCompared 0（違いあり）。ログの中身は記録しない。

| クエリ | 結果 | 原因の見立て |
|---|---|---|
| q01〜q11、p01〜p07、p09、p10 | Same | Loop-back 2 の直しが効いた。q01・q02・q09・q10 はクエリの `limit` どおりの行数（20・20・200・50）になり、limit の判定は macOS の bash 3.2 と awk で動いた。p03（0 行）、p04（830 行、値の書き方も一致）、p05（0 行）、p10（22 行）も一致 |
| p08-limit0（`limit 0`） | Different（Logs Insights 0、アプリ 992） | アプリの仮の決まり（`limit 0` を上限なしとして扱う）が違った。コンソールは `limit 0` で 0 行を返す |
| p11-field-ne-missing（`eventName != errorCode`） | Different（Logs Insights 73、アプリ 992） | アプリの仮の決まり（右辺の項目がないときは `!=` だけが合う）が違った。コンソールは右辺の項目がないイベントを合わないとし、`errorCode` があるイベントだけを返した |
| p12-text-vs-digits（`eventName > "5"`） | Different（Logs Insights 0、アプリ 992） | アプリの仮の決まり（数字でない文字列と数字だけの文字列は Unicode の順で比べる）が違った。p03 と同じく、コンソールでは片方が数として読めない大小の比べは合わない |

3 本とも U2 のコードの仮の決まりで、スクリプトと比べの道具の誤りではない。

### Loop-back 3 — 2026-10-19T03:00:00Z

- **Diagnosis**：コンソールとの互換の確かめ・2 回目（上の 13）で違いが 3 本残った。どれも U2 の仮の決まりがコンソールと違った。(1) p08：`limit 0` を、コンソールは 0 行にするが、アプリは上限なしにする。(2) p11：右辺の項目がないときの `!=` を、コンソールは合わないとし、アプリは合うとする。(3) p12：数字でない文字列と数字だけの文字列の大小を、コンソールは合わないとし、アプリは Unicode の順で比べる。
- **Root-cause stage**：code-generation（u2-query-language）
- **Planned fix**：(1) `limit 0` は 0 行を返す。(2) 項目どうしの比べで、どちらかの項目がないときは、`!=` を含めてどの比べも合わない。(3) `<`・`<=`・`>`・`>=` で、どちらか片方でも数として読めない文字列のときは合わない（p03 の決まりを、片方だけが数の場合に広げる）。開発者の答え「U2 を直してコンソールに合わせる」（2026-10-19）。
- **Estimated impact**：手間 小（U2 の評価の小さな変更とテスト、計画の承認とレビューをもう一度）。費用 0。危うさ 小（既存のテストを全部通し、互換の確かめをもう一度流す）。これが 3 回目の Loop-back で、上限に達する。

## 14. Loop-back 3 のあとの流し直し（2026-10-19、コミット `693b3af`）

U2 の Code Generation で (1)〜(3) を直した（`693b3af`、レビュー READY、作業単位の確かめは成功）。U4 は変えていない。

| コマンド | 結果 |
|---|---|
| `cargo test --workspace` | すべて通過。中核ライブラリの単体は 637 件（634 → 637）、9 件は既定で無視（速さのテスト） |
| `cargo test -p local-sights-core --examples` | compat_check 51 件、fetch_check 7 件、すべて通過 |
| `cargo test --release -p local-sights-core --lib -- --ignored` | 9 件とも通過（最も遅い 100 万件の JSON の sort で 1.93 s。NFR1 の 10 秒・100 秒以内のまま） |
| `npx vitest run` | 41 ファイル・469 件が通過 |
| `cargo fmt --all --check`、`npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit` | 通過 |
| `cargo clippy --workspace --all-targets` | 警告・エラーなし |

レビューの Minor（直していない。レビューを記録したあとの指摘は持ち越す）：`DEFAULT_RESULT_LIMIT` の説明（plan.rs）と code-summary.md の部品の表に `limit 0` を絞りなしとする古い記述が残る。`limit 0` のとき画面は「合うイベントなし」と出す（コンソールも 0 行）。U4 の確認用プログラムは `limit 0` を絞りなし（上限 10,000）と扱うが、アプリが 0 行・打ち切りなしになるため両側 0 行で同じになり、比べには影響しない。

残りは T-SM1（23 本でもう一度、開発者の Mac。p08・p11・p12 が Same になるか）と T-WS-WIN（Windows 11、最後にまとめて）。Loop-back は上限の 3 回に達したため、これ以降に違いが出たら直さず、既知の違いとして README と記録に残すか、開発者に判断を仰ぐ。

## 15. コンソールとの互換の確かめ・3 回目（T-SM1、開発者の Mac、2026-10-19、コミット `8af7d04`）

条件：11・13 と同じ（CloudTrail のロググループ、期間 1 時間 START=1767435900・END=1767439500）。イベント 992 件、ストリーム 4 本、失敗したストリーム 0。書き出しは 23 本すべて Complete（Error 0、そのほか 0）。書き出しのフォルダはリポジトリの外の新しい空のフォルダ。開発者が手元で流した。

結果：Same 23、Different 0、BothRejected 0、OnlyAppRejected 0、OnlyInsightsRejected 0、Skipped 0、NotCompared 0。ログの中身は記録しない。

- Loop-back 3 の直しが効き、p08（0 行）、p11（73 行）、p12（0 行）がコンソールと一致した。
- ほかの 20 本も 13 と同じ行数で一致したまま。
- T-SM1 を Met にする（上の表の行も更新）。

残りは T-WS-WIN（Windows 11、最後にまとめて）。
