# Requirements — リリースの .app をアドホック署名する

出典の記法：`[desc]` は初期の依頼文、`[Q<n>]` は `requirements-analysis-questions.md` の回答、`[codekb]` はコードの知識ベース（`aidlc/spaces/default/codekb/local-sights/`）、`[assumption]` は本書で置いた前提。

## 1. 意図の分析

リリースの `.dmg` をダウンロードして開くと、macOS 15 Sequoia が「"Local Sights" は壊れているため開けません。ゴミ箱に入れる必要があります。」と表示し、GUI だけでは起動できない。原因はアプリのバンドルに署名が一切ないこと。Apple Developer Program には参加せず、CI でアドホック署名（証明書を使わない自己署名）を付けることで、このダイアログを「Apple はこのアプリを確認できません」系の警告に変え、システム設定の「このまま開く」だけで起動できるようにする。`[desc]`

ねらいは次の 2 つ。

- 会社の PC に配るとき、ターミナルで `xattr` を打たなくても GUI だけで初回起動できるようにする。`[desc]`
- `docs/install.md` の初回起動の手順を、実際に出るダイアログと一致させる。`[desc]`

### 成功指標

- リリースの `.dmg` から入れた `Local Sights.app` の初回起動で、「壊れている」ダイアログが出ず、「システム設定 → プライバシーとセキュリティ → このまま開く」の操作だけで起動できる（実際のリリース後に会社の Mac で確認する）。`[desc]` `[Q3]`
- Release ワークフローが、署名の付いていない `.app` を公開してしまう前に失敗する。`[Q2]`

## 2. 機能要件

### FR1. Release ワークフローでのアドホック署名

- **FR1.1** `.github/workflows/release.yml` の `build-and-publish` ジョブのビルドステップ（`npm run tauri build -- --bundles dmg`）に、環境変数 `APPLE_SIGNING_IDENTITY` を値 `-` で渡す。Tauri はこの変数を署名 ID として `codesign` に渡し、`-` はアドホック署名を意味する。`[desc]` `[codekb]`
- **FR1.2** `src-tauri/tauri.conf.json` は変更しない（`bundle.macOS` の節はなく、署名 ID は環境変数からだけ読まれる）。`[codekb]`
- **FR1.3** `.github/workflows/release-dry-run.yml` は変更しない。`[Q1]`
- **FR1.4** `release.yml` のステップ名「build the .dmg (unsigned)」、冒頭コメントの「The app is not signed or notarized.」、リリースノート `NOTES` の「Unsigned macOS (Apple Silicon) build」を、「アドホック署名あり、Apple による署名・公証なし」の意味に改める。秘密情報を使わない旨の記述は残す。`[codekb]`

受け入れ基準：

```
Given `main` のコミットに `vX.Y.Z` のタグを押した
When Release ワークフローの build-and-publish ジョブがビルドステップを実行した
Then 生成された `Local Sights.app` には有効なアドホック署名が付いている（`codesign -dv` の出力に `Signature=adhoc` が含まれる）
And ワークフローは引き続き自動の GITHUB_TOKEN 以外の秘密情報を使わない
```

```
Given Release dry run ワークフローを手動で実行した
When build ジョブが完了した
Then その成果物は従来どおり署名なしのままである（変更しない）
```

### FR2. CI での署名検証

- **FR2.1** `release.yml` のビルドステップの直後、`.dmg` の存在確認より前に、ビルドされた `.app`（`target/release/bundle/macos/Local Sights.app`）の署名を `codesign --verify --deep --strict` で検証するステップを置く。検証に失敗したらワークフローを失敗させ、Release は作らない。`[Q2]`
- **FR2.2** 同じステップで `codesign -dvv` の出力をログに出し、`Signature=adhoc` であることを確認する。Developer ID など想定外の署名 ID が使われていたら失敗させる。`[Q2]`
- **FR2.3** 検証ステップも秘密情報を使わない。`[codekb]`

受け入れ基準：

```
Given ビルドステップに `APPLE_SIGNING_IDENTITY` が渡されなかった（付け忘れ、または Tauri 側の挙動変更で署名が付かなかった）
When 署名検証ステップが実行された
Then ステップは失敗し、`::error::` でその旨を出力し、後続の公開ステップは実行されない
```

```
Given ビルドステップがアドホック署名付きの `.app` を生成した
When 署名検証ステップが実行された
Then ステップは成功し、ログに `Signature=adhoc` を含む `codesign` の出力が残る
```

### FR3. 手順書と関連文書の更新

- **FR3.1** `docs/install.md` の「Before you install」で、「not signed or notarized by Apple」の記述を、「Apple の Developer ID による署名も公証もない（アドホック署名のみ）」と正確に書き直す。会社のルール確認を促す趣旨は残す。`[desc]` `[codekb]`
- **FR3.2** `docs/install.md` の「First launch」の節を、アドホック署名付きのアプリで実際に出るダイアログに合わせる。macOS 15 以降では、初回のダブルクリックで「Apple はこのアプリにマルウェアが含まれていないことを確認できませんでした」系の警告が出て「完了」を押し、システム設定の「プライバシーとセキュリティ」で「このまま開く」を押し、もう一度開く、という GUI だけの流れを主手順とする。`[desc]`
- **FR3.3** 「壊れているため開けません」が出た場合の `xattr -dr com.apple.quarantine` の回避策は、古い版（署名なしの版）を入れたときや想定外の場合の予備手順として節の末尾に残す。`[assumption]`
- **FR3.4** `docs/releasing.md` と `README.md` の「not signed or notarized」「unsigned app」の記述を FR3.1 と同じ意味にそろえる。`[codekb]`
- **FR3.5** `docs/releasing.md` のリリース後の確認手順（手順 7）に、初回起動で「壊れている」ダイアログが出ないこと、GUI だけで開けることの確認を加える。「壊れている」が出た場合は署名が付いていないリリースの不具合として扱う旨を書く。`[Q3]`
- **FR3.6** 既存の文書は英語で書かれているため、更新箇所も英語で書く。`[codekb]`

受け入れ基準：

```
Given 更新後の `docs/install.md` を読む
When 「First launch」の節を上から追う
Then macOS 15 以降の主手順にターミナルの操作は含まれず、`xattr` は予備手順としてだけ現れる
And リポジトリ内に「not signed or notarized」「unsigned」で署名がまったくないと読める記述は残っていない（`grep` で確認できる）
```

```
Given 更新後の `docs/releasing.md` を読む
When リリース後の確認手順を追う
Then 初回起動のダイアログの種類を確認する手順があり、「壊れている」が出たときの扱いが書かれている
```

## 3. 非機能要件

- **NFR1** 秘密情報の不使用：Release ワークフローは引き続き自動の `GITHUB_TOKEN` だけを使い、Apple の証明書や鍵、AWS の認証情報をリポジトリにも GitHub Actions のシークレットにも置かない。`[codekb]` `[desc]`
- **NFR2** 既存の CI の緑を保つ：`ci.yml` の全ジョブ（`cargo test`、Vitest、`cargo fmt --check`、clippy、`prettier --check`、ESLint、`tsc --noEmit`、`cargo-deny`、`npm audit`）は変更後も通る。`[codekb]`
- **NFR3** 再実行の安全性：Release ワークフローの再実行で同じタグの `.dmg` を置き換える既存の振る舞いを壊さない。`[codekb]`

## 4. 制約

- Apple Developer Program には参加しない。Developer ID による署名も Apple の公証も行わない。`[desc]`
- チームのルールどおり、`main` への直接 push はせず、短命ブランチから PR を作り、CI が通ってからスクワッシュマージする。
- `release-dry-run.yml` は今回の対象外とする。`[Q1]`
- 変更は GitHub Actions の設定と Markdown 文書に限り、アプリのコードは変えない。`[codekb]`

## 5. 前提

- **A1** アドホック署名を付けると、macOS 15 Sequoia の Gatekeeper は「壊れているため開けません」ではなく「Apple は確認できませんでした」系の警告を出し、システム設定に「このまま開く」が現れる。これは macOS 側の挙動であり、CI では確かめられない。実際のリリース後に会社の Mac で確認する。`[desc]` `[Q3]` `[assumption]`
- **A2** Tauri 2.x の CLI は、環境変数 `APPLE_SIGNING_IDENTITY` に `-` を渡すとアドホック署名を行う。`[desc]` `[assumption]`
- **A3** `npm run tauri build -- --bundles dmg` は `.dmg` を作る前に `target/release/bundle/macos/Local Sights.app` を生成するため、署名検証ステップはそのパスを対象にできる。`[codekb]` `[assumption]`
- **A4** `docs/install.md` の読者は会社の PC の利用者で、ターミナルの操作に慣れていない。GUI だけの手順を主手順にする理由はここにある。`[desc]`

## 6. 対象外

- Developer ID による署名と Apple の公証。`[desc]`
- `curl` で取得するインストールスクリプトや Homebrew の tap など、隔離属性を付けない配布経路。`[desc]`
- `release-dry-run.yml` への署名の追加。`[Q1]`
- タグを打つ前のドライラン成果物による手元確認の手順化。`[Q3]`
- 自動更新の仕組み、Intel Mac や Windows への配布。

## 7. 未決事項

- FR2.2 で `Signature=adhoc` 以外を失敗とするかどうかは、`codesign -dvv` の出力形式を実装時に確認して決める。`Signature=adhoc` の行が将来の macOS で変わる可能性があれば、署名の有効性（`--verify`）だけを必須にし、`adhoc` の確認は警告にとどめる。
- 実際のダイアログの文言は macOS の版によって異なる。FR3.2 の文言は Apple の公開情報に基づく想定で書き、リリース後の確認（FR3.5）で食い違いがあれば手順書を直す。
- チームのルール（`team.md` の Deployment）は「今は Apple の署名・公証をしない」と書かれている。アドホック署名は Apple による署名ではないので矛盾しないが、「アドホック署名は行う」と明文化するかどうかは、この作業の外で決める。
