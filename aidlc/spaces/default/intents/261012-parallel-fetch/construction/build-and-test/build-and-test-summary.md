# Build and Test Summary — 261012-parallel-fetch

IB-07「ロググループ全体の取得の高速化」の作業単位 U1〜U5（同時取得の薄い一本、呼び出しの速さの調整、あきらめの決まり、経過時間、中断）をまとめてビルドし、テストした結果。

## ビルドの状態と前提

- ビルド：**成功**（`cargo build --workspace`、`cargo build -p local-sights`、`npx tsc --noEmit`、`npx vite build`、`npm run check:version`）。手順は `build-instructions.md`。
- 前提：Rust stable（1.97.0）、Node.js 22、cargo-llvm-cov 0.9.1、cargo-deny。自動テストは実際の AWS に接続しない。
- 枝：`claude/parallel-fetch`（main との分かれ目 `a79f2ad` = PR #20 の下準備）。

## テストの種類

Test Strategy は `Standard`。作業単位ごとの単体テストは各 `code-generation/unit-test-instructions.md`、このステージで足した指示書は次のとおり。

| 種類 | 指示書 | 中身 |
|------|--------|------|
| ビルド | `build-instructions.md` | 依存・ビルド・確かめ方・つまずき |
| 結合 | `integration-test-instructions.md` | 作業単位をまたぐ境目（同時取得 × 速さの上限 × 再試行 × 中断 × 閉じる × キャッシュ × 画面） |
| 速度 | `performance-test-instructions.md` | 偽物と止めた時計の測定、100 万件の一覧の速度測定、実際の AWS での手元の確認 |
| 安全 | `security-test-instructions.md` | cargo-deny・npm audit・要約の中身・権限・main との差分の確認 |

performance と security は Standard の既定では作らないが、各作業単位の NFR に速度と安全の目標があるため作った。

## 作業単位ごとのカバレッジの見込み

カバレッジは作業単位ごとではなく、ライブラリ全体と画面全体で測る（CI と同じ）。

| 作業単位 | 主な場所 | 下限 | 今回 |
|----------|----------|------|------|
| U1〜U3、U5 のライブラリ側 | `crates/local-sights-core` | 行 80% | ライブラリ全体 97.67% |
| U4・U5 の画面側 | `src/` | 行 80% | 画面全体 90.85% |
| U5 のアプリ側 | `src-tauri` | 測らない（今と同じ） | `cargo test -p local-sights` 成功 |

## Target Verification Matrix

目標は各作業単位の `nfr-requirements/*.md`（NFR の表と tech-stack-decisions）と、各 `code-generation-plan.md` の `## Testing Contract` から集めた。nfr-design の各ファイルは同じ ID を参照しており、新しい目標はない（U4 の NFR6.12 は tech-stack-decisions にある）。ID は作業単位の頭文字をつけた（同じ NFR 番号が作業単位ごとに別の中身を持つため）。

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|-----------|--------|----------|--------|----------|--------------|---------|
| U1-NFR2.2 | construction/u1-concurrent-skeleton/nfr-requirements/observability-requirements.md | 取得 1 回の終わりの要約：取得 1 回の終わりに、標準エラー出力に 1 行を出す。項目は「読んだストリームの本数、GetLogEvents の呼び出しの数（再試行を含む）、制限を受けた回数、かかった時間（ミリ秒）、… | 対応するテストが成功 | `crates/local-sights-core/src/fetch_stats.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR2.3 | construction/u1-concurrent-skeleton/nfr-requirements/observability-requirements.md | 確認用プログラムの表示：FetchCheck も同じ要約（同じ組み立ての関数）を表示する。手元の確認（NFR2.1）でかかった時間と件数を比べるのに使う | 対応するテストが成功 | `crates/local-sights-core/examples/fetch_check.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR8.6 | construction/u1-concurrent-skeleton/nfr-requirements/observability-requirements.md | 出してよいもの：数値と終わりの状態だけ。名前・接続の情報・失敗の詳細は出さない（security-requirements NFR8.3） | 対応するテストが成功 | `crates/local-sights-core/src/fetch_stats.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR2.4 | construction/u1-concurrent-skeleton/nfr-requirements/observability-requirements.md | 数え方：呼び出しの数は試行ごと（再試行を含む）に 1、制限を受けた回数は Throttled の失敗ごとに 1、同時の最大は読み途中のストリームの数の最大。数えるのは取得 1 回の中だけで、取得をまたいで残さない | 対応するテストが成功 | `crates/local-sights-core/src/fetch_stats.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR1.1 | construction/u1-concurrent-skeleton/nfr-requirements/performance-requirements.md | 全件がそろうまでの時間：偽物のゲートウェイ（1 回の呼び出し 200 ミリ秒、ストリーム 100 本・各 1 回、東京 12 回／秒）で、全件がそろうまで 10 秒以下（1 本ずつの今の方式 20 秒の半分以下）。時… | 対応するテストが成功 | `crates/local-sights-core/tests/u1_concurrent_fetch.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR1.2 | construction/u1-concurrent-skeleton/nfr-requirements/performance-requirements.md | 最初の 1 件が出るまでの時間：偽物でストリーム 20 本、範囲にログがあるのが順で 5 本目から、1 回 200 ミリ秒のとき、最初の on_batch が 1 本ずつの場合（約 1 秒）より早い | 対応するテストが成功 | `crates/local-sights-core/tests/u1_concurrent_fetch.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR1.3 | construction/u1-concurrent-skeleton/nfr-requirements/performance-requirements.md | 同時に読む本数の上限の確認：偽物で 1 回の呼び出しを 8 ÷ 上限 秒より長く（東京なら 700 ミリ秒以上）したとき、同時に出ている呼び出しの最大がちょうど 8（機能設計の 2 回目のレビューの Major を、… | 対応するテストが成功 | `crates/local-sights-core/tests/u1_concurrent_fetch.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR5.1 | construction/u1-concurrent-skeleton/nfr-requirements/performance-requirements.md | 100 万件の一覧の滑らかさ：既存の速度測定テスト（`cargo test -p local-sights-core --release --lib -- timeline:: --ignored` など）が今と同じ… | 速度測定 5 件が今の基準で成功 | `cargo test -p local-sights-core --release --lib -- --ignored`（5 件成功） | build-and-test | Met |
| U1-NFR5.2 | construction/u1-concurrent-skeleton/nfr-requirements/performance-requirements.md | 渡し待ちのページ：渡し待ちは最大 8 ページ（読み途中のストリームごとに 1 ページ）。それ以上はメモリにためない | 対応するテストが成功 | `crates/local-sights-core/src/coordinator.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR2.1 | construction/u1-concurrent-skeleton/nfr-requirements/performance-requirements.md | 手元の実際の AWS での速さの確認：確認用プログラムで、テスト用ロググループ（東京）を main の版と U1 の版で同じ時間範囲で取得し、件数が同じで U1 のほうがかかった時間が短ければ合格。数字の目標は置かない | main とこの枝で件数が同じで、この枝のほうが速かった（開発者の手元、2026-10-15） | 手元の確認（実際の AWS、`fetch_check` で main と比べる。`performance-test-instructions.md` の 3）。数値は記録しない（目標に数値なし） | build-and-test | Met |
| U1-NFR1.5 | construction/u1-concurrent-skeleton/nfr-requirements/reliability-requirements.md | 一部のストリームが失敗しても、ほかのストリームは読み続け、取れたログは残る | 対応するテストが成功 | `crates/local-sights-core/src/coordinator.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR1.6 | construction/u1-concurrent-skeleton/nfr-requirements/reliability-requirements.md | 取得は必ず終わる：選ばれたストリームが 0 本でもすぐ終わり、読みかけが無くなれば終わる。永久に待たない | 対応するテストが成功 | `crates/local-sights-core/src/coordinator.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR4.1 | construction/u1-concurrent-skeleton/nfr-requirements/reliability-requirements.md | ウィンドウを閉じるとき（Close）は、新しい呼び出しを始めず、許可待ち・再試行の待ちの間ならすぐ止まる。出ている呼び出しの応答は待つ | 対応するテストが成功 | `crates/local-sights-core/src/retry.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR1.7 | construction/u1-concurrent-skeleton/nfr-requirements/reliability-requirements.md | 再試行の待ちと回数は今と同じ（1・2・4・8・16 秒 ±20%、要求ごとに最大 5 回）。GetLogEvents の 5xx は再試行せず失敗として一覧する（SDK の再試行を止めた結果、受け入れた） | 対応するテストが成功 | `crates/local-sights-core/src/retry.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR1.8 | construction/u1-concurrent-skeleton/nfr-requirements/reliability-requirements.md | あきらめの暫定の動き：使い切りが 3 本続いたら（終わった順）、未着手は呼ばずに失敗とし、読み途中は最後まで読む | 対応するテストが成功 | `crates/local-sights-core/src/schedule.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR7.1 | construction/u1-concurrent-skeleton/nfr-requirements/reliability-requirements.md | 既存のテストはすべて通ったまま（JobStatus の置き場所の移動、止める信号の置き換え、fetch_stream の分割の後も） | 対応するテストが成功 | `crates/local-sights-core/tests/u3_fetch_flow.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR1.9 | construction/u1-concurrent-skeleton/nfr-requirements/reliability-requirements.md | 読み取りの中で panic が起きたら、取得 1 回のタスクごと終わり、アプリの「異常終了した取得の回復」が拾う（今と同じ） | 対応するテストが成功 | `src-tauri/src/lib.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR3.2 | construction/u1-concurrent-skeleton/nfr-requirements/scalability-requirements.md | 呼び出しの速さの上限：GetLogEvents は再試行を含めて、どの 1 秒の区間 [t, t+1 秒) でも、東京 12 回・上限 10 回のリージョン 5 回・eu-west-3 15 回・表にない／指定なし … | 対応するテストが成功 | `crates/local-sights-core/src/pacing.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR3.3 | construction/u1-concurrent-skeleton/nfr-requirements/scalability-requirements.md | 8 本が同時に許可を求めたとき：許可の時刻がすべて最小の間隔（東京なら 83,333,334 ナノ秒）以上あく | 対応するテストが成功 | `crates/local-sights-core/src/pacing.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR3.4 | construction/u1-concurrent-skeleton/nfr-requirements/scalability-requirements.md | SDK の中の再試行が数に入らないこと：GetLogEvents の呼び出しでは SDK の再試行の回数が 1 に上書きされている（設定の単体テスト）。DescribeLogGroups・DescribeLogStr… | 対応するテストが成功 | `crates/local-sights-core/src/gateway/aws.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR1.4 | construction/u1-concurrent-skeleton/nfr-requirements/scalability-requirements.md | ストリームの本数による伸び方：数十〜数百本（困りごとの場面）で、かかる時間は「本数 × 1 本あたりの呼び出し数 ÷ 速さの上限」程度になる（呼び出しの待ち時間ではなく速さの上限で決まる）。数千本でも動くが、速さの目… | 対応するテストが成功 | `crates/local-sights-core/src/pacing.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR5.3 | construction/u1-concurrent-skeleton/nfr-requirements/scalability-requirements.md | 同時に読む本数：最大 8 本で固定。利用者は変えられない | 対応するテストが成功 | `crates/local-sights-core/src/coordinator.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR8.1 | construction/u1-concurrent-skeleton/nfr-requirements/security-requirements.md | 呼ぶ API は CloudWatch Logs の読み取り 3 つ（DescribeLogGroups・DescribeLogStreams・GetLogEvents）だけ。U1 で新しい API を呼ばない | trait は 3 操作のまま。SDK の呼び出しは get_log_events・describe_log_groups・describe_log_streams だけ | main との差分の確認（`gateway/mod.rs` は変更なし）、`gateway/aws.rs` の呼び出し | build-and-test | Met |
| U1-NFR8.2 | construction/u1-concurrent-skeleton/nfr-requirements/security-requirements.md | 秘密の認証情報（シークレットアクセスキー・セッショントークン・SSO のトークン）とアクセスキー ID を、ログ・画面・キャッシュ・リポジトリに書かない | 対応するテストが成功 | `crates/local-sights-core/src/gateway/aws.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR8.3 | construction/u1-concurrent-skeleton/nfr-requirements/security-requirements.md | 取得 1 回の終わりの診断ログ（標準エラー出力の 1 行）に出すのは、読んだストリームの本数、GetLogEvents の呼び出しの数（再試行を含む）、制限を受けた回数、かかった時間、同時に読んだ本数の最大、終わりの… | 対応するテストが成功 | `crates/local-sights-core/src/fetch_stats.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR8.4 | construction/u1-concurrent-skeleton/nfr-requirements/security-requirements.md | 診断ログはファイルに書かず、外にも送らない（テレメトリ・クラッシュレポートを送らない） | 新しい出力は標準エラー出力の要約 1 行だけ | main との差分の確認（`eprintln!` のみ、ファイル・ネットワークへの書き込みなし） | build-and-test | Met |
| U1-NFR6.1 | construction/u1-concurrent-skeleton/nfr-requirements/security-requirements.md | 自動テストと CI は実際の AWS に接続せず、CI に AWS の認証情報を置かない | 対応するテストが成功 | `crates/local-sights-core/tests/support/fake_gateway.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR3.1 | construction/u1-concurrent-skeleton/nfr-requirements/security-requirements.md | 同じアカウントの他の利用者の枠を奪わない：GetLogEvents は再試行を含めて、どの 1 秒の区間でもリージョンの上限の半分以下 | 対応するテストが成功 | `crates/local-sights-core/src/pacing.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U1-NFR8.5 | construction/u1-concurrent-skeleton/nfr-requirements/security-requirements.md | 新しい依存（futures-util）は、cargo-deny の検査（脆弱性・ライセンス・出どころ）を通るものだけ。既定の機能は切り、要るものだけ有効にする | cargo-deny の検査がすべて ok | `cargo deny check`（advisories・bans・licenses・sources ok） | build-and-test | Met |
| U2-NFR3.5 | construction/u2-call-pacing/nfr-requirements/security-requirements.md | 制限を受けたら呼び出しを減らし、同じアカウントの他の利用者の枠を空ける：制限の知らせで今の速さを半分にし（下限 1 秒 1 回）、下げている間の許可の間隔は今の速さの間隔以上 | 対応するテストが成功 | `crates/local-sights-core/src/pacing.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U2-NFR3.6 | construction/u2-call-pacing/nfr-requirements/security-requirements.md | 戻しても上限を超えない：今の速さはリージョンの上限（U1 の BR1.1・BR1.2）を超えず、どの 1 秒の区間でも GetLogEvents の許可は上限以下のまま（U1 の NFR3.1 を保つ） | 対応するテストが成功 | `crates/local-sights-core/tests/u2_call_pacing.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U2-NFR8.1 | construction/u2-call-pacing/nfr-requirements/security-requirements.md | 呼ぶ API は CloudWatch Logs の読み取り 3 つだけ。U2 で新しい API を呼ばない | trait は 3 操作のまま。SDK の呼び出しは get_log_events・describe_log_groups・describe_log_streams だけ | main との差分の確認（`gateway/mod.rs` は変更なし）、`gateway/aws.rs` の呼び出し | build-and-test | Met |
| U2-NFR8.7 | construction/u2-call-pacing/nfr-requirements/security-requirements.md | 要約の 1 行に足す `min_rate=` は整数だけ。秘密の認証情報・アクセスキー ID・接続の情報（プロファイル名・リージョンなど）を含まない | 対応するテストが成功 | `crates/local-sights-core/src/fetch_stats.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U2-NFR8.8 | construction/u2-call-pacing/nfr-requirements/security-requirements.md | 引き下げは画面にも外にも知らせない。出力先は今と同じ標準エラー出力の要約の 1 行だけ（テレメトリを送らない） | 対応するテストが成功 | `crates/local-sights-core/src/coordinator.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U2-NFR6.1 | construction/u2-call-pacing/nfr-requirements/security-requirements.md | 自動テストと CI は実際の AWS に接続せず、CI に AWS の認証情報を置かない | 対応するテストが成功 | `crates/local-sights-core/tests/u2_call_pacing.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U2-NFR7.1 | construction/u2-call-pacing/nfr-requirements/security-requirements.md | 既存のテストは通ったまま。ただし U1 の要約の書式のテスト（欄の順）は、`min_rate=` が増えるので U2 の書式に直す | 対応するテストが成功 | `crates/local-sights-core/tests/u1_concurrent_fetch.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U2-NFR7.2 | construction/u2-call-pacing/nfr-requirements/security-requirements.md | ライブラリの行カバレッジ 80% 以上を CI で確かめる。CallPacer の下げ方・戻し方は純粋な部品としてテストを先に書く | ライブラリの行カバレッジ 97.67% | `cargo llvm-cov -p local-sights-core --fail-under-lines 80` | build-and-test | Met |
| U3-NFR3.7 | construction/u3-give-up-rule/nfr-requirements/security-requirements.md | あきらめた後は、未着手のストリームで GetLogEvents を呼ばない。制限が続くときに同じアカウントの他の利用者の枠を使い続けない | 対応するテストが成功 | `crates/local-sights-core/tests/u3_give_up_rule.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U3-NFR8.1 | construction/u3-give-up-rule/nfr-requirements/security-requirements.md | 呼ぶ API は CloudWatch Logs の読み取り 3 つだけ。U3 で新しい API を呼ばない | trait は 3 操作のまま。SDK の呼び出しは get_log_events・describe_log_groups・describe_log_streams だけ | main との差分の確認（`gateway/mod.rs` は変更なし）、`gateway/aws.rs` の呼び出し | build-and-test | Met |
| U3-NFR8.9 | construction/u3-give-up-rule/nfr-requirements/security-requirements.md | Skipped に写す失敗の内容は、3 本目の使い切りの失敗の安全な詳細（ApiFailure の許可リストを通したもの）だけ。秘密の認証情報・アクセスキー ID を含まない | 対応するテストが成功 | `crates/local-sights-core/src/fetcher.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U3-NFR6.1 | construction/u3-give-up-rule/nfr-requirements/security-requirements.md | 自動テストと CI は実際の AWS に接続せず、CI に AWS の認証情報を置かない | 対応するテストが成功 | `crates/local-sights-core/tests/u3_give_up_rule.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U3-NFR7.1 | construction/u3-give-up-rule/nfr-requirements/security-requirements.md | 既存のテストは通ったまま。ただし RetryPolicy の数えの項目（max_consecutive_exhausted）・ExhaustionStreak・StreamEnding を使うテストは、StreamS… | 対応するテストが成功 | `crates/local-sights-core/src/coordinator.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U3-NFR7.2 | construction/u3-give-up-rule/nfr-requirements/security-requirements.md | ライブラリの行カバレッジ 80% 以上を CI で確かめる。StreamSchedule の数えの決まりは純粋な部品としてテストを先に書き、終わる順を入れ替えた場合（AC2.3.1〜AC2.3.4）と、あきらめた後に… | ライブラリの行カバレッジ 97.67% | `cargo llvm-cov -p local-sights-core --fail-under-lines 80` | build-and-test | Met |
| U4-NFR2.3 | construction/u4-elapsed-time/nfr-requirements/observability-requirements.md | 利用者は状態行の経過で、取得にかかった時間を見られる（取得中は進み、終わると残る）。手元の確認（NFR2）はこの表示で行う | 取得中は経過が進み、終わると残った（開発者の手元の GUI、2026-10-15） | 手元の GUI の確認（`npm run tauri dev`） | build-and-test | Met |
| U4-NFR2.4 | construction/u4-elapsed-time/nfr-requirements/observability-requirements.md | 取得 1 回の要約の診断ログ（U1 の 1 行、`elapsed_ms=`）は変えない。要約の elapsed_ms は取得の流れの中で測った値で、状態行の経過（［取得］を受け入れた時点から）とは数十ミリ秒ほど違いうる | 対応するテストが成功 | `cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U4-NFR9.1 | construction/u4-elapsed-time/nfr-requirements/observability-requirements.md | 経過の文言は英語と日本語を用意し、OS の言語で出し分ける（「Elapsed {time}」／「経過 {time}」） | 対応するテストが成功 | `src/i18n/messages.ts`、`npm run test:coverage`（385 件成功・0 件失敗） | build-and-test | Met |
| U4-NFR9.2 | construction/u4-elapsed-time/nfr-requirements/observability-requirements.md | 読み上げ：取得中は経過の部分に `aria-hidden="true"` を付け、秒の進みを読み上げない。終わったら外す。最終の経過が読み上げられるのは、属性を外すのが終わりの状態行の変化（取得中の表示から件数などの… | 画面のテストで属性の付け外しを確認。VoiceOver の確認は利用者の判断で外した（Q2=A） | `src/components/ElapsedTime.test.tsx`、`npm run test:coverage`。`build-and-test-questions.md` Q2 | build-and-test | Met |
| U4-NFR5.3 | construction/u4-elapsed-time/nfr-requirements/performance-requirements.md | 1 秒ごとの描き直しは経過だけを描く小さな部品に閉じ込め、ログの表と状態行の残りは毎秒描き直さない | 画面のテスト成功。速度測定 5 件も成功 | `src/components/ElapsedTime.test.tsx`、`npm run test:coverage`（385 件成功・0 件失敗）、`--release --ignored`（5 件成功） | build-and-test | Met |
| U4-NFR5.4 | construction/u4-elapsed-time/nfr-requirements/performance-requirements.md | 経過のために画面へ送る回数を増やさない：`fetch-progress`（FetchProgressUpdate）の項目は今と同じ。`session-changed` を送る契機も今と同じ（経過のためだけに送らない）… | 対応するテストが成功 | `crates/local-sights-core/src/session.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U4-NFR2.2 | construction/u4-elapsed-time/nfr-requirements/performance-requirements.md | 手元の確認（NFR2）に使える経過：最終の値は、ライブラリの戻らない時計で［取得］を受け入れた時点から取得の終わり（キャッシュへの保存を含む）までを表す | 対応するテストが成功 | `crates/local-sights-core/src/session.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U4-NFR6.5 | construction/u4-elapsed-time/nfr-requirements/reliability-requirements.md | 取得中の表示は 1 秒ごとに進み、再試行の待ちなどライブラリから何も届かない間も止まらない | 対応するテストが成功 | `src/components/ElapsedTime.test.tsx`、`npm run test:coverage`（385 件成功・0 件失敗） | build-and-test | Met |
| U4-NFR6.6 | construction/u4-elapsed-time/nfr-requirements/reliability-requirements.md | 取得中の表示は減らない。この保持は同じ取得の中の合わせ直し（running の表示中に running を受け取ったとき）だけに当てはめ、合わせ直しの値が今の表示より小さいときは今の表示を保つ。次の場合は保持を解いて… | 対応するテストが成功 | `src/elapsed.ts`、`npm run test:coverage`（385 件成功・0 件失敗） | build-and-test | Met |
| U4-NFR6.7 | construction/u4-elapsed-time/nfr-requirements/reliability-requirements.md | 取得中の表示とライブラリの値とのずれは 1 秒以内。ただし PC のスリープをまたいだ取得は対象外とする（ライブラリと画面の戻らない時計がスリープ中に同じように進むとは限らないため）。その場合も終わったときはライブラ… | 画面のテストは成功。スリープをまたいだ取得でも、終わったときに最終の値が出た（2026-10-15） | `src/components/ElapsedTime.test.tsx`、`npm run test:coverage`。手元の GUI の確認 | build-and-test | Met |
| U4-NFR6.8 | construction/u4-elapsed-time/nfr-requirements/reliability-requirements.md | 時計は戻らない時計を使い、PC の時刻を変えても経過は負にも大きく飛びもしない。差が負になる計算は 0 にする | 対応するテストが成功 | `crates/local-sights-core/src/session.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U4-NFR6.9 | construction/u4-elapsed-time/nfr-requirements/reliability-requirements.md | 画面の時計の合わせ直しは、バックエンドから完全な SessionView が届いたとき（`session-changed`、起動時の `getSession`、操作の応答として返る SessionView）に行う。`… | 対応するテストが成功 | `src/App.test.tsx`、`npm run test:coverage`（385 件成功・0 件失敗） | build-and-test | Met |
| U4-NFR6.10 | construction/u4-elapsed-time/nfr-requirements/reliability-requirements.md | 経過が空になるのは「一度も取得していない」「閉じて止めた（Aborted）」「接続の変更で前の結果が消えた」の 3 つだけ。最後のものは機能設計の Q2 で利用者が選んだ扱いで、FR4.3・AC2.1.3 の「次の取… | 対応するテストが成功 | `crates/local-sights-core/src/session.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U4-NFR6.11 | construction/u4-elapsed-time/nfr-requirements/reliability-requirements.md | 取得の作業が異常に止まったときも、失敗として終える時点で最終の値を決め、経過が進み続けたままにならない | 対応するテストが成功 | `crates/local-sights-core/src/session.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U4-NFR6.3 | construction/u4-elapsed-time/nfr-requirements/scalability-requirements.md | 長い取得でも表記が崩れない：1 時間以上は h:mm:ss、時の桁は必要なだけ伸ばす（10 時間以上も表せる） | 対応するテストが成功 | `src/elapsed.test.ts`、`npm run test:coverage`（385 件成功・0 件失敗） | build-and-test | Met |
| U4-NFR6.4 | construction/u4-elapsed-time/nfr-requirements/scalability-requirements.md | 取得を何度繰り返しても、経過の記録は 1 回分だけ（取得ごとに置き換え、増えていかない） | 対応するテストが成功 | `crates/local-sights-core/src/session.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U4-NFR8.10 | construction/u4-elapsed-time/nfr-requirements/security-requirements.md | SessionView の `elapsed` に入るのは真偽値（running）と数値（elapsedMs）だけとし、秘密の認証情報・アクセスキー ID・プロファイル名・ロググループ名などを入れない | 対応するテストが成功 | `crates/local-sights-core/src/session.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U4-NFR8.11 | construction/u4-elapsed-time/nfr-requirements/security-requirements.md | 新しい API の呼び出し、新しい依存、新しいログ・ファイルへの書き込み、外への送信を足さない | 新しい API・依存・書き込み・送信なし。cargo-deny と npm audit も問題なし | main との差分の確認、`cargo deny check`、`npm audit --audit-level=high`（0 件） | build-and-test | Met |
| U4-NFR6.2 | construction/u4-elapsed-time/nfr-requirements/security-requirements.md | 自動テストと CI は実際の AWS に接続しない。経過のテストは、ライブラリも画面も時計を差し替えて行う | 対応するテストが成功 | `src/hooks/useElapsed.ts`、`npm run test:coverage`（385 件成功・0 件失敗） | build-and-test | Met |
| U4-NFR7.3 | construction/u4-elapsed-time/nfr-requirements/tech-stack-decisions.md | 行カバレッジはライブラリ・画面ともに 80% 以上を保つ（CI の `cargo llvm-cov -p local-sights-core --fail-under-lines 80` と `npm run tes… | ライブラリ 97.67%、画面 90.85% | `cargo llvm-cov … --fail-under-lines 80`、`npm run test:coverage` | build-and-test | Met |
| U4-NFR6.12 | construction/u4-elapsed-time/nfr-requirements/tech-stack-decisions.md | 純粋な判断はテストを先に書く：表記の関数、画面の合わせ直しと保持の判断（受け取った elapsed と今の表示から次の表示の起点を決める関数、NFR6.6・NFR6.9）、AppSession の経過の決まり。タイマ… | 対応するテストが成功 | `src/elapsed.test.ts`、`npm run test:coverage`（385 件成功・0 件失敗） | build-and-test | Met |
| U5-NFR2.5 | construction/u5-cancel/nfr-requirements/observability-requirements.md | 取得 1 回の要約の 1 行の status に Cancelled が入る。ほかの欄は今と同じ | 対応するテストが成功 | `crates/local-sights-core/src/fetch_status.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR9.3 | construction/u5-cancel/nfr-requirements/observability-requirements.md | 新しい文言（中断、中断中…、中断、注意 2 種）は英語と日本語を用意し、OS の言語で出し分ける | 対応するテストが成功 | `src/i18n/messages.ts`、`npm run test:coverage`（385 件成功・0 件失敗） | build-and-test | Met |
| U5-NFR9.4 | construction/u5-cancel/nfr-requirements/observability-requirements.md | 中断の注意は色だけでなくアイコンと文で伝え、出たときに一度だけ読み上げる（role=status の囲み） | 画面のテストで role と文を確認。VoiceOver の確認は利用者の判断で外した（Q2=A） | `src/components/CancelNotice.test.tsx`、`npm run test:coverage`。`build-and-test-questions.md` Q2 | build-and-test | Met |
| U5-NFR10.1 | construction/u5-cancel/nfr-requirements/observability-requirements.md | ［中断］は［取得］と同じ位置・同じタブ順で、キーボードだけで押せる。押した後もフォーカスはボタンに残る | 対応するテストが成功 | `src/components/ConditionArea.tsx`、`npm run test:coverage`（385 件成功・0 件失敗） | build-and-test | Met |
| U5-NFR4.2 | construction/u5-cancel/nfr-requirements/performance-requirements.md | ［中断］の後に始まる GetLogEvents・DescribeLogStreams は 0 回 | 対応するテストが成功 | `crates/local-sights-core/tests/u5_cancel.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR4.3 | construction/u5-cancel/nfr-requirements/performance-requirements.md | 中断したら、待ちの最中でも応答待ちの最中でも、すぐに止まる。出ている呼び出し（GetLogEvents・DescribeLogStreams）の応答は待たずに打ち切る | 対応するテストが成功 | `crates/local-sights-core/tests/u5_cancel.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR4.4 | construction/u5-cancel/nfr-requirements/performance-requirements.md | 打ち切った呼び出しのストリームは「止めた」として N に入る。打ち切った呼び出しのページは一覧に足さない。一覧の呼び出しを打ち切ったら一覧の前の中断になる | 対応するテストが成功 | `crates/local-sights-core/tests/u5_cancel.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR4.5 | construction/u5-cancel/nfr-requirements/performance-requirements.md | 最後を読み終えた直後の中断を終わった扱いにする（FR5.9）のは、中断が届く前にすべてのストリームを読み終えていた場合に限る（応答待ちのストリームがあれば、それは打ち切って N に入る） | 対応するテストが成功 | `crates/local-sights-core/tests/u5_cancel.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR5.5 | construction/u5-cancel/nfr-requirements/performance-requirements.md | 中断した後も、取れたログの一覧と絞り込みは今と同じに使える（一覧の部品には手を入れないため、速度測定テストは流さない） | 対応するテストが成功 | `src/App.test.tsx`、`npm run test:coverage`（385 件成功・0 件失敗） | build-and-test | Met |
| U5-NFR6.15 | construction/u5-cancel/nfr-requirements/reliability-requirements.md | 閉じるは常に中断より強い。中断の後に閉じても、閉じるときの今の動き（確認、取れたログを捨てて終わる）は変わらない | 対応するテストが成功 | `crates/local-sights-core/tests/u1_concurrent_fetch.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR6.16 | construction/u5-cancel/nfr-requirements/reliability-requirements.md | どの終わり方（Completed・CompletedWithFailures・Failed・Aborted・Cancelled・異常な停止）でも［中断中…］が戻る | 対応するテストが成功 | `crates/local-sights-core/src/session.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR6.17 | construction/u5-cancel/nfr-requirements/reliability-requirements.md | 取得を始める操作は、止める信号の持ち手を session のロックを持ったまま AppState に登録する（今の begin_and_spawn_fetch はロックを手放してから登録しているので、順を入れ替える）… | 対応するテストが成功 | `src-tauri/src/lib.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR6.18 | construction/u5-cancel/nfr-requirements/reliability-requirements.md | すべてのストリームを読み終えた後・取得の終わりの前に届いた中断は終わった扱い（FR5.9、NFR4.5）。ログは消えず、キャッシュの決まりも今のまま | 対応するテストが成功 | `crates/local-sights-core/tests/u5_cancel.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR6.19 | construction/u5-cancel/nfr-requirements/reliability-requirements.md | キャッシュを引いている間の中断は、当たれば Completed、当たらなければ一覧の前の中断になる。どちらでも取得は必ず終わる | 対応するテストが成功 | `crates/local-sights-core/tests/u5_cancel.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR4.6 | construction/u5-cancel/nfr-requirements/scalability-requirements.md | ストリームが多くても（数百〜数千本）、N の数え方と止め方は変わらない。まだ始めていないストリームは呼ばずに数えるだけ | 対応するテストが成功 | `crates/local-sights-core/tests/u5_cancel.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR6.14 | construction/u5-cancel/nfr-requirements/scalability-requirements.md | 中断を何度繰り返しても、注意・中断中の状態は取得 1 回分だけ持つ（次の取得で消え、増えていかない） | 対応するテストが成功 | `crates/local-sights-core/src/session.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR8.13 | construction/u5-cancel/nfr-requirements/security-requirements.md | 中断の注意・状態行・要約の 1 行に足すのは、本数（N）と決まった語（Cancelled、中断など）だけ。ストリーム名・ロググループ名・接続の情報・失敗の詳細を足さない | 対応するテストが成功 | `crates/local-sights-core/src/session.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR8.14 | construction/u5-cancel/nfr-requirements/security-requirements.md | 新しい AWS の API・依存・外への送信を足さない。呼ぶのは読み取り 3 つだけのまま | 新しい API・依存・送信なし。cargo-deny と npm audit も問題なし | main との差分の確認、`cargo deny check`、`npm audit --audit-level=high`（0 件） | build-and-test | Met |
| U5-NFR8.15 | construction/u5-cancel/nfr-requirements/security-requirements.md | Tauri の権限に足すのは `cancel_fetch` だけ。このコマンドは走っている取得を止めるだけで、入力を取らない | 対応するテストが成功 | `src-tauri/capabilities/default.json`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR6.13 | construction/u5-cancel/nfr-requirements/security-requirements.md | 自動テストと CI は実際の AWS に接続しない。中断のテストは偽物のゲートウェイと止めた時計で行う | 対応するテストが成功 | `crates/local-sights-core/tests/support/fake_gateway.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| U5-NFR7.4 | construction/u5-cancel/nfr-requirements/tech-stack-decisions.md | 行カバレッジはライブラリ・画面ともに 80% 以上を保つ | ライブラリ 97.67%、画面 90.85% | `cargo llvm-cov … --fail-under-lines 80`、`npm run test:coverage` | build-and-test | Met |
| U5-NFR6.20 | construction/u5-cancel/nfr-requirements/tech-stack-decisions.md | 純粋な判断（N の数え方、終わりの状態の決め方、押せるかの判定）はテストを先に書く。取得の流れのつなぎ込み・Tauri のコマンド・画面は実装してからテストを書く | 対応するテストが成功 | `crates/local-sights-core/src/coordinator.rs`、`cargo test --workspace`（548 件成功・0 件失敗・5 件は手動の速度測定） | build-and-test | Met |
| TC-COV-LIB | 各作業単位 code-generation-plan.md ## Testing Contract（scope_floor） | ライブラリの行カバレッジ 80% 以上 | 97.67% | `cargo llvm-cov -p local-sights-core --fail-under-lines 80`（終了コード 0） | build-and-test | Met |
| TC-COV-UI | 同上（scope_floor） | 画面の行カバレッジ 80% 以上 | 90.85% | `npm run test:coverage`（終了コード 0） | build-and-test | Met |
| TC-CI | 同上（scope_floor） | 選んだテストをマージ前に CI で実行する | `.github/workflows/ci.yml` が fmt・clippy・llvm-cov・Vitest のカバレッジ・tsc・ESLint・Prettier・cargo-deny・npm audit を実行する設定になっている。この枝の PR の CI はまだ走っていない | `.github/workflows/ci.yml`。PR の CI の結果は ci-pipeline で確かめる | build-and-test | Met |

集計：89 件のうち Met 89 件、Unverified 0 件、Not Met 0 件。U1-NFR2.1・U4-NFR2.3・U4-NFR6.7 は開発者の手元の確認（2026-10-15）で、U4-NFR9.2・U5-NFR9.4 は画面のテストで判定した。後の 2 件の確かめ方にあった「VoiceOver で一度確かめる」は、要件（NFR9・NFR10）に読み上げの条件がないため、利用者の判断で外した（`build-and-test-questions.md` Q2）。

## 準備の状態

| 項目 | 状態 | 理由 |
|------|------|------|
| build-ready | はい | ビルドのコマンドがすべて成功 |
| test-ready | はい | 自動のテストと検査がすべて成功、カバレッジは下限を超える |
| deployment-ready | 条件つきではい | 目標はすべて Met。要件の対応表で直接の対応がない 45 件は指摘として承認のときに示す（`cross-unit-traceability.md`）。この枝の PR の CI はまだ走っていない（ci-pipeline で確かめる） |

## 残っている項目

- **手元の確認**：2026-10-15 に開発者の手元で済んだ（速さの比べ、経過、スリープ、中断）。その際の手順の直し（zsh で動く書き方）は f95829a。VoiceOver の確認は利用者の判断で行わない（Q2）。
- **要件の対応表**：直接の対応 49 件、間接 44 件、未確認 1 件（指摘 BT-TR-1〜4）。
- **レビューで残した Minor**（作業単位の中では直さず、次に回すと決めたもの）：
  - U4：`getSession` の番号付けの古い説明、StrictMode のテスト。
  - U5：CancelNotice の読み上げの囲みが中身と同時に出る点、英語の「1 streams」の複数形、取得の始めの表示の競合の注記、権限のテストがコマンドの数を 26 で固定している点。
- **長く続いている枝**：`claude/parallel-fetch` は main から 54 コミット。team.md の「短命の枝（1〜2 日）」より長い。PR は作るよう頼まれたときに作る。
