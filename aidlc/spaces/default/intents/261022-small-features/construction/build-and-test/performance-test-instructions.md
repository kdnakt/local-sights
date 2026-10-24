# Performance Test Instructions — アプリ内の言語切替・書き出し・JSON の整形（261022-small-features）

この計画（refactor）には NFR Requirements と NFR Design の段がなく、要件 NFR2 は「書き出しと JSON の整形で画面が固まらない。処理中も画面はスクロールと画面の切り替えに応じる。大きな行数での処理時間の目標値は決めない（上限を置かず時間がかかることを受け入れる、Q3 A）」と定めている。したがって数値の目標はなく、ここで確かめるのは「固まらない」ことの仕組みと、その自動テストである。

## 1. 目標と確かめ方

| 目標 | 由来 | 確かめ方 | 自動 / 手元 |
|---|---|---|---|
| 書き出しの処理中も画面が応じる | NFR2、FR4.6.3、BR4.8 | 錠の中では行の複製だけを行い、文字への変換と書き込みは錠を離した blocking スレッドで行う（U2 レビュー R-01）。`start_export` が `spawn_blocking` を使い、`finish_export` まで錠を持たないことをコードで確かめる。`App.test.tsx` の `export` の describe で、Running の間もメニュー以外の操作が無効になっていないことを確かめる | 自動（構造の確認とテスト） |
| 100 万行で画面が固まらない | NFR2 | 開発者の手元で 100 万行を取得し、CSV で保存する間にスクロールと画面の切り替えができることを目で確かめる。時間は記録するが合否の数値はない | 手元 |
| JSON の整形で画面が固まらない | NFR2、BR5.1（上限） | `jsonPretty.test.ts` の上限のテスト（深さ 256・候補 64・1 MiB）。U2 のコードレビュー Minor 2（候補をまたいだ走査量の合計に上限がない。最悪 64 × 1 MiB）は次の作業の候補 | 自動（上限の存在）、改善は持ち越し |
| 言語の切り替えで画面を作り直さない | FR1.5、BR1.3 | `App.test.tsx` の `switches the texts to Japanese…` が、取得済みの行・入力の文字が残ることを確かめる | 自動 |

## 2. 流し方

```bash
cargo test -p local-sights-core export
npx vitest run src/jsonPretty.test.ts src/App.test.tsx
```

## 3. 今の速さの基準

この作業で速さの基準（ベンチマーク）は足していない。前の作業（261017-insights-query）の基準（`cargo test -p local-sights-core` の中の速さのテスト 5 本）は変えていない。

## 4. 手元の記録の仕方

100 万行の書き出しは、OS・行数・形式・出し方・かかった時間・処理中にスクロールと画面の切り替えができたかを `test-results.md` に 1 行で足す。ログの中身は残さない。
