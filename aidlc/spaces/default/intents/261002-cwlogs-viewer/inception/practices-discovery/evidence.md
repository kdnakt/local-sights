# Evidence

## 調べたもの・推論したこと（参加者別）

### リード（aidlc-pipeline-deploy-agent）

| 対象 | 内容 |
|------|------|
| `aidlc/spaces/default/memory/org.md` | 5 セクションの既定（トランクベース・`main` へのスクワッシュマージ、`skeleton` はスコープ依存、テスト既定 `test-after`、マージでステージング・本番は手動承認、言語標準のフォーマッタ／リンタで lint 失敗は PR をブロック）を基準として読んだ。 |
| `aidlc/spaces/default/memory/team.md` | 5 セクションとも空。再実行の基準となる確定済み内容はない。 |
| `aidlc/spaces/default/memory/project.md` | `## Corrections` の学習済みルール 4 件（Rust 製・GetLogEvents API は利用者指定の前提、質問は一問ずつ、取得中の最低限の状態表示、GUI の見た目は標準部品のみ）。 |
| `ideation/approval-handoff/initiative-brief.md`、`decision-log.md` | 個人開発・OSS、macOS のみ、ローカル実行、MVP 目標 2026-10-10。D-03（AWS デプロイなし）、D-07（ディスク保存は利用者が選んだ場合のみ）、D-13（対象外の API・操作）、D-16（MVP は macOS のみ）。 |
| `aidlc-state.md`、`.claude/scopes/aidlc-local-tool.md` | Greenfield、スコープ `local-tool`（`skeleton` 指定なし、org.md のスコープ別テスト下限の一覧にもない）、Test Strategy `Standard`。 |
| git | アプリのコード・CI 設定・フォーマッタ／リンタ設定はまだない。既定ブランチは `main`。 |

推論：開発者 1 人のため PR はセルフレビューになる／サーバーがないため Deployment は「配布」に読み替える／Rust のため `rustfmt`・`clippy` が自然な既定。

### aidlc-quality-agent（`contributions/aidlc-quality-agent.md`）

- リードの Testing Posture を読み、`local-tool` にカバレッジ下限の既定がないこと、下限は一度決めると Build and Test で下げられないことを指摘。
- GUI に依存しない層（時刻順マージ、ページ終端判定、再試行、タイムゾーン変換など）をテストの中心にし、AWS はモック／スタブに差し替える型を提案。
- 薄い一本の確認は、自動で確かめる部分（`cargo test`）と手で確かめる部分（実 AWS・GUI の目視）に分けることを提案（Q3 の選択肢の元）。

### aidlc-developer-agent（`contributions/aidlc-developer-agent.md`）

- 層の分離（ドメイン／AWS アダプター／GUI）を慣行として明記すること、lib／bin 分離、AWS 呼び出しを trait の裏に置くことを提案（Q8 の元）。
- Rust のエラー処理（`Result` で返す、テスト以外で `unwrap()` / `expect()` を使わない）、命名を AWS の用語にそろえること、`Cargo.lock` のコミットを提案。
- GUI フレームワーク次第（例：Web 技術ベース）で Code Style の見直しが要ることを未解決事項として指摘。

### aidlc-devsecops-agent（`contributions/aidlc-devsecops-agent.md`）

- `main` への直接 push はフォーマット・lint・テスト・依存チェックを素通りさせると指摘（Q1・F2 の背景）。
- 認証情報をリポジトリ・ログ・CI に置かないこと、`Cargo.lock` コミットと `cargo-deny` 等の依存チェック、GitHub の秘密情報スキャンを提案（Q9 の元）。
- 署名・公証なしで配ると利用者の Mac で警告が出ることを指摘（Q6 の背景）。
- ハードな制約の候補（認証情報、読み取り API のみ、CI で実 AWS に接続しない、テレメトリ送信なし）を挙げ、Q10 の選択肢の元にした。

## インタビューでの決定

| 質問 | 決定 |
|------|------|
| Q1 / F2 | 当初の回答は C（`main` に直接 push、PR なし）。org.md の既定（短命ブランチ → `main` へスクワッシュマージ、CI の lint 失敗はマージ不可）と矛盾するため F2 で確認し、A（短命ブランチ → PR → CI が通ったらスクワッシュマージ、直接 push なし）に変更。 |
| Q2 | 薄い一本を最初の作業単位で作る（A）。 |
| Q3 | `cargo test` ＋ 実 AWS から取得結果を表示する確認用プログラムを手元で実行 ＋ GUI を起動して目視（A）。 |
| Q4 | custom：純粋なロジックはテスト先行、AWS 接続と GUI は実装後にテスト（B）。 |
| Q5 | 数値のカバレッジ下限なし。GUI に依存しない層の公開関数には必ずテスト（A）。 |
| Q6 | MVP はソースからのビルドのみ（`cargo build` / `cargo install`）。Releases は後で（B）。 |
| Q7 | `cargo fmt --check` は必須、clippy の警告はエラーにしない（B）。 |
| Q8 | lib と GUI アプリを分け、AWS 呼び出しは trait の裏。テスト以外で `unwrap()` / `expect()` 不使用、エラーは呼び出し元へ返す（A）。 |
| Q9 | `Cargo.lock` コミット、CI で `cargo-deny`、GitHub の秘密情報スキャン有効化（A）。 |
| Q10 / F1 | B・C・D をハードな制約として採用。A は人間の言葉で範囲を修正：「AWS の認証情報がどこまでかわからないけど、リポジトリには置かない、ロール名とかであればログやエラー表示に残すのはあり」。F1 で「アクセスキーIDはログ出力NG。他の識別子は良さそう。」と回答。これにより、秘密の認証情報とアクセスキー ID はリポジトリ・ログ・エラー表示・キャッシュに書かない、プロファイル名・ロール名（ARN）・アカウント ID はログやエラー表示に出してよい（リポジトリには置かない）と記録。 |
| まとめの確認 | 「Looks correct」。 |

## 未解決の不確かさ

- **GUI フレームワーク**：未選定（後の設計段階で決める）。Web 技術ベースのもの（例：Tauri）を選んだ場合、JS/TS 側のフォーマッタ／リンタが必要になり、Code Style を見直す必要がある。非同期の AWS SDK と GUI のイベントループの橋渡しも未決。
- **Q1 の経緯**：人間の第一希望は `main` への直接 push だった。org.md に合わせて PR 経由に変更したが、個人開発での手間が実際に負担になれば、org 側の方針との兼ね合いを含めて見直しの話題になりうる。
- **clippy の扱い**：警告はエラーにしないが、deny レベルの lint（エラー）は PR をブロックする、と解釈して記録した。どの lint を deny にするか（例：`clippy::unwrap_used` で Q8 を機械的に検査するか）は未決。
- **Walking Skeleton とスコープ**：スコープ `local-tool` は `skeleton` を宣言していない。チームとしては薄い一本の採用を決めたが、骨組みのチェックポイントの儀式がエンジン上でどう扱われるかは Construction の開始時に確認が要る。
- **Q3 の確認用プログラム**：実 AWS への接続は開発者本人の手元・本人の認証情報で行う前提。具体的なコマンド（Construction Verification Command）は Construction で記録する。
- **配布の後続**：GitHub Releases、タグ規約（semver 等）、macOS の署名・公証は MVP 後に決める。
- **単一パッケージかワークスペースか**：lib／bin 分離までは決定。ワークスペースにするかは設計段階で決める。

まとめの確認（Consolidated Summary Confirmation）は「Looks correct」で記録済み。
