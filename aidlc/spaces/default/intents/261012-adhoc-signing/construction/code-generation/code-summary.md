# Code Summary — リリースの .app をアドホック署名する

## 変更したファイル

| ファイル | 変更 |
|---|---|
| `.github/workflows/release.yml` | ビルドステップを「build the .dmg (ad-hoc signed)」にし、`env` に `APPLE_SIGNING_IDENTITY: "-"` を追加。直後に「the .app carries a valid ad-hoc signature」ステップを追加（`.app` の存在確認 → `codesign --verify --deep --strict --verbose=2` → `codesign -dvv` の出力に `Signature=adhoc` があることを確認。どれかが失敗すれば `::error::` で終了し、Release は作られない）。冒頭コメントとリリースノート `NOTES` を「アドホック署名あり、Apple の署名・公証なし」の表現に変更 |
| `docs/install.md` | 「Before you install」をアドホック署名の説明に書き直し。初回起動の節を「First launch: the "could not verify" warning」にし、macOS 15 以降は Done → Privacy & Security の Open Anyway → 再度開く、の GUI だけの手順を主手順に。「is damaged」が出る場合の説明を小見出しで節末に置き、`xattr` は最後の手段として残した。Update の節のアンカーを更新 |
| `docs/releasing.md` | 署名の説明を更新。手順 7 に初回起動ダイアログの確認と「is damaged」が出た場合の扱い（不具合として扱い配らない）を追加。「When something fails」に署名検証の失敗を追加 |
| `README.md` | 「the first-launch warning (the app is ad-hoc signed, not notarized by Apple)」に修正 |
| `scripts/release/signing.ts`（新規） | 純粋なチェック関数 4 つ：`readTauriBuildSigningIdentity`、`findSignatureCheckStep`、`findUnsignedClaims`、`terminalFallbackOnly`。行ベースで `steps:` と各ステップの `name` / `run` / `env` だけを読む。YAML ライブラリは追加していない |
| `scripts/release/signing.test.ts`（新規） | Vitest 24 件。関数の単体テスト（合成文字列）と、リポジトリの実ファイルに対する要件ごとのテスト（`describe("repository")`） |

変更していないもの：`.github/workflows/release-dry-run.yml`（FR1.3）、`src-tauri/tauri.conf.json`（FR1.2）、アプリのコード（Rust / React）。

## 主な実装判断

- 実ファイルの読み取りは `readFileSync` ではなく Vite の `?raw` インポートにした。プロジェクトに `@types/node` が無く、`node:fs` を import すると `tsc --noEmit` が通らないため。既存の `src/commandRegistration.test.ts` と同じ方式。
- `findUnsignedClaims` は行ではなく段落を結合した「文」単位で判定し、`not signed with`、`not notarized`、`ad-hoc` / `adhoc` を含む文は除外する。行末で折り返された正直な記述を誤検出しないため。
- 署名検証ステップは GitHub Actions の `bash -e` の下でも意図どおり失敗するよう `if !` と `case` で分岐し、`codesign -dvv` は stderr に出るので `2>&1` で捕まえる。`.app` のパスはスペースを含むので常に引用する。
- 要件の未決事項だった `Signature=adhoc` の確認は必須（失敗扱い）にした。macOS 側の出力形式が変わったら、このステップの `case` 行だけを緩めればよい。

## テスト

- `npx vitest run`：33 ファイル / 320 件すべて合格（新規 24 件を含む）。
- `npx prettier --check .`、`npx eslint .`、`npx tsc --noEmit`：すべて合格。
- `cargo test`：アプリのコードを変えていないため、Build and Test で既存のまま通ることを確認する。
- 自動で確かめられないもの：署名検証ステップが macOS ランナーで実際に通ること（A2、A3）と、Gatekeeper のダイアログが変わること（A1）。次のタグ付きリリースと、その後の会社の Mac での初回起動で確認する（`docs/releasing.md` 手順 7）。

## 計画からの逸脱

- 実ファイルの読み取り方法のみ（上記のとおり `?raw` インポート）。それ以外は計画どおり。
