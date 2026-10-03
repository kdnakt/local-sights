**Collaborator:** aidlc-developer-agent

## Contribution

開発者の立場（命名・層の境界・Rust のエラー処理・ファイル／モジュール構成・コードスタイル）から、リードのドラフト（`team-practices.md`、`discovered-rules.md`、`evidence.md`）を読んだ。根拠は `aidlc/spaces/default/memory/org.md`、`aidlc/spaces/default/memory/project.md`、`aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/approval-handoff/initiative-brief.md`。アプリのコードはまだないため、以下はすべて **提案** であり、インタビューで確認するまでは確定した慣行ではない。GUI フレームワークは選ばない（後の段階で決める）。

### 1. 層の境界（ドラフトで最も手薄な点）

リードの Testing Posture は「テストの中心は GUI に依存しない層」と書いているが、その層を分けること自体はどのセクションにも慣行として書かれていない。テスト方針が層の分離に依存しているので、分離そのものを Way of Working か Code Style に明記するのがよい。提案する境界は次の 3 つ。

| 層 | 役割 | 依存してよいもの |
|----|------|------------------|
| ドメイン／取得ロジック | 時刻順マージ、ページ終端判定、スロットリング時の待機・再試行の方針、タイムゾーン変換、部分一致フィルター、キャッシュの読み書き | 標準ライブラリと小さな汎用クレートのみ。GUI と AWS SDK には依存しない |
| AWS アクセス（アダプター） | プロファイル／リージョンの解決、ロググループ一覧、GetLogEvents の呼び出し | AWS SDK。ドメイン層が定めたトレイト（インターフェース）を実装する |
| GUI | 画面表示、入力、取得中の操作ロック、終了確認、状態表示 | ドメイン層のみ。AWS SDK を直接呼ばない |

- ねらい：(a) AWS 呼び出しをトレイト越しのモック／スタブに差し替えて、ページ終端判定や再試行をネットワークなしでテストできる。(b) 後で Tauri などに乗り換えたり Windows 対応（IB-16）を足したりしても、GUI 層だけを差し替えればよい。
- 時刻と待機（`sleep`）も差し替え可能にしておくと、再試行のテストが速く決定的になる（提案）。
- 非同期ランタイムの扱いは未決。AWS SDK for Rust は非同期で、GUI のイベントループとの橋渡し方は GUI フレームワークの選定と一緒に決める必要がある。ここでは「ドメイン層は特定の GUI のスレッドモデルを前提にしない」とだけ提案する。

### 2. ファイル／モジュール構成（単一クレートかワークスペースか）

期限が約 1 週間のため、どちらでも成立する。インタビューで決める。

- 案 A：単一パッケージで `src/lib.rs`（ドメイン層・AWS アダプター）と `src/main.rs`（GUI）に分ける。最小の手間で、`lib` 側を `cargo test` でテストできる。
- 案 B：Cargo ワークスペース（例：`crates/core`、`crates/aws`、`crates/app`）。層の境界をクレートの依存関係でコンパイラに強制でき、Tauri 導入時に `core` を再利用しやすい。反面、初期設定がやや増える。
- 開発者として推奨するのは「少なくとも案 A の lib／bin 分離は必須、ワークスペースは任意」。どちらでも、単体テストは各モジュール内の `#[cfg(test)]`、結合テストは `tests/` に置く。
- バイナリアプリなので `Cargo.lock` はコミットする（提案）。

### 3. Rust のエラー処理の慣行

`aidlc/spaces/default/memory/phases/construction.md` の「失敗を黙って握りつぶさない」「回復可能な失敗と致命的な失敗を区別する」を、Rust で具体化する提案。

- 失敗しうる処理は `Result<T, E>` で返す。ドメイン層・AWS アダプターでは用途ごとのエラー列挙型を定義する（例：`thiserror` を使う）。`anyhow` 等の型消去エラーを使うならアプリ（GUI）側の境界に限る。
- テスト以外のコードでは `unwrap()` / `expect()` / `panic!` を使わない（理由をコメントで書いた不変条件を除く）。`clippy::unwrap_used` などで機械的に検査する案もある。
- AWS のエラーは分類する：スロットリング（待って再試行）と、認証切れ・権限不足・ロググループなし・入力の誤り（再試行せず利用者に知らせる）。分類はアダプター層で行い、GUI はステータス行やエラー画面（wireframes の「該当なし・エラー・入力の誤り」）に表示するだけにする。
- エラーメッセージやログ出力に認証情報や取得したログ本文を含めない（詳細はセキュリティ担当の範囲。ここでは慣行の候補として挙げるのみ）。

### 4. 命名規則

リードの「Rust の慣用に従う」に同意する。補足として次を提案する。

- Rust API Guidelines に従う（例：getter に `get_` を付けない、変換は `as_` / `to_` / `into_`）。
- ドメイン用語を AWS の用語にそろえ、コード全体で 1 つに統一する：`log_group` / `LogGroup`、`log_stream` / `LogStream`、`LogEvent`、`next_forward_token` など。似た語（`stream` と `channel` 等）の混在を避ける。
- クレート名・パッケージ名はケバブケース（コード上はスネークケース）。

### 5. コードスタイル（rustfmt / clippy / edition）

リードの `rustfmt` + `clippy` に同意したうえで、ドラフトに無い項目を挙げる。

- edition：2024 を提案（Rust 1.85 以降が必要）。使うツールチェーンは `rust-toolchain.toml` で固定するか、`Cargo.toml` の `rust-version` で最低版を宣言するかを決める。
- `rustfmt`：既定設定のままとし、`rustfmt.toml` は必要になるまで置かない（提案）。
- `clippy`：CI では `cargo clippy --all-targets -- -D warnings` で警告を失敗扱いにする案。`clippy::pedantic` は MVP では有効にしない（期限優先）。
- lint の設定場所は `Cargo.toml` の `[lints]`（ワークスペースなら `[workspace.lints]`）に一本化する。`unsafe_code = "forbid"` を入れる案もある。
- 依存の脆弱性検査（`cargo audit` / `cargo deny`）は CI ステージとセキュリティ担当の判断に委ねる。
- 注意：GUI に Tauri のような Web 技術ベースの仕組みを選んだ場合、フロントエンド側に JS/TS のコードが生じ、`org.md` の Code Style に従って Prettier / ESLint の設定も必要になる。GUI 選定後に Code Style を見直す必要があることを、未解決事項として残すべき。

### 6. `discovered-rules.md` への確認候補の追加（ルールではない）

リードの確認候補に、次の 2 点も加えてインタビューで尋ねることを提案する。人間が明言しない限り ALWAYS / NEVER に昇格させない。

- 候補：GUI 層から AWS SDK を直接呼ばない（AWS アクセスはアダプター層に閉じる）
- 候補：テスト以外のコードで `unwrap()` / `expect()` を使わない

### 7. インタビューで決めるべき未解決事項（開発者視点）

1. 層の分離（ドメイン／AWS アダプター／GUI）を慣行として明記するか。
2. 単一パッケージ（lib／bin 分離）とワークスペースのどちらにするか。
3. edition（2024 か）と、ツールチェーン固定の方法（`rust-toolchain.toml` か `rust-version` か）。
4. `clippy` の警告をエラー扱いにするか。`unwrap` 禁止を lint で強制するか。
5. エラー型の方針（`thiserror` で型付きエラー、`anyhow` は GUI 境界のみ、で良いか）。
6. AWS のモック方法（自前トレイト + スタブか、SDK のテスト支援機能を使うか）。
7. GUI フレームワークと非同期ランタイムの組み合わせ（本段階では決めない。選定時に Code Style・層の境界を見直す）。

## Positions

- AGREE: Way of Working のトランクベース＋スクワッシュマージ提案 — 1 人開発でも `main` を常に動く状態に保てて、Bolt と履歴が 1 対 1 になる。
- AGREE: Testing Posture の `test-after` 既定と、GUI 非依存の層を中心にテストする方針 — 期限が短く、ロジックの多くは GUI 外にあるため。
- OBJECT: Testing Posture が「GUI に依存しない層」を前提にしているのに、層の分離自体がどのセクションにも慣行として書かれていない — 前提を Way of Working か Code Style に明記しないと、テスト方針の根拠が宙に浮く。
- AGREE: Code Style の `rustfmt` / `clippy` と Rust 慣用の命名 — 言語標準で、`org.md` の「言語既定のツールに従う」と一致する。
- OBJECT: Code Style に edition・lint 設定の置き場所・`unwrap` / エラー処理の方針が無い — Rust ではこれらが日々のコードの形を大きく左右するため、インタビューの論点に加えるべき。
- AGREE: Walking Skeleton の「GUI から GetLogEvents を呼んで 1 画面に表示する」薄い一本 — GUI・AWS SDK・非同期の結線という最大の技術的不安を最初に潰せる。
- AGREE: `discovered-rules.md` を現時点で `None.` とし、候補をコメントに留める扱い — 人間が明言していない制約をルールにしないのは正しい。
- OBJECT: `evidence.md` の未解決事項に「GUI 選定後に Code Style（JS/TS が入る場合の Prettier / ESLint 等）を見直す」が無い — Tauri を将来の選択肢として挙げている以上、見直しの契機を残しておくべき。
