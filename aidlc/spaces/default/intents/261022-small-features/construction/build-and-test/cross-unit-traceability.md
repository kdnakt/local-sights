# Cross-Unit Traceability — アプリの中での言語切替・書き出し・JSON の整形（261022-small-features）

対象：`inception/requirements-analysis/requirements.md` の FR と NFR（62 件）。user-stories の段はこの計画にないため AC はない。読んだ traceability：`construction/u1-language-switch/code-generation/traceability.json`（15 件）、`construction/u2-export-and-json/code-generation/traceability.json`（51 件）。stage-level の `construction/code-generation/traceability.json` はない。

## 判定

**FAIL（厳密な判定）**：62 件のうち 39 件は少なくとも 1 つの作業単位の code-generation の traceability に `OK` で載り、対象ファイルはすべて存在する。残る 23 件はどの作業単位の code-generation の traceability にも ID として載っていない。ただし中身の上では、どれも親または子の ID、あるいはこの段の実行結果で確かめられていて、実装やテストの欠けを示すものはない（下の「載っていない ID」）。この結果は承認ゲートで知らせる。

## 載っていない ID（23 件）

- **FR1**：見出し。FR1.1〜FR1.6（U1）がすべて OK
- **FR4.1.1**：3 段の項目。親 FR4.1（U2）と BR4.x が OK
- **FR4.1.2**：3 段の項目。親 FR4.1（U2）と BR4.x が OK
- **FR4.1.3**：3 段の項目。親 FR4.1（U2）と BR4.x が OK
- **FR4.1.4**：3 段の項目。親 FR4.1（U2）と BR4.x が OK
- **FR4.2.1**：3 段の項目。親 FR4.2（U2）と BR4.x が OK
- **FR4.2.2**：3 段の項目。親 FR4.2（U2）と BR4.x が OK
- **FR4.2.3**：3 段の項目。親 FR4.2（U2）と BR4.x が OK
- **FR4.2.4**：3 段の項目。親 FR4.2（U2）と BR4.x が OK
- **FR4.3.1**：3 段の項目。親 FR4.3（U2）と BR4.x が OK
- **FR4.3.2**：3 段の項目。親 FR4.3（U2）と BR4.x が OK
- **FR4.3.3**：3 段の項目。親 FR4.3（U2）と BR4.x が OK
- **FR4.4.1**：3 段の項目。親 FR4.4（U2）と BR4.x が OK
- **FR4.4.2**：3 段の項目。親 FR4.4（U2）と BR4.x が OK
- **FR4.4.3**：3 段の項目。親 FR4.4（U2）と BR4.x が OK
- **FR4.4.4**：3 段の項目。親 FR4.4（U2）と BR4.x が OK
- **FR4.5.1**：3 段の項目。親 FR4.5（U2）と BR4.x が OK
- **FR4.5.2**：3 段の項目。親 FR4.5（U2）と BR4.x が OK
- **FR4.6.1**：3 段の項目。親 FR4.6（U2）と BR4.x が OK
- **FR4.6.2**：3 段の項目。親 FR4.6（U2）と BR4.x が OK
- **FR4.6.3**：3 段の項目。親 FR4.6（U2）と BR4.x が OK
- **NFR1**：既存のテストの維持。build-and-test の T-RUST-TEST・T-VITEST で確認（865 + 549 本、失敗 0）
- **NFR4**：テストの順序とカバレッジ 80%。build-and-test の T-ORDER・T-COV-CORE・T-COV-FE で確認

内訳：FR1 は見出しの ID で、U1 は子の FR1.1〜FR1.6 だけを載せた。FR4.1.1〜FR4.6.3 の 20 件は要件が 3 段で書いた項目で、U2 の functional-design と code-generation の traceability は親の FR4.1〜FR4.6 と、それを具体化した BR4.1〜BR4.9 を載せた（`rules.md` の各ルールの source に FR4.x.y が書かれている）。NFR1 と NFR4 は「既存のテストが緑のまま」「テストの順序とカバレッジ」という、作業単位のコードではなく Build and Test の実行で確かめる要件である。

## ID ごとの対応

| ID | 状態 | 持ち主（stage / Unit） | 対象ファイル | ファイルの存在 | 備考 |
|---|---|---|---|---|---|
| FR1 | **code-generation の traceability にない** | U1（FR1.1〜FR1.6 経由） | — | — | 見出し。FR1.1〜FR1.6（U1）がすべて OK |
| FR1.1 | OK | code-generation（U1） | `src/components/SettingsDialog.tsx` | あり | 直接 |
| FR1.2 | OK | code-generation（U1） | `src/App.tsx` | あり | 直接 |
| FR1.3 | OK | code-generation（U1） | `src/App.test.tsx` | あり | 直接 |
| FR1.4 | OK | code-generation（U1） | `crates/local-sights-core/src/cache/settings.rs` | あり | 直接 |
| FR1.5 | OK | code-generation（U1） | `src/App.test.tsx` | あり | 直接 |
| FR1.6 | OK | code-generation（U1） | `crates/local-sights-core/src/session.rs` | あり | 直接 |
| FR2 | OK | code-generation（U2） | `crates/local-sights-core/src/session.rs` | あり | 直接 |
| FR2.1 | OK | code-generation（U2） | `src-tauri/src/lib.rs` | あり | 直接 |
| FR2.2 | OK | code-generation（U2） | `crates/local-sights-core/src/export/exporter.rs` | あり | 直接 |
| FR2.3 | OK | code-generation（U2） | `crates/local-sights-core/src/export/exporter.rs` | あり | 直接 |
| FR2.4 | OK | code-generation（U2） | `crates/local-sights-core/src/export/formatter.rs` | あり | 直接 |
| FR2.5 | OK | code-generation（U2） | `crates/local-sights-core/src/session.rs` | あり | 直接 |
| FR2.6 | OK | code-generation（U2） | `crates/local-sights-core/src/session.rs` | あり | 直接 |
| FR3 | OK | code-generation（U2） | `crates/local-sights-core/src/insights_session.rs` | あり | 直接 |
| FR3.1 | OK | code-generation（U2） | `crates/local-sights-core/src/insights_session.rs` | あり | 直接 |
| FR3.2 | OK | code-generation（U2） | `crates/local-sights-core/src/insights_session.rs` | あり | 直接 |
| FR3.3 | OK | code-generation（U2） | `crates/local-sights-core/src/export/exporter.rs` | あり | 直接 |
| FR3.4 | OK | code-generation（U2） | `crates/local-sights-core/src/insights_session.rs` | あり | 直接 |
| FR3.5 | OK | code-generation（U2） | `crates/local-sights-core/src/insights_session.rs` | あり | 直接 |
| FR4 | OK | code-generation（U2） | `src-tauri/src/lib.rs` | あり | 直接 |
| FR4.1 | OK | code-generation（U2） | `src/components/ExportMenu.tsx` | あり | 直接 |
| FR4.1.1 | **code-generation の traceability にない** | U2（FR4.1 経由） | — | — | 3 段の項目。親 FR4.1（U2）と BR4.x が OK |
| FR4.1.2 | **code-generation の traceability にない** | U2（FR4.1 経由） | — | — | 3 段の項目。親 FR4.1（U2）と BR4.x が OK |
| FR4.1.3 | **code-generation の traceability にない** | U2（FR4.1 経由） | — | — | 3 段の項目。親 FR4.1（U2）と BR4.x が OK |
| FR4.1.4 | **code-generation の traceability にない** | U2（FR4.1 経由） | — | — | 3 段の項目。親 FR4.1（U2）と BR4.x が OK |
| FR4.2 | OK | code-generation（U2） | `crates/local-sights-core/src/export/formatter.rs` | あり | 直接 |
| FR4.2.1 | **code-generation の traceability にない** | U2（FR4.2 経由） | — | — | 3 段の項目。親 FR4.2（U2）と BR4.x が OK |
| FR4.2.2 | **code-generation の traceability にない** | U2（FR4.2 経由） | — | — | 3 段の項目。親 FR4.2（U2）と BR4.x が OK |
| FR4.2.3 | **code-generation の traceability にない** | U2（FR4.2 経由） | — | — | 3 段の項目。親 FR4.2（U2）と BR4.x が OK |
| FR4.2.4 | **code-generation の traceability にない** | U2（FR4.2 経由） | — | — | 3 段の項目。親 FR4.2（U2）と BR4.x が OK |
| FR4.3 | OK | code-generation（U2） | `crates/local-sights-core/src/export/formatter.rs` | あり | 直接 |
| FR4.3.1 | **code-generation の traceability にない** | U2（FR4.3 経由） | — | — | 3 段の項目。親 FR4.3（U2）と BR4.x が OK |
| FR4.3.2 | **code-generation の traceability にない** | U2（FR4.3 経由） | — | — | 3 段の項目。親 FR4.3（U2）と BR4.x が OK |
| FR4.3.3 | **code-generation の traceability にない** | U2（FR4.3 経由） | — | — | 3 段の項目。親 FR4.3（U2）と BR4.x が OK |
| FR4.4 | OK | code-generation（U2） | `src-tauri/src/export_boundary.rs` | あり | 直接 |
| FR4.4.1 | **code-generation の traceability にない** | U2（FR4.4 経由） | — | — | 3 段の項目。親 FR4.4（U2）と BR4.x が OK |
| FR4.4.2 | **code-generation の traceability にない** | U2（FR4.4 経由） | — | — | 3 段の項目。親 FR4.4（U2）と BR4.x が OK |
| FR4.4.3 | **code-generation の traceability にない** | U2（FR4.4 経由） | — | — | 3 段の項目。親 FR4.4（U2）と BR4.x が OK |
| FR4.4.4 | **code-generation の traceability にない** | U2（FR4.4 経由） | — | — | 3 段の項目。親 FR4.4（U2）と BR4.x が OK |
| FR4.5 | OK | code-generation（U2） | `src-tauri/src/export_boundary.rs` | あり | 直接 |
| FR4.5.1 | **code-generation の traceability にない** | U2（FR4.5 経由） | — | — | 3 段の項目。親 FR4.5（U2）と BR4.x が OK |
| FR4.5.2 | **code-generation の traceability にない** | U2（FR4.5 経由） | — | — | 3 段の項目。親 FR4.5（U2）と BR4.x が OK |
| FR4.6 | OK | code-generation（U2） | `crates/local-sights-core/src/session.rs` | あり | 直接 |
| FR4.6.1 | **code-generation の traceability にない** | U2（FR4.6 経由） | — | — | 3 段の項目。親 FR4.6（U2）と BR4.x が OK |
| FR4.6.2 | **code-generation の traceability にない** | U2（FR4.6 経由） | — | — | 3 段の項目。親 FR4.6（U2）と BR4.x が OK |
| FR4.6.3 | **code-generation の traceability にない** | U2（FR4.6 経由） | — | — | 3 段の項目。親 FR4.6（U2）と BR4.x が OK |
| FR5 | OK | code-generation（U2） | `src/components/ExpandedMessage.tsx` | あり | 直接 |
| FR5.1 | OK | code-generation（U2） | `src/jsonPretty.ts` | あり | 直接 |
| FR5.2 | OK | code-generation（U2） | `src/jsonPretty.ts` | あり | 直接 |
| FR5.3 | OK | code-generation（U2） | `src/jsonPretty.test.ts` | あり | 直接 |
| FR5.4 | OK | code-generation（U2） | `src/jsonPretty.test.ts` | あり | 直接 |
| FR5.5 | OK | code-generation（U2） | `src/components/ExpandedMessage.tsx` | あり | 直接 |
| FR5.6 | OK | code-generation（U2） | `src/components/ExpandedMessage.test.tsx` | あり | 直接 |
| FR5.7 | OK | code-generation（U2） | `src/components/ExpandedMessage.test.tsx` | あり | 直接 |
| FR5.8 | OK | code-generation（U2） | `src/components/ExpandedMessage.tsx` | あり | 直接 |
| NFR1 | **code-generation の traceability にない** | build-and-test | — | — | 既存のテストの維持。build-and-test の T-RUST-TEST・T-VITEST で確認（865 + 549 本、失敗 0） |
| NFR2 | OK | code-generation（U2） | `src-tauri/src/lib.rs` | あり | 直接 |
| NFR3 | OK | code-generation（U2） | `src-tauri/src/export_boundary.rs` | あり | 直接 |
| NFR4 | **code-generation の traceability にない** | build-and-test | — | — | テストの順序とカバレッジ 80%。build-and-test の T-ORDER・T-COV-CORE・T-COV-FE で確認 |
| NFR5 | OK | code-generation（U1、U2） | `crates/local-sights-core/src/cache/settings.rs`<br>`src-tauri/src/export_boundary.rs` | あり | 直接 |
| NFR6 | OK | code-generation（U1、U2） | `src/components/SettingsDialog.tsx`<br>`src/components/ExpandedMessage.tsx` | あり | 直接 |

## 作業単位の分担

- U1 `u1-language-switch`：FR1.1〜FR1.6、BR1.1〜BR1.7、NFR5、NFR6。
- U2 `u2-export-and-json`：FR2〜FR5 の全項目（2 段まで）、BR2.1〜BR5.5、NFR2、NFR3、NFR5、NFR6。
- Build and Test：NFR1、NFR4（実行結果）。NFR3 の両 OS の確認は Target Verification Matrix の T-OS-CI・T-OS-MAC・T-OS-WIN で `Unverified`（`build-and-test-summary.md`）。
