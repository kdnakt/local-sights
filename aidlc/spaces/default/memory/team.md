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
- CI のジョブを足したり名前を変えたりしたときは、同じ PR の中で `main` のルールセットの必須の状態確認も合わせて直す（例：Windows のランナーを足したら `Tauri app (windows-latest)` を必須に加える）。必須に入っていないジョブは、赤でもマージを止めないため。

## Walking Skeleton

私たちは各作業の最初の作業単位で、端から端まで通る薄い一本を作り、それが動くことを確かめてから機能を足していく。

- 薄い一本は、その作業で新しく加える振る舞いが画面から AWS まで通る最小の版とする（例：261012-parallel-fetch では、複数のストリームを同時に読んで結果を表示するところまで。261015-windows-support では、Windows 11 の PC にインストーラで入れたアプリを起動し、選んだ条件で取得して結果を表示するところまで）。アプリを最初に作ったときの一本は「GUI を起動し、選んだ条件で GetLogEvents を呼んで結果を表示する」までの極小版だった。どれも、つなぎ込みの不安を最初に潰すのがねらい。
- 確認は次の 3 つを合わせて行う。
  1. 自動テスト `cargo test` がすべて通る（CI の各 OS のランナーでも通る）。
  2. 実際の AWS に対して取得を行い、結果を表示する小さな確認用プログラムを手元で実行する。
  3. GUI を起動し、取得結果が表示されることを目で確かめる。
- 2 と 3 は開発者本人の手元で、本人の AWS 認証情報を使って行う。その作業が対象にする OS の機械で行い、macOS は開発に使う Mac、Windows は手元の Windows 11（x64）の PC を使う。Windows では、CI で作ったインストーラで入れたアプリを使う。
- 両方の OS に効く変更（共通のコードや、OS ごとに分かれるコードを触る変更）では、2 と 3 を macOS と Windows の両方で行い、もう一方の OS で前と同じに動くことも確かめる。
- 自動テストと CI からは実際の AWS に接続しない。
- 確認が済んだら、骨組みのチェックポイントを人間が承認してから次の作業単位に進む。

## Testing Posture

私たちはテストを各 Bolt の成果物の一部として扱う。純粋なロジックはテストを先に書き、外部とつながる層は実装してからテストを書く。

- **Methodology**: custom
- **Methodology evidence**: 261002-cwlogs-viewer の面談（Q4 の回答 B）で、純粋なロジックはテスト先行、AWS 接続と GUI の層は実装後にテスト、という混在した進め方に決め、261011-local-release と 261012-parallel-fetch の面談でも変えていない。261015-windows-support の面談でも進め方そのものを変える回答はなく（Q2 は Windows のランナーでテストを流す決まりを加えるだけ）、増えるのは OS ごとに分かれる境界の層（AWS CLI の起動と停止、ファイルの置き場所と権限など）で、純粋なロジックの扱いは変わらない。今のコード（判断は副作用のない関数に寄せ、OS に触れる処理は `cfg(unix)` などで境界に閉じ込め、テストは各モジュールの中にある）と git の履歴にも、これと食い違う跡はない。
- **Ordering**: 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続・GUI・OS ごとに分かれる境界の層（プロセスの起動と停止、ファイルの置き場所と権限など）は先に実装してからその層のテストを書いて実行する。
- テストの量と種類は、記録済みの Test Strategy（`Standard`）に従う。
- カバレッジ：行カバレッジを CI で測り、80% を下回る PR はマージしない（ライブラリは `cargo-llvm-cov`、画面は Vitest のカバレッジ）。計測は Linux のランナーで行う。これは org.md のスコープ別の下限（`mvp` などの 80% 行カバレッジとマージ前の CI での実行）を CI の合否で確かめるもので、下限を下げることはしない。そのほかのスコープ別の決まりも org.md に従う。
- GUI に依存しない層（ライブラリ側）の公開関数には必ずテストを書く。
- 自動テストと CI では実際の AWS に接続しない。AWS 呼び出しは trait の裏に置き、テストではモック／スタブに差し替える。AWS CLI もテストでは本物を使わず、偽の CLI に差し替える。
- OS ごとに分かれるコード（`cfg(windows)`、`cfg(unix)` など）のテストは、その OS のランナーで実行されるように書く。偽の CLI などテストの道具も OS ごとに用意する。
- Rust 側のテストは `cargo test`（ライブラリとアプリの両方）で、画面側（フロントエンド）のテストは Vitest で実行し、どちらも CI でマージ前に実行する。Rust 側のテストは CI の Linux・macOS・Windows（`windows-latest`）の各ランナーで実行し、画面側のテストは OS に依存しないため Linux のランナーで実行する。既存のテストはすべての OS で通った状態を保つ。

## Guard Policy

<!-- Affirmed by the team. Mode: strict, relaxed, or off. Strict here holds for every intent and cannot be changed from chat. A section under the retired Change Control heading, written by an earlier release, is still read. -->

## Deployment

私たちのプロジェクトにはデプロイ先のサーバーがない。「デプロイ」はローカルアプリの配布、つまり会社の PC へのインストールを指す。

- 配布の対象：macOS（Apple Silicon）と、Microsoft がサポートしている版の Windows 11（x64）とする。Windows の対象の版は開発者の Windows の PC で動くことを確かめ、特定のビルド番号は書かない。Intel の Mac、Linux、Windows 10、arm64 の Windows には配らない。Microsoft Store でも配らない。
- 配布の形：Tauri のインストーラを作って会社の PC に入れる。macOS は `.app` を入れた `.dmg`、Windows は NSIS のインストーラ（`-setup.exe`、管理者の権限なしで利用者ごとに入れられるもの）とする。ソースからの `cargo install` は開発用として残す。
- 作る場所と置き場所：`main` に `vX.Y.Z` のタグを付けると、GitHub Actions がテストを流してから、macOS のランナーで `.dmg` を、Windows のランナーで Windows のインストーラを作り、両方がそろってから 1 つの Release として GitHub Releases に下書きにせず公開して置く。どちらかが作れなければ何も公開しない。会社の PC はそこから取得する。
- リリース前の確認：タグを付ける前に、手動で動かす「Release dry run」のワークフローで、`.dmg` と Windows のインストーラの作成だけを試せる（タグも公開もしない）。
- 署名と公証：Apple Developer ID の署名と Apple の公証はしない。macOS の `.app` は CI でアドホック署名だけを行い、初回起動時に出る Gatekeeper の警告を開く方法を手順書に書く。Windows のコード署名もしない。初回起動時に出る SmartScreen の警告（「Windows によって PC が保護されました」）を開く方法を手順書に書く。会社のルールで未署名のアプリが禁止されていると分かったら見直す（Windows の会社の PC には、署名のないアプリを入れてよいことを確かめてある）。
- バージョン：セマンティックバージョニングとし、`v0.1.0` から始める。`Cargo.toml`・`Cargo.lock`・`package.json`・`package-lock.json`・`src-tauri/tauri.conf.json` の版をそろえ、タグと一致しなければリリースのビルドを失敗させる。macOS と Windows の配布物は同じ版の番号で出す。
- 次の版のタグは `v0.2.0` とする。マージ済みの取得の高速化（PR #22）と Windows 版を合わせてこの版で配り、`v0.1.2` は付けない。公開済みの `v0.1.0` と `v0.1.1`（どちらも `main` 上のコミット）を含め、一度公開したタグは付け替えも使い回しもしない。
- 更新：自動更新の仕組みは入れず、手で入れ替える。macOS は新しい版の `.dmg` を取得し、中の `.app` を「アプリケーション」フォルダにドラッグして置き換える。Windows は新しい版のインストーラを実行して上書きで入れる。設定やキャッシュはアプリの外（macOS は `~/Library` の下、Windows は利用者の `AppData` の下）にあるため、入れ替えても残る。
- 戻し方：前の版の成果物（`.dmg` と Windows のインストーラ）は GitHub Releases に残しておき、問題があったときは前の版を入れ直す。
- 入れ方・初回起動・更新・戻し方の手順書は `docs/install.md` に置き、macOS と Windows の節を分けて書く。AWS CLI v2（アプリからの SSO ログインに使う）の入れ方と、アプリが探す場所も OS ごとに書く。Windows の節には SmartScreen の警告の開き方（「詳細情報」→「実行」）を含める。
- AWS へのデプロイやサーバー環境（ステージング・本番）への反映は行わない。そのため org.md のステージング・本番の既定は、適用する環境がない。
- リリースを区切るときは、長命のリリースブランチではなく `main` 上のタグを使う。

- `v0.2.0` には、取得の高速化（PR #22）と Windows 版に加えて、IB-11（Logs Insights 互換のクエリ）も含める。IB-11 が終わるまで `v0.2.0` のタグは付けない（261017-insights-query の範囲の定義 Q5、承認と引き継ぎ Q2） (learned 2026-10-17) <!-- cid:261017-insights-query:approval-handoff:d5ee0e8be18871f019870e37dc4f8598701d61e433caf0981e62580454096075 -->

## Code Style

私たちは Rust 標準のツール、画面側の標準的なツール、そしてプロジェクトの設定ファイルに従う。エージェントは提案の前にこれらの設定を読む。

- フォーマッタ：Rust は `rustfmt`。CI で `cargo fmt --check` を必須とし、通らなければマージしない。画面側は Prettier。CI で `prettier --check` を必須とし、通らなければマージしない。
- 改行コード：リポジトリのテキストファイルは LF にそろえ、`.gitattributes` で固定する。Windows で取り出しても `cargo fmt --check` と `prettier --check` が同じ結果になるようにするため。
- リンタ：Rust は `clippy`。CI で実行するが、警告はエラーにしない（`-D warnings` は付けない）。clippy がエラーとして報告するもの（deny レベルの lint）は org.md のとおり PR をブロックする。ワークスペース全体の clippy とアプリのビルドは CI の Linux・macOS・Windows の各ランナーで行い、OS ごとに分かれるコードもそれぞれの OS で検査する。画面側は ESLint を CI で実行し、失敗は org.md のとおり PR をブロックする。
- 型検査：画面側は CI で `tsc --noEmit` を実行し、通らなければマージしない。
- カバレッジ：Testing Posture のとおり、CI で行カバレッジを測り、80% を下回る PR はマージしない。
- 命名規則：言語の慣用に従う。Rust は関数・変数が snake_case、型が UpperCamelCase、定数が SCREAMING_SNAKE_CASE。画面側（TypeScript）は org.md のとおり camelCase を基本とする。プロジェクト独自の改名ルールは設けない。
- 構成：GUI に依存しないライブラリと GUI アプリを分ける。AWS 呼び出しはアプリ側で定義した境界（trait）の裏に置く。OS ごとに違う処理（プロセスの扱い、ファイルの場所と権限など）は `cfg` で分けた境界のコードに閉じ込め、判断はできるだけ OS に依存しない関数に寄せる。
- エラー処理：テスト以外のコードでは `unwrap()` / `expect()` を使わず、エラーは `Result` で呼び出し元に返す。
- 依存関係と秘密情報：`Cargo.lock` をコミットする。CI で `cargo-deny` による依存関係のチェック（脆弱性を含む）を、実際にビルドする機能を対象に行い、画面側の依存関係は `npm audit --audit-level=high` で確かめる。GitHub の秘密情報スキャン（secret scanning）を有効にする。
- フレームワークがスタイルを提案するのは、`rustfmt` / `clippy` / Prettier / ESLint が扱っていない点に限る。
## Forbidden

<!-- Team-specific forbidden patterns -->

## Mandated

<!-- Team-specific mandates -->

## Corrections

<!-- Self-learning loop appends here. -->
