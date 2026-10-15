# Performance Requirements — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/functional-design/`（functional-spec.md・rules.md BR4.1〜BR4.10、レビュー 1 回目の R-01〜R-06）、`inception/requirements-analysis/requirements.md`（FR4.1〜FR4.6、NFR1〜NFR10）、`inception/contract-design/contract-summary.md`（C4、C5）、`codekb/local-sights/technology-stack.md`。質問票の回答は `nfr-requirements-questions.md` の Q1 とまとめの確認。

| ID | 要件 | 目標・測り方 | 出典 |
|----|------|--------------|------|
| NFR5.3 | 1 秒ごとの描き直しは経過だけを描く小さな部品に閉じ込め、ログの表と状態行の残りは毎秒描き直さない | 画面のテストで、時計を 3 秒進めたときに経過の部品だけが描き直され、ログの表の部品の描画回数が増えないことを確かめる。100 万件の一覧の速度測定テストは今と同じ基準で通る | NFR5、R-03 |
| NFR5.4 | 経過のために画面へ送る回数を増やさない | `fetch-progress`（FetchProgressUpdate）の項目は今と同じ。`session-changed` を送る契機も今と同じ（経過のためだけに送らない）。ライブラリのテストで FetchProgressUpdate の項目が変わらないことを確かめる | ADR-005、C5、BR4.5 |
| NFR2.2 | 手元の確認（NFR2）に使える経過：最終の値は、ライブラリの戻らない時計で［取得］を受け入れた時点から取得の終わり（キャッシュへの保存を含む）までを表す | ライブラリのテスト（時計を差し替える）で、始めて 31.4 秒後に終わると最終の値が 31,400 ミリ秒、表示は 00:31 | NFR2、BR4.1、BR4.2 |

## Assumptions & Open Questions

None.
