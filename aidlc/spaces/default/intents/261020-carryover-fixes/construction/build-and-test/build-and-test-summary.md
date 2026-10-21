# Build and Test のまとめ（261020-carryover-fixes）

## 全体の状態

- **ビルド**：成功。画面のビルド、ライブラリ、Tauri のアプリ（Linux）が通った。CI では Linux・macOS・Windows の 3 つで通った。
- **テスト**：
  - 手元で Rust 762 件、例 58 件、速さ 9 件、画面 496 件がすべて通った。
  - CI は PR [kdnakt/local-sights#36](https://github.com/kdnakt/local-sights/pull/36) の最後のコミット（13bc6a9）で、7 つのジョブがすべて成功した。
- **前提**：Rust の stable、Node.js、Linux では WebKitGTK などの開発用ライブラリ（`build-instructions.md`）。AWS の認証情報は要らない。
- **この段で足した直し**：
  - CI の `cargo-deny` が、新しく出た勧告 RUSTSEC-2026-0382 で落ちた。原因は `rustls-webpki` 0.103.15 で、この作業の変更とは関係がない。
  - `Cargo.lock` だけを直して 0.103.16 に上げた（e6c53b4）。PR にもコメントで書いた。

## 作ったテストの手引き

Test Strategy は Standard。

| 種類 | ファイル | 中身 |
|---|---|---|
| ビルド | `build-instructions.md` | 依存関係、ビルド、CI と同じ検査、つまずきやすい点 |
| 結合 | `integration-test-instructions.md` | 取得 × キャッシュ × 中断、クエリの実行 × 保持、コマンドの受け渡し、3 か所の権限の一覧、名前の表 × Rust の一覧。macOS の目視の手順 |
| 速さ | `performance-test-instructions.md` | NFR4：既存の速さのテスト 9 件 |
| 安全 | `security-test-instructions.md` | 読み取りの 3 つ以外の API を呼ばない、ネットワークから名前を取らない、認証情報を書かない、依存関係の脆弱性 |

単体テストは Code Generation の `unit-test-instructions.md` のとおり。

## カバレッジ

作業単位はないため、段の単位で見る。

| 対象 | 下限 | 実際 |
|---|---|---|
| ライブラリ（`cargo-llvm-cov`） | 80% | 96.63%（手元）。CI でも `--fail-under-lines 80` で成功 |
| 画面（Vitest） | 80% | 90.01%（手元）。CI でも成功 |

## Target Verification Matrix

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| T-NFR1-linux | requirements.md NFR1 | Rust と画面のテストがすべて通る（Linux） | Rust 762/762、画面 496/496 | test-results.md「テスト」。CI の Rust library・Front end・Tauri app (ubuntu-latest) | build-and-test | Met |
| T-NFR1-macos | requirements.md NFR1 | Rust のテストが macOS のランナーで通る | 成功 | CI の Tauri app (macos-latest)、run 39542854716 | build-and-test | Met |
| T-NFR1-windows | requirements.md NFR1 | Rust のテストが Windows のランナーで通る | 成功 | CI の Tauri app (windows-latest)、run 39542854716 | build-and-test | Met |
| T-NFR2 | requirements.md NFR2 | FR1〜FR5 のそれぞれに狙ったテストがある。純粋なロジックはテスト先行 | 各 FR にテストがあり、テスト先行の 4 か所は Red の記録がある | code-generation/code-summary.md、traceability.json | build-and-test | Met |
| T-NFR3-lib | requirements.md NFR3、team.md Testing Posture | ライブラリの行カバレッジ ≥ 80% | 96.63% | `cargo llvm-cov -p local-sights-core --fail-under-lines 80`（手元・CI） | build-and-test | Met |
| T-NFR3-ui | requirements.md NFR3、vitest.config.ts | 画面の行カバレッジ ≥ 80% | 90.01% | `npm run test:coverage`（手元・CI） | build-and-test | Met |
| T-NFR4 | requirements.md NFR4（261002 NFR1・NFR2） | 速さのテスト 9 件が通る（10 万件 10 秒、100 万件 100 秒） | 9/9 | `cargo test --release -p local-sights-core --lib -- --ignored` | build-and-test | Met |
| T-NFR5 | requirements.md NFR5、project.md Forbidden | 読み取りの 3 つ以外の API を呼ばない。名前はネットワークから取らない | 新しい呼び出しなし | security-test-instructions.md の検索の結果 | build-and-test | Met |
| T-NFR6 | requirements.md NFR6 | 新しい文が英語と日本語の両方にある | 両方の言語がそろっている、というテストが通った | `src/i18n/messages.test.ts` | build-and-test | Met |
| T-NFR7 | requirements.md NFR7、team.md Code Style | 書式・検査・依存関係のチェックが通る。テスト以外で `unwrap`／`expect` を足さない | fmt・clippy（警告 0）・tsc・ESLint・Prettier・npm audit・cargo-deny が成功 | test-results.md「ビルド」、CI の cargo-deny（13bc6a9） | build-and-test | Met |
| T-FR1.3 | requirements.md FR1.3 | 直したテストが 200 回続けて通る | 200/200 | test-results.md「テスト」 | build-and-test | Met |
| T-A2-macos-region | requirements.md A2（FR5） | macOS の GUI でリージョンの選択欄を目で確かめる | 開発者の報告：日本語の画面で問題なし（2026-10-21） | 開発者の Mac での目視（会話での報告） | build-and-test | Met |
| T-A2-macos-plural | requirements.md A2（FR4.1） | macOS の GUI で英語の単数形「1 stream」を目で確かめる | 見ていない（英語に切り替える手間のため） | 自動のテスト（`src/i18n/messages.test.ts`、`StatusLine.test.tsx`）は通過 | build-and-test（次の版を配る前の両 OS の手元の確認で拾う） | Unverified |
| T-A2-macos-cancel | requirements.md A2（FR4.2） | macOS の GUI で中断の知らせと空のときの余白を目で確かめる | 見ていない | 自動のテスト（`CancelNotice.test.tsx`、`App.test.tsx`）は通過。余白は jsdom では確かめられない | build-and-test（次の版を配る前の両 OS の手元の確認で拾う） | Unverified |
| T-FR4.4 | requirements.md FR4.4（Build and Test の目視から足した依頼） | 進み具合がクエリの枠の中、［Run query］の並びの下に出て、結果の枠には出ない | 置き場所のテスト 4 件を含め 31/31 件が通過。CI（ec0aa5b）も成功 | `src/components/LogAnalyticsPage.test.tsx`、run 39606616095 | build-and-test | Met |
| T-A2-macos-fr44 | requirements.md A2（FR4.4） | macOS の GUI で、進み具合の置き場所を目で確かめる | Pending | 開発者の Mac での目視 | build-and-test | Pending |
| T-A2-macos-query | requirements.md A2（FR2） | macOS の GUI でロググループを変えてクエリを実行し直す | 見ていない | 自動のテスト（`query::runner`、`query::store`）は通過 | build-and-test（次の版を配る前の両 OS の手元の確認で拾う） | Unverified |

## 準備の状態

- ビルド：準備できている。
- テスト：自動のテストと CI はすべて通った。macOS の目視（A2）は、リージョンの選択欄だけを行った。英語の単数形、中断の知らせ、クエリの画面の 3 つは見ていない（Unverified）。この 3 つは自動のテストで振る舞いを確かめている。目で見ていないのは見た目だけ。
- 配ること：この作業だけでは配らない（team.md Deployment）。次の機能と合わせて、版を上げる PR → Release dry run → 両 OS での手元の確認 → タグの順で進める。

## 残っていること

- macOS の目視（A2）のうち、英語の単数形、中断の知らせ、クエリの画面の 3 つ。次の版を配る前の両 OS の手元の確認（`docs/releasing.md`）で見る。
- 開発者からの要望：英語の画面を確かめるのに OS の言語を変えるのは手間なので、アプリの中で言語を切り替えられるとよい。この作業の範囲外。次の作業の候補として残す。
- Windows の手元の確認は省く（A1）。Windows は CI のランナーのビルドとテストで確かめた。GUI の見た目（日本語の名前の表示、選択欄の操作）は、次の版を配る前の両 OS での手元の確認で見る。
- Code Generation のレビューの Minor 3 件：
  - R-01：リージョンの欄で、文字を打って閉じたあとの開き直し。macOS の目視で確かめる。
  - R-02：NFR1 の文面と、直したテストの食い違い。この記録の「既存のテストの期待値を直したもの」に書いた。
  - R-03：ライブラリのカバレッジ。96.63% で解消した。

## 既存のテストの期待値を直したもの（レビュー R-02）

NFR1 は「FR2.2.5 と FR4.3 で直すテスト以外は期待値を変えない」としている。実際には、次のテストも、この作業で直す振る舞いそのものを固定していたため直した（中身はレビューで妥当とされた）。

- FR4.1：`src/components/StatusLine.test.tsx`（2 か所）、`src/components/LogAnalyticsPage.test.tsx`（1 か所）。「1 streams」を期待していた。
- FR4.2：`src/App.test.tsx`。知らせが消えると囲みもなくなる、を期待していた。
- FR2.3：`src-tauri/src/lib.rs`（`holdingsEpoch` が送り返される）、`src/components/QueryResultTable.test.tsx`（別の世代の応答を捨てる）。
