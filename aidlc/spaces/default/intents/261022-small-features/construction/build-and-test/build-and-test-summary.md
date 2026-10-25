# Build and Test Summary — アプリの中での言語切替・書き出し・JSON の整形（261022-small-features）

## 1. ビルドの状態と前提

- **ビルド**：成功（`build-instructions.md` の 4 のコマンドをすべて通した。詳細は `test-results.md` の 2）。
- **前提**：Rust stable（edition 2024）、Node.js 22、cargo-llvm-cov 0.9.1、cargo-deny 0.20.2、Linux では WebKitGTK/GTK の開発用パッケージ。AWS の認証情報は要らない。
- **この段で直したこと**：`npm audit` の既存の high 2 件（lockfile の更新）、`cargo deny` の既存の勧告 1 件（`ctutils` の更新）と、U2 の依存に入った Windows 向けクレートの BSL-1.0 の許可（`deny.toml`）。生成したコードとテストは変えていない。

## 2. テストの種類の一覧

| 種類 | 指示書 | 実行 |
|---|---|---|
| 単体（Rust の純粋ロジックと境界、画面の部品） | U1・U2 の `unit-test-instructions.md` | 実行（`cargo test --workspace`、`npm run test:coverage`） |
| 結合（作業単位の境目・部品の境目） | `integration-test-instructions.md` | 実行（`cargo llvm-cov -p local-sights-core`、`cargo test -p local-sights`、Vitest） |
| 性能（応答性 NFR2） | `performance-test-instructions.md` | 自動の部分は実行。100 万行の体感は手元 |
| セキュリティ（NFR5、Forbidden、依存関係） | `security-test-instructions.md` | 実行（テスト、`npm audit`、`cargo deny`、差分の確認） |
| 契約・E2E・アクセシビリティの独立した指示書 | なし | NFR6 は部品のテスト（aria の名前とキー操作）の中で確かめた。E2E は手元の目視に含める |

Test Strategy は `Standard`、scope は `refactor`（新しいテストの下限は足さず、既存のテストが緑のまま）。

## 3. 作業単位ごとのカバレッジの期待と実績

| 作業単位 | 期待 | 実績 |
|---|---|---|
| U1 `u1-language-switch` | 中核の公開関数（`LanguagePreference`・`load_settings`・`save_settings`）にテスト。新規およそ 26 本。既存は緑のまま | `cache/settings.rs` 99.57%、`session.rs` 98.25%。画面 `SettingsDialog`・`App` のテストが通過 |
| U2 `u2-export-and-json` | 中核の公開関数（`write_csv`・`write_markdown`・`sanitize_file_stem`・`default_file_name`・`into_snapshot`・`render_to`・`begin_export`・`finish_export`・`can_export_logs`・`can_export`・`export_source`）にテスト。既存は緑のまま | `export/exporter.rs` 99.62%、`export/formatter.rs` 98.34%、`insights_session.rs` 97.44%、`export_boundary`（アプリ）のテスト 5 本以上が通過。`jsonPretty.ts`・`ExpandedMessage`・`ExportMenu`・`ExportNoticeAlert` のテストが通過 |
| 全体 | 中核 80% 以上、画面 80% 以上（team.md） | 中核 96.74%、画面 90.48% |

## Target Verification Matrix

出所：team.md（Testing Posture・Code Style）、`.github/workflows/ci.yml`、要件の NFR1〜NFR6、U1・U2 の `code-generation-plan.md`（Testing Contract と「完了の目安」）、記録済みの確認コマンド。数値の目標がない NFR2 は、構造と自動テストで確かめる。

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| T-VER | ci.yml / team.md Deployment | 4 つの版がそろう | そろっている（0.2.0） | `npm run check:version` 成功（test-results 2） | build-and-test | Met |
| T-FMT | team.md Code Style | `cargo fmt --all --check` が通る | 通った | test-results 2 | build-and-test | Met |
| T-CLIPPY | team.md Code Style | deny レベルの lint が 0（警告はエラーにしない） | error 0、warning 0 | `cargo clippy --workspace --all-targets`（test-results 2） | build-and-test | Met |
| T-TSC | team.md Code Style | `tsc --noEmit` が通る | 通った | test-results 2 | build-and-test | Met |
| T-ESLINT | team.md Code Style | `eslint .` が通る | 通った | test-results 2 | build-and-test | Met |
| T-PRETTIER | team.md Code Style | `prettier --check .` が通る | 通った | test-results 2 | build-and-test | Met |
| T-BUILD | ci.yml | `vite build` と `cargo build -p local-sights` が通る | 通った | test-results 2 | build-and-test | Met |
| T-RUST-TEST | 確認コマンド / NFR1 | `cargo test --workspace` が全件通る | 865 通過、0 失敗、9 無視（既存） | test-results 3.1 | build-and-test | Met |
| T-VITEST | 確認コマンド / NFR1 | `npx vitest run` が全件通る | 549 通過、0 失敗 | test-results 3.2 | build-and-test | Met |
| T-COV-CORE | team.md Testing Posture / NFR4 | 中核の行カバレッジ 80% 以上 | 96.74% | `cargo llvm-cov -p local-sights-core --fail-under-lines 80`（test-results 4） | build-and-test | Met |
| T-COV-FE | team.md Testing Posture / NFR4 | 画面の行カバレッジ 80% 以上 | 90.48% | `npm run test:coverage`（test-results 4） | build-and-test | Met |
| T-AUDIT | team.md Code Style | `npm audit --audit-level=high` が 0 件 | 0 件（`npm audit fix` 後） | test-results 1・2 | build-and-test | Met |
| T-DENY | team.md Code Style | `cargo deny check` の 4 検査が ok | advisories ok, bans ok, licenses ok, sources ok（修正後） | test-results 1・2 | build-and-test | Met |
| T-ORDER | NFR4 / Testing Contract | 純粋ロジックはテスト先行、境界は実装後 | 両作業単位の計画が [テスト] → [実装] の順に並び、全手順が完了。U1 の局所的な入れ替えも順序の原則を保った（code-generation の diary） | U1・U2 `code-generation-plan.md`、`code-summary.md` | build-and-test | Met |
| T-NFR2 | NFR2 | 書き出し中も画面が応じる（数値の目標なし） | `start_export` は錠の中で複製だけを行い、変換と書き込みは `spawn_blocking` で錠を離して行う。Running 中にメニュー以外の操作が有効なことを `App.test.tsx` が確かめる。JSON の整形は再帰なし・上限あり | `src-tauri/src/lib.rs`、`src/App.test.tsx`、`src/jsonPretty.test.ts` | build-and-test | Met |
| T-NFR5 | NFR5 / Forbidden | 設定ファイルは 3 項目だけ。失敗の知らせに OS の文を出さない。新しい AWS の API・外部送信なし | テスト `language_is_read_back_and_defaults_to_system`・`export_failure_kinds_follow_the_io_error_kind` が通過。`git diff main...HEAD` に AWS 呼び出しと HTTP の送信の追加なし。足した依存は OS のダイアログとクリップボードだけ | `security-test-instructions.md` 1 | build-and-test | Met |
| T-NFR6 | NFR6 | 足した部品がキーボードで操作でき読み上げの名前を持つ | `ExportMenu.test.tsx`（キーボードで開いて選べる、ariaLabel）、`ExpandedMessage.test.tsx`（切替のキー操作が行に伝わらない）、`SettingsDialog.test.tsx` が通過 | test-results 3.2 | build-and-test | Met |
| T-PERMS | U2 レビュー R-04 / Forbidden | WebView の権限は 2 つのコマンドだけ増える | `capabilities/default.json` の追加は `allow-start-export`・`allow-dismiss-export-notice` のみ。`commandRegistration.test.ts` 通過 | `security-test-instructions.md` 1 | build-and-test | Met |
| T-OS-CI | team.md Testing Posture / NFR3 | Rust のテストと clippy・ビルドが CI の macOS・Windows のランナーでも通る | 通った（承認後に pull request #46 を作り、CI run 167 が 4 ジョブすべて成功。2026-10-24T23:50Z） | https://github.com/kdnakt/local-sights/actions/runs/40143537037 | build-and-test（承認後の追記） | Met |
| T-OS-MAC | team.md Walking Skeleton 3 / NFR3 | macOS の手元で GUI を起動し、README のチェックリストを目で確かめる | 良好（開発者が 2026-10-25 に手元で確認） | test-results 6 | 人（開発者） | Met |
| T-OS-WIN | team.md Walking Skeleton 3 / NFR3 | Windows 11 の手元で、CI で作ったインストーラで入れたアプリで同じく確かめる | 良好（開発者が 2026-10-25 に手元で確認） | test-results 6 | 人（開発者） | Met |

`Pending` は残っていない。承認時は `Met` 18、`Not Met` 0、`Unverified` 3。承認後に pull request #46 の CI が緑になり T-OS-CI を `Met` に、開発者の手元の確認で T-OS-MAC・T-OS-WIN を `Met` に更新した（`Met` 21、`Unverified` 0）。

## 4. 準備の状態（readiness）

| 観点 | 判定 | 根拠 |
|---|---|---|
| build-ready | はい | CI と同じ検査がすべて Linux で通った |
| test-ready | はい | 自動テスト 865 + 549 本が通り、カバレッジは両方とも 80% を上回る |
| deployment-ready（＝配布できるか） | **まだ** | 3 つの `Unverified`：CI の macOS・Windows のランナー（pull request を作ると走る）と、両 OS での手元の目視。team.md Deployment の手順（版を上げる PR → Release dry run → 両 OS での手元の確認 → タグ）はこの作業の外で、次の機能と合わせて配る |

この段の合否の決まり（どの対象も `Unverified` を残さない）に照らすと、**この実行は失敗**として扱う。原因はこの段の設定や環境の中では直せず（GUI も他の OS のランナーもこのコンテナにはない）、生成したコードの欠陥でもないため、code-generation への戻し（loop-back）はあてはまらない。承認ゲートの前に停止して人に問う（5）。

## 5. 停止して問うこと

候補と影響の見積もり：

| 候補 | 内容 | 手間 | 費用 | リスク |
|---|---|---|---|---|
| A. pull request を作って CI を回す | `claude/zen-goldberg-0ogpm0` から `main` への pull request を作る。CI の `Tauri app (macos-latest)` / `(windows-latest)` が走り、T-OS-CI の証拠になる。team.md の流れ（PR → CI 緑 → スクワッシュマージ）どおり | 数分（CI は 15〜25 分） | GitHub Actions の macOS・Windows ランナーの分数 | 低。CI が赤なら原因はこのブランチにある |
| B. 手元の目視（T-OS-MAC・T-OS-WIN） | 開発者が macOS と Windows 11 で README のチェックリストを確かめ、`test-results.md` の 6 に結果を足す | 30 分〜1 時間 × 2 OS | なし | なし（確認のみ） |
| C. Linux の結果だけで承認する | 3 つの `Unverified` を残したまま承認し、CI と目視はマージ前・配布前の作業として持ち越す | なし | なし | 中。OS ごとに分かれる `export_boundary`（Windows の rename・権限）と保存ダイアログ・クリップボードは Linux のテストだけでは確かめられていない |

A と B は両立する。この段のできることは A の pull request の作成まで（人の指示があれば）。B は人の作業である。

## 6. 既知の制限と残っていること

- **コードレビューの Minor（凍結済みの成果物のため直していない。次の作業の候補）**：U1 R-01 `cloudscapeMessages` の配列を `useMemo` に入れる。U2 R-01 複製した行数と `begin_export` の行数の一致を確かめる。U2 R-02 `findJson` の候補をまたいだ走査量の合計に上限を置く（最悪 64 × 1 MiB）。U2 R-03 Markdown のセルの末尾 `\` の扱いを決める（式のような値と `\` の注意は README に書いた）。
- **手元の確認**：T-OS-MAC・T-OS-WIN（5 の B）。
- **CI**：T-OS-CI（5 の A）。
- **ドキュメント**：README（状態の段落、チェックリスト「Language, export and JSON (261022-small-features)」）と `docs/install.md`（設定ファイルの中身）をこの段で更新した。
- **配布**：この作業だけでは配らない（team.md Deployment）。次の機能が終わったあと、版を上げる PR → Release dry run → 両 OS での確認 → タグの順で進める。
