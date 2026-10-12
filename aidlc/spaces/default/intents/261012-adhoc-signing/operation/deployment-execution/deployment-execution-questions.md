# Deployment Execution Questions — リリースの .app をアドホック署名する

上流：`operation/deployment-pipeline/cd-config.md`、`deployment-strategy.md`、`rollback-runbook.md`、`construction/build-and-test/test-results.md`。

配布の実行とは、ここでは「変更を `main` に入れ、`v0.1.1` のタグを打って Release ワークフローに署名付きの `.dmg` を公開させ、開発者の Mac でスモークテストをする」こと。前提の確認（CI 相当の検査がこの環境ですべて通っていること）は Build and Test で済んでいる。データベースの移行や依存サービスはない。配布の時間帯の制約もない（利用者は手で入れ替えるため）。

いまの状態：
- 変更はこの作業環境の作業ツリーにあり、まだコミットされていない（`release.yml`、`docs/install.md`、`docs/releasing.md`、`README.md`、`scripts/release/signing.ts`、`signing.test.ts`、AI-DLC の記録）。
- `main` の先頭は `v0.1.0` と同じコミット `9d08cbf`。
- team.md の決まり：`main` に直接 push しない。短命ブランチから PR を作り、CI がすべて緑になってからスクワッシュマージする。

## Q1. この場でどこまで進めますか？

- A. ブランチを作ってコミットし、GitHub に push して PR を作る（推奨：CI がこの PR で回る。マージは CI が緑になってから開発者が行う）
- B. ブランチを作ってコミットするだけにし、push と PR は開発者が手元で行う
- C. 何もしない（変更はこの環境の作業ツリーに残す）
- X. Other (please specify)

[Answer]: A. ブランチを作ってコミットし、GitHub に push して PR を作る

## Q2. 版上げ（`0.1.1`）を同じ PR に含めますか？

`Cargo.toml`、`Cargo.lock`、`package.json`／`package-lock.json`、`src-tauri/tauri.conf.json` を `0.1.1` にそろえる変更です。含めると、マージ後すぐにタグを打てます。

- A. 同じ PR に含める（推奨）
- B. 別の PR にする（版上げは開発者が後で行う）
- C. Not yet defined
- X. Other (please specify)

[Answer]: A. 同じ PR に含める

## Q3. タグ打ちと公開は誰が行いますか？

タグを打つと Release ワークフローが動き、GitHub Releases に公開されます。マージは CI が緑になってから行う決まりなので、タグはそのあとになります。

- A. 開発者が、CI が緑になったのを見てマージし、`docs/releasing.md` の手順でタグを打つ（推奨：公開は開発者の手で）
- B. マージとタグ打ちもこの場で私が行う（CI の完了を待ってから。公開まで自動で進む）
- C. Not yet defined
- X. Other (please specify)

[Answer]: A. 開発者が、CI が緑になったのを見てマージし、`docs/releasing.md` の手順でタグを打つ
