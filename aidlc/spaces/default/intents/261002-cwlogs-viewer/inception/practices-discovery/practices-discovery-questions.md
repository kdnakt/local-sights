# Practices Discovery 質問票

グリーンフィールドのため、選択肢の既定は `aidlc/spaces/default/memory/org.md` を出発点にしている。そこに、リード下書き（`team-practices.md`、`evidence.md`）と、品質・開発・セキュリティ担当の確認（`contributions/`）の指摘を加えた。

## Q1. ブランチとマージの進め方はどうしますか？

背景：開発者は 1 人です。PR を経由せず `main` に直接 push すると、フォーマット・lint・テスト・依存関係のチェックを通らずに入ります（セキュリティ担当の指摘）。

A. 短命のブランチ → PR → CI が通ったらスクワッシュマージ（`main` への直接 push はしない）
B. 原則は A だが、ドキュメントだけの変更などは `main` に直接 push してよい（CI は push 後に確認し、赤なら直ちに直す）
C. `main` に直接 push する（PR は使わない）
D. Not yet defined
X. Other (please specify)

[Answer]: C. `main` に直接 push する（PR は使わない）

## Q2. 最初に、端から端まで通る薄い一本を作りますか？（薄い一本＝本格的な機能を入れる前に、全体がつながることを確かめるための最小版）

背景：このプロジェクトなら「GUI を起動し、選んだ条件で GetLogEvents を呼んで結果を表示する」までの極小版です（リード下書き）。期限が約 1 週間なので、GUI・AWS SDK・認証のつなぎ込みの不安を最初に潰せます。

A. 作る（最初の作業単位で薄い一本を作り、確認してから機能を足す）
B. 作らない（作業単位の順に普通に作る）
C. Not yet defined
X. Other (please specify)

[Answer]: A. 作る（最初の作業単位で薄い一本を作り、確認してから機能を足す）

## Q3. 薄い一本ができたことを、どう確かめますか？（Q2 で作らない場合は Not applicable）

背景：GUI の表示は目で見るしかなく、実際の AWS につなぐには認証情報が要ります。自動で確かめる部分と、手で確かめる部分を分けます（品質担当の指摘）。

A. 自動テスト（`cargo test`）に加え、実際の AWS に対して取得結果を表示する小さな確認用プログラムを手元で実行し、GUI も起動して目で確かめる
B. 自動テストだけをコマンドとし、実際の AWS と GUI の確認は手動の確認項目にする
C. Not applicable
X. Other (please specify)

[Answer]: A. 自動テスト（`cargo test`）に加え、実際の AWS に対して取得結果を表示する小さな確認用プログラムを手元で実行し、GUI も起動して目で確かめる

## Q4. テストはどの順番で書きますか？

背景：コード生成のステップは、この回答（進め方と順番）だけを見て作業の進め方を決めます。

A. 実装してからテストを書く（test-after）：層ごとに、実装してからその層のテストを書いて実行する
B. ロジックだけ先にテストを書く（custom）：時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書き、AWS 接続と GUI は実装後にテストを書く
C. すべてテストを先に書く（TDD）
D. Not yet defined
X. Other (please specify)

[Answer]: B. ロジックだけ先にテストを書く（custom）：時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書き、AWS 接続と GUI は実装後にテストを書く

## Q5. テストのカバレッジ下限を設けますか？

背景：今回の進め方（`local-tool`）には、カバレッジ下限の既定がありません。一度下限を決めると、ビルドとテストのステップで下げられません（品質担当の指摘）。

A. 数値の下限は設けない。代わりに、GUI に依存しない層の公開関数には必ずテストを書く
B. GUI に依存しない層（ライブラリ側）だけ、80% の行カバレッジを下限にする（GUI の描画と起動部分は計測しない）
C. 全体で 80% の行カバレッジを下限にする
D. Not yet defined
X. Other (please specify)

[Answer]: A. 数値の下限は設けない。代わりに、GUI に依存しない層の公開関数には必ずテストを書く

## Q6. MVP の配布方法はどうしますか？

背景：サーバーへのデプロイはなく、「デプロイ」はローカルアプリの配布に読み替えます。署名・公証（Apple の確認手続き）なしで配ると、利用者の Mac で警告が出ます（セキュリティ担当の指摘）。

A. `main` のタグから GitHub Releases に macOS 向けビルドを公開する（MVP では署名・公証なし。チェックサムを添える）
B. MVP ではソースからのビルドだけにする（`cargo build` / `cargo install`）。Releases は後で
C. GitHub Releases に、署名・公証済みのビルドを公開する
D. Not yet defined
X. Other (please specify)

[Answer]: B. MVP ではソースからのビルドだけにする（`cargo build` / `cargo install`）。Releases は後で

## Q7. フォーマットと lint のチェックをどこまで厳しくしますか？

背景：Rust 標準の `rustfmt`（フォーマット）と `clippy`（lint）を使う前提です。

A. CI で `cargo fmt --check` と `cargo clippy -D warnings`（警告もエラー扱い）を通らなければマージしない
B. フォーマットは必須、clippy の警告はエラーにしない
C. どちらも CI では見ない（手元で実行するだけ）
D. Not yet defined
X. Other (please specify)

[Answer]: B. フォーマットは必須、clippy の警告はエラーにしない

## Q8. コードの構成とエラー処理の方針はどうしますか？

背景：テストしやすくするため、GUI と AWS 呼び出しを外側に置き、ログ処理のロジックを GUI に依存しない部分にまとめる案があります（開発担当・品質担当の指摘）。

A. GUI に依存しないライブラリと GUI アプリを分け、AWS 呼び出しはアプリ側で定義した境界（trait）の裏に置く。テスト以外のコードでは `unwrap()` / `expect()` を使わず、エラーは呼び出し元に返す
B. A の構成にするが、`unwrap()` / `expect()` の使用は禁止しない
C. 構成は決めず、設計ステップで決める
D. Not yet defined
X. Other (please specify)

[Answer]: A. GUI に依存しないライブラリと GUI アプリを分け、AWS 呼び出しはアプリ側で定義した境界（trait）の裏に置く。テスト以外のコードでは `unwrap()` / `expect()` を使わず、エラーは呼び出し元に返す

## Q9. 依存関係と秘密情報のチェックをどこまで入れますか？

背景：OSS として公開し、利用者の AWS 認証情報を扱います（セキュリティ担当の指摘）。

A. `Cargo.lock` をコミットし、CI で依存関係の脆弱性チェック（`cargo-deny` など）を行う。GitHub の秘密情報スキャンも有効にする
B. `Cargo.lock` のコミットと GitHub の秘密情報スキャンだけ（脆弱性チェックは後で）
C. 特に入れない
D. Not yet defined
X. Other (please specify)

[Answer]: A. `Cargo.lock` をコミットし、CI で依存関係の脆弱性チェック（`cargo-deny` など）を行う。GitHub の秘密情報スキャンも有効にする

## Q10. 次のうち、必ず守るルールとして残すものはどれですか？（select all that apply）

背景：ここで選んだものだけが、例外のないルールとして記録されます（選ばなかったものは記録しません）。

A. AWS の認証情報を、リポジトリ・アプリのログ・画面のエラー表示・キャッシュファイルに書き込まない
B. CloudWatch Logs は読み取り（DescribeLogGroups / DescribeLogStreams / GetLogEvents）だけを呼び、それ以外の API は呼ばない
C. 自動テストと CI では実際の AWS に接続せず、CI に AWS の認証情報を置かない
D. テレメトリやクラッシュレポートを外部に送らない
E. None
X. Other (please specify)

[Answer]: B, C, D, X. A の代わりに：AWS の認証情報がどこまでかわからないけど、リポジトリには置かない、ロール名とかであればログやエラー表示に残すのはあり。

## Follow-up Questions

## F1. Q10 の認証情報ルールの範囲をどう記録しますか？

背景：Q10 の回答（「リポジトリには置かない、ロール名とかであればログやエラー表示に残すのはあり」）の範囲を明確にします。「秘密の認証情報」＝シークレットアクセスキー・セッショントークン・SSO のトークン、「識別子」＝プロファイル名・ロール名（ARN）・アカウント ID・アクセスキー ID（AKIA...）と分けます。

A. 秘密の認証情報は、リポジトリ・ログ・エラー表示・キャッシュのどこにも書かない。識別子はログやエラー表示に出してよい（リポジトリには置かない）
B. 秘密の認証情報をリポジトリに置かないことだけをルールにする（ログやエラー表示は縛らない）
X. Other (please specify)

[Answer]: X. アクセスキーIDはログ出力NG。他の識別子は良さそう。

## F2. Q1「main に直接 push」と組織の既定ルールの矛盾をどうしますか？

背景：組織の既定（`org.md` の Way of Working と Code Style）は「短命ブランチ → `main` へスクワッシュマージ、CI で lint が失敗したらマージ不可」です。チームのルールは組織の既定に矛盾できないため、Q1 の回答のままだと登録時に拒否されます。

A. 既定に合わせる：短命ブランチ → PR → CI が通ったらスクワッシュマージ（Q1 を A に変更）
B. 折衷：原則はブランチ → PR。ドキュメントだけの変更などは `main` に直接 push 可（Q1 を B に変更）
C. `main` 直接 push を維持し、チームのルールとしては記録しない
X. Other (please specify)

[Answer]: A. 既定に合わせる：短命ブランチ → PR → CI が通ったらスクワッシュマージ（Q1 を A に変更）

## Consolidated Summary Confirmation

回答のまとめ：

- ブランチとマージ：短命のブランチ → PR → CI が通ったらスクワッシュマージ。`main` への直接 push はしない（Q1 を F2 で A に変更）
- 薄い一本：最初の作業単位で作り、確認してから機能を足す（Q2）
- 薄い一本の確認：`cargo test` に加え、実際の AWS に対して取得結果を表示する確認用プログラムを手元で実行し、GUI も起動して目で確かめる（Q3）
- テストの順番：時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書き、AWS 接続と GUI は実装後にテストを書く（Q4）
- カバレッジ：数値の下限は設けない。GUI に依存しない層の公開関数には必ずテストを書く（Q5）
- 配布：MVP はソースからのビルドだけ（`cargo build` / `cargo install`）。Releases は後で（Q6）
- フォーマットと lint：CI で `cargo fmt --check` は必須。clippy は CI で実行するが、警告はエラーにしない（Q7）
- 構成とエラー処理：GUI 非依存ライブラリと GUI アプリを分け、AWS 呼び出しは trait の裏に置く。テスト以外で `unwrap()` / `expect()` を使わず、エラーは呼び出し元に返す（Q8）
- 依存関係と秘密情報：`Cargo.lock` をコミット、CI で `cargo-deny` などの脆弱性チェック、GitHub の秘密情報スキャンを有効化（Q9）
- 必ず守るルール（Q10、F1）：
  - 秘密の認証情報（シークレットアクセスキー・セッショントークン・SSO のトークン）とアクセスキー ID は、リポジトリ・ログ・エラー表示・キャッシュに書かない。プロファイル名・ロール名（ARN）・アカウント ID はログやエラー表示に出してよい（リポジトリには置かない）
  - CloudWatch Logs は読み取り（DescribeLogGroups / DescribeLogStreams / GetLogEvents）だけを呼ぶ
  - 自動テストと CI では実際の AWS に接続せず、CI に AWS の認証情報を置かない
  - テレメトリやクラッシュレポートを外部に送らない

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
