# Practices Discovery — 質問

これまでに決めたチームの進め方（`aidlc/spaces/default/memory/team.md`）を土台にしています。今回はおもに「配布（会社 PC へのインストール）」の方針を決め直します。リポジトリから読み取れなかった点だけを聞きます。

## Q1. 会社 PC の OS と CPU

今の CI は macOS と Linux でしか動いていません。Windows を入れると、ビルド環境の追加とインストーラ形式の検討が必要になり、作業がかなり増えます。

A. macOS（Apple Silicon）だけ
B. macOS（Apple Silicon と Intel の両方）
C. Windows だけ
D. macOS と Windows の両方
X. Other (please specify)

[Answer]: A

## Q2. 会社 PC に何を入れるか

A. Tauri のインストーラ（macOS なら `.dmg` に入った `.app`）を作って入れる。`cargo install` は開発用に残す
B. 今までどおりソースから `cargo install` / `npm run tauri build` して手元でビルドしたものを使う（インストーラは作らない）
C. ビルドした `.app` だけを zip にして渡す
X. Other (please specify)

[Answer]:

## Q3. どこでビルドして、どこに置くか

A. `main` に `vX.Y.Z` のタグを付けると GitHub Actions がビルドし、GitHub Releases（下書き）に置く。会社 PC はそこから取得する
B. A と同じだが、会社のネットワークから GitHub に届かない場合に備えて、手でファイルを運ぶ手順も手順書に書く
C. 自分の手元の PC でビルドし、ファイルを手で会社 PC に運ぶ（GitHub Releases は使わない）
X. Other (please specify)

[Answer]:

## Q4. 署名と公証（notarization）

署名・公証とは、Apple にアプリの作成者を証明してもらう仕組みです。しないと初回起動時に「開発元を確認できません」という警告が出ます。するには Apple Developer Program（年額の有料会員）と、証明書を GitHub の秘密情報に置く作業が必要です。

A. 今はしない。初回起動時の警告の回避方法を手順書に書く。会社のルールで未署名アプリが禁止されていると分かったら見直す
B. 署名と公証を行う（Apple Developer Program に加入する）
C. 会社のルールをまだ確認していないので、要件定義の時点で決める
X. Other (please specify)

[Answer]:

## Q5. バージョンの付け方と更新・巻き戻し

バージョン番号は今 `Cargo.toml`・`package.json`・`tauri.conf.json` の 3 か所に別々に書かれています。

A. セマンティックバージョニング（`v0.1.0` から）。3 か所をそろえ、タグと合わなければリリースのビルドを失敗させる。自動更新は付けず、手で入れ替える。巻き戻しは前のバージョンを入れ直す
B. A と同じだが、自動更新（Tauri の updater）も付ける
C. バージョンは気にせず、日付などで区別する
X. Other (please specify)

[Answer]:

## Q6. チームの進め方の記述を、今の CI の実態に合わせるか

今の CI は、画面側のテスト（Vitest）、Prettier・ESLint・`tsc --noEmit`・`npm audit --audit-level=high` も必須にしていますが、team.md には `cargo test` と Rust のツールしか書かれていません。また、テストの下限の説明に「スコープ `local-tool`」と書かれていますが、今回の作業は別の計画で動いています。

A. 実態に合わせて書き足す（画面側のテストとチェックを追記し、下限は「org.md のスコープ別の決まりに従う」と書き直す）
B. 今のまま変えない
X. Other (please specify)

[Answer]:

## Q7. GitHub 側の設定

リポジトリのファイルからは、GitHub 上で次の設定が実際に有効かどうかが分かりません。team.md では、`main` への直接 push をしないことと、秘密情報スキャン（secret scanning）を有効にすることを決めています。

A. どちらも有効にしてある
B. どちらか、または両方がまだ。今回の作業で有効にする手順を確認したい
C. 分からない
X. Other (please specify)

[Answer]:
