# Performance Requirements — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/functional-design/`（functional-spec.md・rules.md）、`inception/requirements-analysis/requirements.md`（NFR1〜NFR10）、`inception/contract-design/contract-summary.md`、`codekb/local-sights/`。質問票の回答は `nfr-requirements-questions.md` の Q1・Q2。

| ID | 要件 | 目標・測り方 | 出典 |
|----|------|--------------|------|
| NFR1.1 | 全件がそろうまでの時間 | 偽物のゲートウェイ（1 回の呼び出し 200 ミリ秒、ストリーム 100 本・各 1 回、東京 12 回／秒）で、全件がそろうまで 10 秒以下（1 本ずつの今の方式 20 秒の半分以下）。時間を止めて進めるテストで測る | NFR1、functional-spec 5 節 |
| NFR1.2 | 最初の 1 件が出るまでの時間 | 偽物でストリーム 20 本、範囲にログがあるのが順で 5 本目から、1 回 200 ミリ秒のとき、最初の on_batch が 1 本ずつの場合（約 1 秒）より早い | NFR1、AC1.2.4 |
| NFR1.3 | 同時に読む本数の上限の確認 | 偽物で 1 回の呼び出しを 8 ÷ 上限 秒より長く（東京なら 700 ミリ秒以上）したとき、同時に出ている呼び出しの最大がちょうど 8（機能設計の 2 回目のレビューの Major を、テストの条件として片付ける） | BR1.7、functional-design レビュー 2 |
| NFR5.1 | 100 万件の一覧の滑らかさ | 既存の速度測定テスト（`cargo test -p local-sights-core --release --lib -- timeline:: --ignored` など）が今と同じ基準で通る | NFR5 |
| NFR5.2 | 渡し待ちのページ | 渡し待ちは最大 8 ページ（読み途中のストリームごとに 1 ページ）。それ以上はメモリにためない | BR1.11 |
| NFR2.1 | 手元の実際の AWS での速さの確認 | 確認用プログラムで、テスト用ロググループ（東京）を main の版と U1 の版で同じ時間範囲で取得し、件数が同じで U1 のほうがかかった時間が短ければ合格。数字の目標は置かない | NFR2、Q2 |

## Assumptions & Open Questions

None.
