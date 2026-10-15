# Practices Discovery Questions — Windows 対応（IB-16）

チームの決まり（team.md）のうち、Windows 対応で変わるところを決めます。下書きは `team-practices.md`、根拠と変更案（C1〜C12）は `evidence.md` にあります。各質問の A が下書きに入れた推奨です。

## Q1. Windows のインストーラの形はどれにしますか？

背景：会社の PC で管理者の権限なしに入れられるかが変わります。macOS 版はアプリをフォルダに置くだけで入れられます。

A. NSIS の `-setup.exe`（利用者ごとに入れる。管理者の権限は要らない）
B. MSI（WiX。ふつうは管理者の権限が要る）
C. 両方を作る
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q2. CI で Windows をどこまで確かめますか？

背景：今の CI は Linux と macOS だけです。Windows 専用のコード（AWS CLI の起動、キャッシュの置き場所など）のテストは、Windows で動かさないと走りません。

A. Tauri アプリのジョブに `windows-latest` を足し（clippy・ビルド・アプリのテスト）、ライブラリのテストも Windows で動かし、マージに必須のチェックにする
B. A と同じだが、マージに必須にはしない
C. リリースと Release dry run でビルドするだけ
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q3. 薄い一本の手元の確認（実際の AWS からの取得と、画面を目で見る確認）は、どの OS で行いますか？

背景：今の決まりでは、開発者の手元の Mac で確かめることになっています。

A. その作業が対象にする OS で行い、両方の OS に効く変更や OS ごとに分かれるコードの変更なら両方で行う
B. この作業の間は Windows だけで行う
C. いつも両方の OS で行う
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q4. リリースのワークフローの形はどれにしますか？

背景：1 つのタグで macOS と Windows の両方を作るかどうかを決めます。

A. 1 つのタグで両方の OS を作り、両方が成功したときだけ 1 つの Release として公開する。Release dry run も両方を作る
B. OS ごとに作り、できたものから Release に足していく
C. OS ごとにタグやワークフローを分ける
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q5. Windows 版を初めて配るときの版はどれにしますか？

背景：team.md では次のタグを `v0.1.2` と決めています。`v0.1.2` は、マージ済みの取得の高速化（PR #22）を配る版として、まだ付けていません。公開したタグは動かしません。

A. 取得の高速化は `v0.1.2` で先に配り、Windows 版は `v0.2.0`（新しい OS に対応するので、真ん中の数字を上げる）
B. 取得の高速化は `v0.1.2` で先に配り、Windows 版は `v0.1.3`
C. `v0.1.2` は付けず、取得の高速化と Windows 版を合わせて `v0.2.0` で配る
D. 配るときに決める
E. Not yet defined
X. Other (please specify)

[Answer]: C

## Q6. 対象にする Windows 11 の最低の版はどう決めますか？

A. Microsoft がサポートしている Windows 11（x64）とし、開発者の PC で確かめる
B. 決まったビルドを最低とする（例：23H2 以降）
C. 「Windows 11 x64」とだけ書く
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q7. Windows の入れ方の手順書をどこに書き、AWS CLI が要ることをどう書きますか？

背景：SSO ログインのボタンは AWS CLI を起動します。署名をしないため、SmartScreen の警告の開き方も書く必要があります。

A. `docs/install.md` に macOS と Windows の節を分けて書く。AWS CLI v2 の入れ方、アプリが OS ごとに AWS CLI をどこで探すか、SmartScreen の「詳細情報 → 実行」の手順を含める
B. `docs/install-windows.md` を別に作る
C. README に書く
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q8. 改行コードはどうそろえますか？

背景：Windows で取り出したときに改行が CRLF になると、`cargo fmt --check` や `prettier --check` の結果が OS で変わることがあります。

A. `.gitattributes` で LF に固定する
B. 何もせず、`core.autocrlf=false` にする手順を書く
C. Prettier の `endOfLine: auto` にする
D. Not yet defined
X. Other (please specify)

[Answer]: A
