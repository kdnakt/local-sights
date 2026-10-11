# Build and Test Summary — 画面を AWS マネジメントコンソールに寄せる

## 全体の状況

- **ビルド**：成功（画面の束、Rust ワークスペース全体）。
- **自動テスト**：vitest 246 件、cargo test 358 件がすべて成功（既存の `#[ignore]` 5 件を除く）。
- **静的検査**：tsc、eslint、prettier、rustfmt、clippy がすべて成功。
- **依存**：npm audit 0 件。cargo-deny は手元で未実行（CI で実行）。
- **前提**：Node 22、Rust stable（MSRV 1.94.1 以上）、macOS の Xcode Command Line Tools。詳細は `build-instructions.md`。

## 作ったテスト手順

| ファイル | 内容 |
|---|---|
| `build-instructions.md` | 依存の導入、ビルド、確認のコマンド、よくある問題 |
| `integration-test-instructions.md` | 画面とコア、コアと取得範囲、コアと AWS の境界（スタブ）のテスト |
| `performance-test-instructions.md` | NFR2 の 100 万行の手動測定（PERF-1〜PERF-5） |
| `security-test-instructions.md` | 依存・ライセンス・AWS API・CSP・秘密情報（SEC-1〜SEC-7） |

テスト戦略は Standard のため結合テストの手順を作った。性能とセキュリティは要件（NFR2、NFR5、CN3）があるため加えた。

## カバレッジの見込み

数値の下限は設けない（team.md、スコープ bugfix）。bugfix の下限として、対象の回帰テスト 3 つ（欄ごとのエラー表示、3 行構成のアラート、スラッシュ書式の解析）があり、既存のスイートは緑のまま。

## Target Verification Matrix

目標の出どころは `inception/requirements-analysis/requirements.md`（成功指標と非機能要件）、`construction/code-generation/code-generation-plan.md` の Testing Contract、security-test-instructions.md。NFR の要件・設計ステージは今回の計画にない。

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| NFR1 / SM1 | requirements.md §3・§1 | 見える部品がすべて Cloudscape（ログ一覧は NFR2 の但し書きで自前を許す） | ログ一覧以外は Cloudscape。ログ一覧は自前の仮想スクロール。但し書きの条件（Cloudscape の表で性能を満たせない）は未測定 | code-summary.md、`LOG_TABLE_VARIANT` | build-and-test（手動） | Unverified |
| NFR2 | requirements.md §3 | 100 万件で絞り込み 100 秒以内、操作に 1 秒以内、取得中も固まらない | 未測定 | performance-test-instructions.md | build-and-test（手動） | Unverified |
| NFR3 | requirements.md §3 | 主な流れをマウスなしで操作、ダイアログは Escape で閉じる | 部品ごとのキーボード操作と Escape はテストで通過。一連の流れの実機確認は未実施 | vitest（ConditionArea、各ダイアログ） | build-and-test（手動） | Unverified |
| NFR4 | requirements.md §3 | 英語と日本語 | 新しい key が両言語にあり、Cloudscape の文言も言語に合わせる | `src/i18n/messages.test.ts` 通過 | build-and-test | Met |
| NFR5 | requirements.md §3 | Cloudscape（Apache-2.0）を追加し、依存チェックと秘密情報スキャンを通す。Rust に新しい依存を足さない | npm audit 0 件、ライセンス Apache-2.0、Rust の直接依存の追加なし。cargo-deny は未実行 | test-results.md SEC-1〜SEC-3 | build-and-test（CI の cargo-deny） | Unverified |
| NFR6 | requirements.md §3 | 既存の cargo test と vitest がすべて通る | 358 件・246 件すべて通過 | test-results.md | build-and-test | Met |
| NFR7 | requirements.md §3 | 1024×640 で 2 ペインと条件エリアが崩れない | 未確認 | README の手動確認チェックリスト | build-and-test（手動） | Unverified |
| SM2 | requirements.md §1 | 入力欄と時刻列が `yyyy/mm/dd` 系 | コアと画面のテストで通過 | `time_range.rs`、`dateTime.test.ts` | build-and-test | Met |
| SM3 | requirements.md §1 | 開始・終了の日付をカレンダーから選べる | テストで通過（未来日は無効） | `ConditionArea.test.tsx` | build-and-test | Met |
| SM4 | requirements.md §1 | 入力の不備は欄の直下、取得全体の失敗はアラート | 回帰テスト 2 つが通過 | `ConditionArea.test.tsx`、`FetchErrorBanner.test.tsx` | build-and-test | Met |
| SM5 | requirements.md §1 | 開始・終了が日付欄と時刻欄に分かれている | テストで通過 | `ConditionArea.test.tsx` | build-and-test | Met |
| SM6 | requirements.md §1 | コンソールと並べて「同じ流儀」と開発者本人が納得する | 未判定 | ゲートでの本人の判断 | build-and-test（人） | Unverified |
| TC-1 | Testing Contract（scope_floor） | 対象の回帰テストがあり、既存のスイートが緑 | 回帰テスト 3 つがあり、全体が通過 | test-results.md | build-and-test | Met |
| SEC-4 | CN3、security-test-instructions.md | 呼ぶ AWS API が増えない | 3 つの読み取り API だけ | test-results.md SEC-4 | build-and-test | Met |
| SEC-5 | CN3、security-test-instructions.md | テレメトリ・外部通信がない | CSP に外部の接続先なし | `src-tauri/tauri.conf.json` | build-and-test | Met |
| SEC-7 | CN3、project.md | テストと CI が実際の AWS に接続しない | 接続なし | `.github/workflows/ci.yml` | build-and-test | Met |

## 準備の判定

| 観点 | 判定 |
|---|---|
| ビルドの準備 | 整っている |
| テストの準備 | 自動テストは整っている。実機の手動確認が残る |
| 配布の準備 | 未。`Unverified` の 6 件（NFR1、NFR2、NFR3、NFR5、NFR7、SM6）を確かめるまで |

## 未解決の項目

- 実機での手動確認（README のチェックリスト）と 100 万行の性能測定。
- cargo-deny は PR の CI で確かめる。
- コード生成のレビューで残った指摘：R-02（NFR2 と SM1 の例外）、R-04、R-05、R-06、R-07、R-09、R-10、R-11、Step 18 の新規 Minor（アラートが複数のときのログ一覧の高さ）。いずれも実機確認待ちか、Minor の改善。
