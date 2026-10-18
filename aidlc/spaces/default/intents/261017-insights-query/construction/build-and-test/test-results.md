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
| T-SM1 | requirements.md SM1・NFR3、U4 BR3.3 | 互換を確かめる 20 本のクエリで、コンソールとアプリの結果が同じ（終了コード 0） | まだ流していない（本人の AWS 認証情報が要る） | U4 code-summary の手順（`insights-export.sh` → `compat_check`） | 開発者の手元。この計画に後の段はない | Unverified |
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
