## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-24T07:11:31Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261022-small-features/inception/domain-design/components.md > LogExporter / AppSession / InsightsSession / ExportBoundary（behaviour と depends_on） | 書き出しの写し（ExportSnapshot）と状態（ExportJob）の持ち主と流れが部品間で食い違っている。(1) 写しの確定は LogExporter の責任（「写しを取り」）とも AppSession / InsightsSession の責任（「写しの作成」）とも書かれ、持ち主は LogExporter だが作るのは画面ごとのセッションである。(2) ExportJob の状態は LogExporter が持つとあるが、更新して画面に押すのは ExportBoundary が AppSession を通じて行い、ExportBoundary の depends_on は AppSession だけである。クエリの画面の実行可否を判断する InsightsSession（「書き出し中なら無効」）は、この状態を読む依存を宣言していない（コードでは InsightsSession は AppSession の持ち物なので、状態の通り道が別に要る）。(3) 実行中の ExportJob が画面ごとに 1 つか、アプリ全体で 1 つかが書かれておらず、FR4.6.2 の二重実行の防止の範囲（ログの画面の書き出し中にクエリの画面で始められるか）が決まらない。 | 写しの確定の持ち主を 1 つに決めて責任の文を直す（例：各セッションが行の列を渡し、LogExporter が写しを確定する）。ExportJob の実体をどこに持つか（例：AppSession の 1 フィールド）と、InsightsSession がそこから「書き出し中」を読む依存の向きを components.md に足す。ExportJob を全体で 1 つにするか画面ごとにするかを書く。 | New |
| R-02 | Minor | aidlc/spaces/default/intents/261022-small-features/inception/domain-design/components.md > AppSettings behaviour | 「項目がない・知らない値・ファイルがない・読めない・形式が違うときは、キャッシュは今と同じく無効、言語は OS に合わせる」と 1 文にまとめているため、知らない `language` の値だけが入ったファイルでキャッシュの設定まで無効に落ちるように読める。FR1.4 は言語だけを既定にしてキャッシュの設定はそのまま読むことを求めている。今の `load_settings` はパースの失敗を丸ごと Invalid にするため、`language` を列挙型で厳密に読むと実際にそうなる。 | 言語の値の読みはキャッシュの項目と独立にし、知らない値・欠けた値でも `cacheEnabled` は生かすと書く。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261022-small-features/inception/domain-design/components.md > LogExporter / ExportBoundary（ファイル保存の順序） | 写しを取る時点（ADR-004：要求を受けた時点）と、保存ダイアログを開く時点の前後が書かれていない。既定のファイル名はロググループ名を使うため、写しかそれと同じ情報がダイアログより前に要る。ダイアログを開いている間の状態（Running か）と、取り消したときに Idle に戻して写しを捨てることも未記載である。 | 「写し → 既定名 → ダイアログ → 書き込み」の順と、ダイアログ中の状態、取り消し時の写しの破棄を ExportBoundary か LogExporter の behaviour に 1 文で足す。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261022-small-features/inception/domain-design/components.md > AppSession の entities（LanguageSetting）と AppSettings の entities（AppSettingsFile） | `language` が AppSettingsFile の属性と LanguageSetting の属性の両方に載り、二重に持ち主がいるように見える。LanguageSetting は永続化されたものではなく SessionView に載せる実行時の状態である。 | LanguageSetting を AppSettingsFile の実行時の写しと明記するか、独立した実体として載せるのをやめて AppSession の責任の文だけにする。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| 手作業の整合性確認（yaml カタログ） | 部品名は一意、depends_on と dependents は対称、循環なし、実体の持ち主は 1 つずつ、参照の宣言あり | 構造上の問題はない。上の Major は構造ではなく責任と状態の流れの曖昧さである |
| コードとの突き合わせ | `cache/settings.rs` は `SettingsFile`（formatVersion、cacheEnabled）で、パースの失敗は Invalid にまとめて返る。`InsightsSession` は `AppSession` の持ち物で、`SessionView` の `insights` として 1 つのイベントで届く。`main.tsx` は起動時に OS の言語で描く | 設計の前提（設定の拡張、session-changed への相乗り、最初の描画は OS の言語）は現状のコードと合っている |

### Summary

部品の境界、依存の向き、要件（FR1〜FR5）へのたどりはおおむね成立しており、構造の欠陥はない。承認前に見てほしいのは、書き出しの写しと状態の持ち主と更新の経路（とくにクエリの画面側の読み取りと、全体で 1 つか画面ごとかの範囲）が部品間で食い違っている点で、機能設計に入る前に components.md の文を直すと実装で迷わない。
