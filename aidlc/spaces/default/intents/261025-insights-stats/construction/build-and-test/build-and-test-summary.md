# Build and Test Summary — 261025-insights-stats

## 1. ビルドの状態

- ビルドと検査は、すべて成功した：`cargo build`・`fmt`・`clippy`（警告 0）・`prettier`・`eslint`・`tsc`・`cargo deny`・`npm audit`。
- 前提の用意は build-instructions.md のとおり。自動テストとビルドに AWS の認証情報は要らない。

## 2. 作ったテストの手順

| 種類 | ファイル | 中身 |
|---|---|---|
| ビルド | build-instructions.md | 依存関係、ビルドと検査のコマンド、よくある問題 |
| 結合 | integration-test-instructions.md | 4 つの作業単位の境界、手元で行うつながりの確かめ |
| 速さ | performance-test-instructions.md | U2 の 3 倍、写しの鍵、前の速さ、NFR1 の手元の実測 |
| セキュリティ | security-test-instructions.md | 読み取りの 3 つの API、秘密の情報、外部への送信、依存関係、新しいコマンドの権限 |
| 単体 | 各作業単位の unit-test-instructions.md | Code Generation で作ったもの |

## 3. 作業単位ごとのカバレッジ

中核ライブラリ全体は 96.17%、画面全体は 91.03% で、下限の 80% を上回った（test-results.md の 3）。

| 作業単位 | 主なファイルの行カバレッジ |
|---|---|
| U1 | `stats.rs` 99.22%、`plan.rs` 98.45%、`parser.rs` 95.76% |
| U2 | `catalog.rs` 88.13%、`evaluator.rs` 90.85% |
| U3 | `store.rs` 87.61%、`runner.rs` 96.30%、`session.rs` 97.90%（U3 の作る段階の測定で `insights_session.rs` 98.32%） |
| U4 | examples は中核の計測の外。比べる処理は 85 本のテストで確かめた |

## Target Verification Matrix

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| T-STYLE | team.md Code Style | fmt・clippy・prettier・eslint・tsc が通る | すべて通った | test-results.md の 1 | build-and-test | Met |
| T-DEPS | team.md Code Style | cargo-deny と npm audit（high）が通る | どちらも通った | test-results.md の 1 | build-and-test | Met |
| T-C5 | requirements.md C5・NFR8、各計画の BR5.1 | 既存のテストがすべて通る | 中核 848・Vitest 597 を含め、すべて通った（Linux） | test-results.md の 2 | build-and-test | Met |
| T-NFR8-CORE | NFR8、Testing Contract（80%） | 中核ライブラリの行カバレッジ 80% 以上 | 96.17% | test-results.md の 3 | build-and-test | Met |
| T-NFR8-UI | NFR8、Testing Contract（80%） | 画面の行カバレッジ 80% 以上 | 91.03% | test-results.md の 3 | build-and-test | Met |
| T-NFR8-OS | NFR8・NFR6、team.md Testing Posture | Rust のテストが Linux・macOS・Windows で通る | PR kdnakt/local-sights#50 の CI（`4393fcc`）で、Rust library・Tauri app（ubuntu-latest・macos-latest・windows-latest）・Front end・cargo-deny がすべて成功 | test-results.md の 8 | PR の CI | Met |
| T-U2-BR26 | U2 rules.md BR2.6、U2 Q1 A | 重いクエリが基準 `stats sum(latency) by @logStream` の 3 倍以内（100 万件） | 2.14〜2.37 倍、測定のテストが通った | test-results.md の 4 | build-and-test | Met |
| T-NFR2 | NFR2 | 100 万件に `stats` を実行しても落ちず、評価を止められる | 落ちず、止める信号のテストと速さのテストが通った | test-results.md の 4 | build-and-test | Met |
| T-U3-COPY | U3 計画 Step 22 | 取得中の保持の写しの 1 回の鍵が 50 ms 以内 | 8〜13 ms（塊に分けたあと） | test-results.md の 4 | build-and-test | Met |
| T-BASE-SPEED | 前の作業の NFR1、C5 | 前の速さのテスト（10 万件 10 秒・100 万件 100 秒）が通る | すべて通った | test-results.md の 4 | build-and-test | Met |
| T-NFR1 | NFR1、OQ3 | 途中結果で取得が遅くならない（合否の数値は手元の実測から決め、決めたら下げない。暫定の目安は 10%） | 測っていない | performance-test-instructions.md | 開発者の手元。この計画に後の段はない | Unverified |
| T-SM1 | SM1・NFR3、U4 BR3.1 | 互換を確かめる集計のクエリで、コンソールとアプリの結果が同じ | 流していない | U4 code-summary.md の表 | 開発者の手元。この計画に後の段はない | Unverified |
| T-WS-OS | team.md Walking Skeleton の 2・3、NFR6 | macOS と Windows 11 で、実際の AWS に対して集計・途中結果・中断の結果が出る。ログ管理の画面が前と同じ | 確かめていない | integration-test-instructions.md | 開発者の手元 | Unverified |
| T-NFR4 | NFR4、project.md Forbidden | 自動テストと CI が AWS に接続しない。書き出しや秘密の情報をリポジトリに置かない | テストは偽の取得元と偽の CLI だけ。差分に該当する行なし | test-results.md の 5 | build-and-test | Met |
| T-NFR5 | NFR5 | 呼ぶ AWS API は読み取りの 3 つだけ | 新しい API の呼び出しなし | test-results.md の 5 | build-and-test | Met |
| T-NFR11 | NFR11 | Cloudscape の標準の部品だけ | 新しい UI のライブラリ・独自のデザインなし | test-results.md の 5 | build-and-test | Met |
| T-NFR12 | NFR12 | 外部に何も送らない | 外部への送信を足した行なし | test-results.md の 5 | build-and-test | Met |
| T-TRACE | Build and Test Step 10 | すべての FR・NFR が作業単位の追跡表に OK で載る | FR 37 件はすべて OK。NFR は 12 件のうち 7 件で、NFR5・NFR6・NFR8・NFR11・NFR12 の 5 件が載っていない（この表の T-NFR5・T-NFR8-*・T-NFR11・T-NFR12・T-WS-OS で直接確かめた） | cross-unit-traceability.md | build-and-test | Not Met |

## 4. 準備の具合

- ビルド：できている（build-ready）。
- テスト：このコンテナで流せるものは、すべて通った。`Unverified` が 3 つ（T-NFR1、T-SM1、T-WS-OS。T-NFR8-OS は PR の CI で Met になった）、`Not Met` が 1 つ（T-TRACE）残っているため、test-ready とはまだ言えない。
- 配布：まだできない。手元の確かめが済んでいないため。

## 5. 残っていること

- **`Unverified` の 4 つ**：どれも、このコンテナでは流せない確かめ（本人の AWS 認証情報、本物の GUI、macOS・Windows の CI）。コードの誤りではないため、Code Generation に戻る直しはない（test-results.md の 6・7）。
- **T-TRACE**：作業全体にかかる NFR 5 件が、作業単位の追跡表に載っていなかった。中身はこの表で直接確かめてあり、承認の場で示す。
- **レビューの持ち越し**：
  - U3 のコード生成のレビュー R-01（途中の view を作る間の store の鍵が 100 万件で約 600 ms）は、T-NFR1 の実測の結果で扱いを決める。
  - U4 のレビューの低い所見（R-01〜R-04）は、直さずに進めてよいとされた。
- **NFR1 の合否の数値**：手元の実測から決め、決めたら下げない（OQ3）。
