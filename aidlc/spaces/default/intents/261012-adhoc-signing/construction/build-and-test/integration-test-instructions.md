# Integration Test Instructions — リリースの .app をアドホック署名する

## 対象

テスト戦略は Minimal。Minimal では結合テストの手順は作らない（単体テストは Code Generation の `unit-test-instructions.md` で扱う）。この変更はワークフロー定義と文書だけで、アプリの部品どうしのつなぎ目は変わらない。

## 自動では確かめられない「つなぎ目」

ワークフローと macOS のつなぎ目は、CI の自動テストでは確かめられず、実際のリリースで確かめる（要件 Q3 の回答、`requirements.md` の A1〜A3）。

| つなぎ目 | 確かめ方 | 時期 |
|---|---|---|
| Tauri CLI が `APPLE_SIGNING_IDENTITY=-` でアドホック署名する（A2） | Release ワークフローの「the .app carries a valid ad-hoc signature」ステップが通る | 次のタグ付きリリース |
| `.app` が `.dmg` の前に `target/release/bundle/macos/` に生成される（A3） | 同じステップの存在確認が通る | 次のタグ付きリリース |
| Gatekeeper のダイアログが「壊れている」から「確認できません」に変わる（A1） | 会社の Mac で `.dmg` から入れて初回起動し、`docs/install.md` の手順だけで開く（`docs/releasing.md` 手順 7） | リリース後 |

どれかが満たされなかった場合は、`docs/releasing.md` の「When something fails」と手順 7 の扱い（不具合として配らず、修正版のパッチをリリース）に従う。
