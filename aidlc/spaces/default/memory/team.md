# Team-Level Rules

> This team's affirmed practices and corrections. Loaded after `org.md` as
> strict-additive guidance; contradictions with broader policy are rejected.
> Populated by the practices-discovery affirmation gate. Edit at the gate,
> not directly.

## Way of Working

私たちはトランクベース開発で進める。作業は短命のブランチ（目安 1〜2 日以内）で行い、PR を作り、CI がすべて通ってから `main` にスクワッシュマージする。

- `main` への直接 push はしない。ドキュメントだけの変更も同じ流れに乗せる。
- 開発者は 1 人のため、PR のレビューはセルフレビューで足りる。ただし CI が赤のままマージはしない。
- Construction のワークツリーは `main` を基点とし、マージ先も `main` とする。1 Bolt = `main` 上の 1 コミット（Bolt のスラッグ名）とする。
- 長命のブランチ（リリースブランチを含む）は作らない。

## Walking Skeleton

私たちは最初の作業単位で、端から端まで通る薄い一本を作り、それが動くことを確かめてから機能を足していく。

- 薄い一本は「GUI を起動し、選んだ条件で GetLogEvents を呼んで結果を表示する」までの極小版とする。GUI・AWS SDK・認証のつなぎ込みの不安を最初に潰すのがねらい。
- 確認は次の 3 つを合わせて行う。
  1. 自動テスト `cargo test` がすべて通る。
  2. 実際の AWS に対して取得を行い、結果を表示する小さな確認用プログラムを手元で実行する。
  3. GUI を起動し、取得結果が表示されることを目で確かめる。
- 2 と 3 は開発者本人の手元で、本人の AWS 認証情報を使って行う。自動テストと CI からは実際の AWS に接続しない。
- 確認が済んだら、骨組みのチェックポイントを人間が承認してから次の作業単位に進む。

## Testing Posture

私たちはテストを各 Bolt の成果物の一部として扱う。純粋なロジックはテストを先に書き、外部とつながる層は実装してからテストを書く。

- **Methodology**: custom
- **Ordering**: 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。
- テストの量と種類は、記録済みの Test Strategy（`Standard`）に従う。
- 数値のカバレッジ下限は設けない（スコープ `local-tool` には org.md のスコープ別下限がない）。その代わり、GUI に依存しない層（ライブラリ側）の公開関数には必ずテストを書く。
- 自動テストと CI では実際の AWS に接続しない。AWS 呼び出しは trait の裏に置き、テストではモック／スタブに差し替える。
- テストは `cargo test` で実行し、CI でマージ前に実行する。既存のテストはすべて通った状態を保つ。

## Guard Policy

<!-- Affirmed by the team. Mode: strict, relaxed, or off. Strict here holds for every intent and cannot be changed from chat. A section under the retired Change Control heading, written by an earlier release, is still read. -->

## Deployment

私たちのプロジェクトにはデプロイ先のサーバーがない。「デプロイ」はローカルアプリの配布を指す。

- MVP の配布はソースからのビルドだけとする（`cargo build` / `cargo install`）。
- GitHub Releases でのビルド成果物の公開、macOS の署名・公証は MVP の後に検討する。
- AWS へのデプロイやサーバー環境（ステージング・本番）への反映は行わない。そのため org.md のステージング・本番の既定は、適用する環境がない。
- リリースを区切る場合は、長命のリリースブランチではなく `main` 上のタグを使う。

## Code Style

私たちは Rust 標準のツールとプロジェクトの設定ファイルに従う。エージェントは提案の前にこれらの設定を読む。

- フォーマッタ：`rustfmt`。CI で `cargo fmt --check` を必須とし、通らなければマージしない。
- リンタ：`clippy`。CI で実行するが、警告はエラー扱いにしない（`-D warnings` は付けない）。clippy がエラーとして報告するもの（deny レベルの lint）は org.md のとおり PR をブロックする。
- 命名規則：Rust の慣用（関数・変数は snake_case、型は UpperCamelCase、定数は SCREAMING_SNAKE_CASE）。プロジェクト独自の改名ルールは設けない。
- 構成：GUI に依存しないライブラリと GUI アプリを分ける。AWS 呼び出しはアプリ側で定義した境界（trait）の裏に置く。
- エラー処理：テスト以外のコードでは `unwrap()` / `expect()` を使わず、エラーは `Result` で呼び出し元に返す。
- 依存関係と秘密情報：`Cargo.lock` をコミットする。CI で `cargo-deny` による依存関係のチェック（脆弱性を含む）を行う。GitHub の秘密情報スキャン（secret scanning）を有効にする。
- フレームワークがスタイルを提案するのは、`rustfmt` / `clippy` が扱っていない点に限る。

## Forbidden

<!-- Team-specific forbidden patterns -->

## Mandated

<!-- Team-specific mandates -->

## Corrections

<!-- Self-learning loop appends here. -->
