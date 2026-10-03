# Evidence（ドラフト）

> **ステータス: ドラフト（リード下書き時点）**。サポートエージェントのレビューとインタビュー結果は最終統合で追記する。

## 調べたもの

| 対象 | 内容 |
|------|------|
| `aidlc/spaces/default/memory/org.md` | 5 セクション（Way of Working / Walking Skeleton / Testing Posture / Deployment / Code Style）を既定値として読んだ。トランクベース・`main` へのスクワッシュマージ、`skeleton` はスコープ依存、テストの既定は `test-after`、デプロイはマージでステージング・本番は手動承認、スタイルは言語標準ツールに従う。 |
| `aidlc/spaces/default/memory/team.md` | 5 セクションとも空。再実行の基準となる確定済み内容はない。 |
| `aidlc/spaces/default/memory/project.md` | `## Corrections` に学習済みルール 4 件（Rust 製・GetLogEvents API は利用者指定の前提、質問は一問ずつ、取得中の最低限の状態表示、GUI の見た目は標準部品のみで期限優先）。その他のセクションは空。 |
| `aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/approval-handoff/initiative-brief.md` | 個人開発・OSS、macOS のみの Rust 製デスクトップ GUI、ローカル実行、MVP 目標 2026-10-10、作る順番 IB-00 → IB-06、技術選定は開発ルール確認以降。 |
| `aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/approval-handoff/decision-log.md` | D-02（利用者・決定者は開発者本人）、D-03（AWS デプロイなし）、D-07（ディスク保存は利用者が選んだ場合のみ）、D-13（対象外の API・操作）、D-16（MVP は macOS のみ）。 |
| `aidlc/spaces/default/intents/261002-cwlogs-viewer/aidlc-state.md` | Greenfield、スコープ `local-tool`、Depth `Standard`、Test Strategy `Standard`、Construction Checkpoints `enabled`、Construction Iteration `unit-major`。 |
| `.claude/scopes/aidlc-local-tool.md` | `skeleton` の指定なし。 |
| git（ブランチ `claude/festive-goodall-jdkya4`、既定ブランチ `main`、HEAD `4876bb9`） | アプリのコード・CI 設定・フォーマッタ／リンタ設定はまだない。コミット履歴は AI-DLC の成果物のみ。 |

## 推論したこと

- 開発者が 1 人のため、org.md のトランクベース・スクワッシュマージはそのまま適用しやすいが、PR レビューは実質セルフレビューになる。
- サーバーへのデプロイがないため、org.md の Deployment 既定（ステージング・本番）は文字どおりには当てはまらず、「GitHub 上での配布・リリース」に読み替える必要がある。
- 言語が Rust（利用者指定の前提）のため、Code Style は `rustfmt` / `clippy` が自然な既定になる。
- スコープ `local-tool` は org.md のスコープ別テスト下限に載っていないため、既定の Methodology（`test-after`）以外の下限（カバレッジ等）は自動では決まらない。
- 期限が約 1 週間と短いため、結線の不安を先に潰す「薄い一本」は有効だが、儀式の重さとの兼ね合いがある。

## インタビューで決める未解決事項

1. **ブランチ運用**：トランクベース＋スクワッシュマージでよいか。PR を必ず経由するか、個人開発として `main` への直接 push を許すか。
2. **薄い一本を先に作るか**：採用するか否か。採用する場合、最小の一本の範囲と、それを確認するコマンド（実 AWS アカウントでの表示確認など）。
3. **テストの進め方**：`test-after` でよいか、TDD など別のやり方にするか。カバレッジ下限を設けるか（設けるなら何 %）。GUI 層をテスト対象に含めるか。
4. **配布方法**：GitHub Releases でビルド成果物を公開するか、ソースからのビルドのみとするか。タグ規約（semver など）。macOS の署名・公証を MVP で行うか。
5. **コードスタイル**：`rustfmt` / `clippy` の採用、`clippy` 警告をエラー扱いにするか。
6. **ハードな制約**：`discovered-rules.md` の確認候補（認証情報を書き込まない、ディスク保存は選択時のみ、変更系 API を呼ばない 等）を、必ず守るルールとして残すか。
