# Performance Test Instructions — 画面を AWS マネジメントコンソールに寄せる

対象の要件は NFR2（MVP の NFR2・NFR3 の維持）：100 万件で絞り込み 100 秒以内、スクロールと行の展開に 1 秒以内に反応、取得中も固まらない。ログ一覧は自前の仮想スクロールのまま（AS3、code-summary.md の判断）で、Cloudscape の表の試作は環境変数で切り替える。

この測定は Tauri の実機でしか行えず、自動テストにはしない。今回の実行計画には後続の検証ステージがないため、測定は開発者本人が手元で行う。

## 準備

開発用の性能ハーネス（`src/dev/perfRows.ts`）は `import.meta.env.DEV` のときだけ有効になる。本番ビルドでは常に無効。

```
VITE_LOCAL_SIGHTS_PERF_ROWS=1000000 RUSTUP_TOOLCHAIN=stable npm run tauri dev
```

取得中の追加を模す場合と、Cloudscape の表の試作で測る場合の変数は README の「Performance harness」節を参照する。変数を変えたら dev を起動し直す。

## 測定項目と合格の条件

| ID | 操作 | 合格 |
|---|---|---|
| PERF-1 | 絞り込み欄に語を入れてから結果の件数が出るまで | 100 秒以内 |
| PERF-2 | ホイール 1 回・PageDown・End、行の展開から画面が変わるまで | 1 秒以内（目視またはストップウォッチ） |
| PERF-3 | 10 万行ずつの追加の間に、入力欄への入力が反映されるまで | 1 秒以内 |
| PERF-4 | 100 万行を表示したまま操作を続ける | アプリが落ちない |
| PERF-5 | 100 万行でステータス行が枠の下端に見え続ける（Step 18） | 常に見える |

## 記録先

`construction/code-generation/code-summary.md` の「性能測定」節の表に、数値・日時・機種を書く。すべて合格し、Cloudscape の表の試作でも合格した場合だけ、`LOG_TABLE_VARIANT` の既定を切り替えるかを決める。
