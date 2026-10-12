# Security Test Instructions — リリースの .app をアドホック署名する

## 対象

テスト戦略は Minimal で、セキュリティテストの手順は本来作らない。ただしこの変更は CI の署名まわりなので、秘密情報を増やしていないこと（NFR1、project.md の Forbidden）だけは確かめる。

## 確認したこと

| 観点 | 確かめ方 | 結果 |
|---|---|---|
| Apple の証明書や鍵を足していない | `release.yml` の差分に `secrets.` の参照や証明書の読み込みがない。`APPLE_SIGNING_IDENTITY` の値は固定の `-` で、秘密ではない | 確認済み |
| AWS の認証情報を CI に置いていない | `release.yml` の冒頭コメントどおり、ジョブは AWS に触れない。差分にも AWS の参照なし | 確認済み |
| 依存関係に脆弱性がない | `npm audit --audit-level=high` → 0 件。Rust の依存関係は変えていない（`cargo deny` は CI に任せる） | 確認済み |
| 署名検証ステップがログに秘密を出さない | `codesign -dvv` の出力は署名のメタデータ（識別子・形式・`Signature=adhoc`）だけ | 確認済み |

## 手順書に関する注意

`docs/install.md` の `xattr -dr com.apple.quarantine` は Gatekeeper の検査を回避する操作であるため、主手順から外し、「is damaged」が出る想定外のケースの最後の手段として末尾に残した（FR3.3）。会社のルールで未署名・未公証のアプリが禁止されている場合は使わない旨も「Before you install」に残している。
