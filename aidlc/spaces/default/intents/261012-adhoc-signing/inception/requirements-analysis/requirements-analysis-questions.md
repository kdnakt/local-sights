# Requirements Analysis Questions — リリースの .app をアドホック署名する

依頼：リリースの `.dmg` に含める `Local Sights.app` を CI でアドホック署名し、macOS 15 Sequoia で「壊れているため開けません」と出るダイアログを、システム設定の「このまま開く」だけで起動できる警告に変える。`docs/install.md` の初回起動手順も実際のダイアログに合わせて更新する。Apple Developer Program には参加しない。

いまのリポジトリの関係する状態（コードの知識ベースから）：
- `.github/workflows/release.yml` のビルドステップ（`npm run tauri build -- --bundles dmg`）は署名なし。同じビルドステップが `.github/workflows/release-dry-run.yml` にもある。
- `src-tauri/tauri.conf.json` に `bundle.macOS` の節はなく、Tauri は署名 ID を環境変数 `APPLE_SIGNING_IDENTITY` からだけ読む。`-` を渡すとアドホック署名になる。
- 「not signed or notarized」の文言が `docs/install.md`、`docs/releasing.md`、`README.md`、`release.yml` のコメントとリリースノートにある。
- ダイアログの変化は macOS 側の挙動なので、CI の自動テストでは確かめられない。

## Q1. アドホック署名はどのワークフローに入れますか？

`release-dry-run.yml` は、タグを打つ前に同じ手順でビルドだけ試すためのものです。ここにも入れておくと、タグを打つ前にドライランの成果物で手元の Mac の挙動を確かめられます。

- A. `release.yml` と `release-dry-run.yml` の両方に入れる（推奨：ドライランの `.dmg` で事前に確かめられる）
- B. `release.yml` だけに入れる
- C. Not yet defined
- X. Other (please specify)

[Answer]: B. `release.yml` だけに入れる

## Q2. 署名されたことを CI で自動で確かめますか？

ダイアログの変化そのものは CI で確かめられませんが、「ビルドした `.app` に署名が付いているか」は macOS ランナー上で `codesign --verify` で機械的に確かめられます。環境変数の付け忘れや Tauri 側の挙動変更を CI で検出できます。

- A. ビルド直後に `.app` の署名を検証するステップを足し、署名がなければワークフローを失敗させる（推奨）
- B. 自動の検証は入れず、手元の Mac での目視だけにする
- C. Not yet defined
- X. Other (please specify)

[Answer]: A. ビルド直後に `.app` の署名を検証するステップを足し、署名がなければワークフローを失敗させる

## Q3. 実際のダイアログの変化は、どこで確かめますか？

- A. タグを打つ前に、開発者がドライランの `.dmg` を自分の macOS 15 の Mac にダウンロードして初回起動を試し、手順書どおりに開けることを確かめる。その手順を `docs/releasing.md` に書く（推奨）
- B. 実際にリリースしてから、会社の Mac で確かめる
- C. Not yet defined
- X. Other (please specify)

[Answer]: B. 実際にリリースしてから、会社の Mac で確かめる
