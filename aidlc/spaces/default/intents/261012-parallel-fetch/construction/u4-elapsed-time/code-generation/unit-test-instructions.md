# Unit Test Instructions — U4 かかった時間（u4-elapsed-time）

## テストの道具と設定

- ライブラリ：Rust の標準のテスト。AppSession の時計は `SessionClock` の手で進める版に差し替える
- 画面：Vitest（jsdom、globals、`src/test/setup.ts`）と Testing Library。時計は Vitest の偽のタイマー（`vi.useFakeTimers()`、`performance.now` も偽にする）と、`useElapsed` の今の時刻の関数の差し替え
- 新しいテストの道具・設定ファイルは足さない

## この作業単位のテストの走らせ方

最初のテスト先行の手順（Step 3）より前に、`cargo test -p local-sights-core --lib session::` と `npx vitest run src/components/StatusLine.test.tsx` が動くことを確かめる（Step 2）。

| 対象 | コマンド |
|------|----------|
| AppSession の経過 | `cargo test -p local-sights-core --lib session::` |
| 表記と合わせ直し | `npx vitest run src/elapsed.test.ts` |
| 経過の部品 | `npx vitest run src/components/ElapsedTime.test.tsx` |
| 状態行と App | `npx vitest run src/components/StatusLine.test.tsx src/App.test.tsx` |
| 文言の一覧 | `npx vitest run src/i18n/messages.test.ts` |
| すべて（画面） | `npm run test:coverage` |

## カバレッジの目標

- ライブラリ（local-sights-core）の行カバレッジ 80% 以上（`cargo llvm-cov -p local-sights-core --fail-under-lines 80`）
- 画面の行カバレッジ 80% 以上（`npm run test:coverage`、`vitest.config.ts` の閾値）
- 下限は下げない。新しい公開の項目（`SessionClock`・`with_clock`・`ElapsedView`・`formatElapsed`・`nextAnchor`・`shownMs`・`msUntilNextSecond`）には必ずテストを書く（team.md）
- テストの量は Standard：部品ごとに 5〜8 件ほど、主な境目（AppSession → SessionView → 画面）にテスト

## 偽物と差し替え

- 実際の AWS には接続しない（NFR6.2）。U4 のテストは AWS を呼ばない
- ライブラリの時計は手で進める。画面の時計は偽のタイマーで進める
- 届く順の崩れは、`nextAnchor` に番号と running の組み合わせを並べて与えて確かめる

## テストのデータ

- 経過は 0、7,000、31,400 ミリ秒などの決まった値。境目は 999、59,999、3,599,999、3,600,000 ミリ秒
- 取得の番号は 1、2 のような小さい数。秘密に見える値は入れない
