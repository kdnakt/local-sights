# Requirements Analysis — 質問

配布の方針は前の工程で決めました（macOS Apple Silicon 向けの `.dmg`、タグを付けると GitHub Actions がビルドして GitHub Releases に下書きで置く、署名なし、semver、手で入れ替え）。ここでは、アプリ側の仕様と、会社 PC 側の条件のうち、こちらでは分からない点を聞きます。

## Q1. アプリの表示名

今の設定（`src-tauri/tauri.conf.json` の `productName`）は `local-sights` です。そのため「アプリケーション」フォルダには `local-sights.app` と表示され、Dock・メニューバー・ウィンドウのタイトルも `local-sights` になります。

A. `Local Sights` に変える（`Local Sights.app`。ウィンドウのタイトルも合わせる）
B. `local-sights` のまま
X. Other (please specify)

[Answer]: A

## Q2. 内部の識別子（identifier）

識別子は今 `dev.local-sights.app` です。Tauri は、末尾の `.app` が macOS のアプリの拡張子とまぎらわしいため、使わないよう勧めています。識別子を変えると設定やキャッシュの保存場所も変わりますが、会社 PC にはまだ何も入れていないので、変えるなら最初のリリースの前が一番影響が小さくなります（手元の Mac の古いキャッシュは使われなくなるだけです）。

A. 最初のリリースの前に変える（例：`dev.local-sights.viewer`）
B. 今のまま変えない
X. Other (please specify)

[Answer]: B

## Q3. 会社 PC の管理者権限

アプリを「アプリケーション」フォルダ（`/Applications`）に入れるには、通常は管理者権限が要ります。

A. 管理者権限がある。`/Applications` に入れる
B. 管理者権限がない。自分のホームの `~/Applications` に入れる
C. 分からない。手順書に両方の入れ方を書く
X. Other (please specify)

[Answer]: C

## Q4. 会社のルール（未署名アプリ・インストールの承認）

前の工程で、署名・公証は「今はしない」と決めました。ただし会社によっては、MDM（会社が PC を一括管理する仕組み）で未署名のアプリの起動が禁止されていたり、ソフトを入れる前に申請が必要だったりします。

A. 未署名のアプリも起動でき、申請も要らない
B. 申請は要るが、未署名でも起動はできる（申請の手順は自分で行う）
C. 未署名のアプリは禁止されている
D. まだ確認していない。分からない
X. Other (please specify)

[Answer]: D

## Q5. GitHub Releases の下書きを公開するか

このリポジトリは公開（public）です。GitHub Releases の下書きは、GitHub にログインしたリポジトリの持ち主しか見られません。会社 PC でログインせずにダウンロードするには、リリースを公開する必要があります。

A. CI が作った下書きを自分で確かめてから、手で「公開」する（公開後は誰でもダウンロードできる）
B. CI が最初から公開する（下書きにしない）
C. 下書きのまま使う。会社 PC でも GitHub にログインしてダウンロードする
X. Other (please specify)

[Answer]: B

### Q5-1（追加の質問）. 確定したチームの方針との食い違い

さきほど確定したチームの方針（`team.md` の Deployment）では「GitHub Releases に下書きとして置く」としています。Q5 の答え（CI がすぐ公開する）はこれと食い違います。

A. CI がすぐ公開する。チームの方針の「下書きとして置く」は、このあとの記録の手順で「公開して置く」に直す
B. チームの方針どおり下書きにして、確かめてから手で公開する（Q5 を A に改める）
X. Other (please specify)

[Answer]: A

## Q6. 会社のネットワーク

会社 PC から GitHub Releases にファイルを取りに行けるか、またアプリから AWS に接続するときにプロキシ（社内ネットワークの中継サーバー）を通す必要があるかで、作るものが変わります。

A. GitHub にも AWS にも直接つながる（プロキシなし）
B. プロキシ経由ならつながる（プロキシの設定に対応させたい）
C. GitHub からはダウンロードできない（ファイルを手で運ぶ手順が要る）
D. 分からない
X. Other (please specify)

[Answer]: A

## Q7. 今回の作業の「完了」をどこに置くか

A. 手順書に沿って会社 PC にインストールし、起動・ロググループの一覧表示・ログの取得表示まで、自分の目で確かめられたら完了
B. GitHub Releases に `.dmg` が置かれたところで完了（会社 PC での確認は今回の作業の外）
X. Other (please specify)

[Answer]: B
