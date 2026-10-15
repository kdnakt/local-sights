# Security Design — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/nfr-requirements/`（NFR2.2〜NFR9.2、レビュー 2 回目の新しい指摘）、`construction/u4-elapsed-time/functional-design/functional-spec.md`（BR4.1〜BR4.10）、`inception/contract-design/contract-summary.md`（C5）、今のコード（`crates/local-sights-core/src/session.rs`、`src/api.ts`、`src/App.tsx`、`src/components/StatusLine.tsx`、`src/i18n/messages.ts`）。質問票は `nfr-design-questions.md`。

この文書は U4 の設計の正本も兼ねる（ほかの設計の文書はここを参照する）。U4 は秘密に触れないため、守りの設計は「渡すものを数値に限る」「何も足さない」の 2 点で足りる。

## 1. 守りの設計

| 要件 | 仕組み |
|------|--------|
| NFR8.10 | SessionView の `elapsed` は `{ running: boolean, elapsedMs: number } \| null` の形に固定する。ライブラリ側の型は真偽値と符号なし整数（ミリ秒）だけを持ち、文字列の欄を持たない |
| NFR8.11 | AWS の呼び出し・依存（Cargo.toml・package.json）・ログの出力・ファイルの書き込みを足さない。経過の記録は AppSession のメモリの中だけ |
| NFR6.2 | ライブラリの時計と画面の時計は差し替えられる。テストは手で進める時計と Vitest の偽のタイマーを使い、実際の AWS に接続しない |

## 2. 設計の正本

### 2.1 ライブラリ（`crates/local-sights-core/src/session.rs`）

- **時計の型**：「今」を返す小さな型 `SessionClock` を作る。中身は「今を返すもの」を共有する参照で、本番は戻らない時計（`std::time::Instant::now`）、テストは手で進める時計。AppSession と同じく複製でき（Clone）、表示でき（Debug、中身は出さない）、別のタスクと共有できる（Send・Sync）。`AppSession::new()` などは本番の時計を使い、`with_clock(clock)` で差し替える
- **記録**：AppSession に `elapsed: Option<ElapsedRecord>` を持たせる。`ElapsedRecord { started_at: Instant, final_ms: Option<u64> }`
- **書き換える場所**（BR4.1〜BR4.4）：
  - `begin_fetch` が受け入れたとき（`self.phase = Phase::Fetching` の直後）：`Some(ElapsedRecord { started_at: 今, final_ms: None })`
  - `finish_fetch` で Aborted のとき：`None`
  - `finish_fetch` で Aborted 以外の終わり（今の判定を通った後）：`final_ms = 今 − started_at`（飽和、ミリ秒）。すでに final_ms があれば変えない
  - `abort_fetch_with_failure` が失敗として終えるとき：上と同じく final_ms を決める
  - `apply_connection_change`：`None`
- **画面へ渡す形**：`SessionView.elapsed: Option<ElapsedView>`、`ElapsedView { running: bool, elapsed_ms: u64 }`（serde の camelCase で `elapsedMs`）。`view()` の中で、final_ms があれば `{ running: false, elapsed_ms: final_ms }`、なければ `{ running: true, elapsed_ms: 今 − started_at }`
- **送り方**：今の `session-changed`・`getSession`・操作の応答で SessionView を返す場所はそのまま。`FetchProgressUpdate` は変えない（NFR5.4）

### 2.2 画面

- **型**（`src/api.ts`）：`ElapsedView = { running: boolean; elapsedMs: number }`、`SessionView.elapsed: ElapsedView | null`
- **純粋な関数**（新しい `src/elapsed.ts`、テスト先行、NFR6.12）：
  - `formatElapsed(ms)`：BR4.7 の表記（負は 0、秒未満切り捨て、mm:ss / h:mm:ss）
  - `nextAnchor(previous, incoming, nowMs)`：合わせ直しの判断。起点 `Anchor = { baseMs, receivedAt, running }` を返す
    - `incoming` が null → null
    - `incoming.running` が false → `{ baseMs: incoming.elapsedMs, receivedAt: nowMs, running: false }`
    - `incoming.running` が true で、`previous` が running → `baseMs = max(incoming.elapsedMs, shownMs(previous, nowMs))`（保持、NFR6.6）
    - `incoming.running` が true で、`previous` が null か running でない → `baseMs = incoming.elapsedMs`（新しい取得の始まり）
  - `shownMs(anchor, nowMs)`：running なら `baseMs + max(0, nowMs − receivedAt)`、でなければ `baseMs`
  - `msUntilNextSecond(anchor, nowMs)`：表示の秒が次に変わるまでのミリ秒（`1000 − shownMs % 1000`）
- **合わせ直しの契機**（NFR6.9）：`session.elapsed` のオブジェクトが入れ替わったとき。バックエンドからの SessionView は毎回新しいオブジェクトを持ち、`withProgress`・`withFilter`・`mergeSessionView` の合成（`{...view}`）は同じオブジェクトを引き継ぐ
- **新しい取得の見分け方**：ライブラリは取得中に次の取得を受け付けず（`begin_fetch` は Fetching のとき Busy）、終わり・異常な停止・閉じたときは running でない値か空を送る。そのため running のまま別の取得が始まる経路はなく、`previous` が running でないことで新しい取得の始まりと分かる（非機能要件のレビュー 2 回目の指摘）
- **時計のフック**（新しい `src/hooks/useElapsed.ts`）：引数は `elapsed`、戻り値は表示するミリ秒か null。`elapsed` のオブジェクトが変わったら `nextAnchor` で起点を置き換え、running の間は `msUntilNextSecond` 後に描き直しを予約する（`setTimeout`）。running でなくなったとき、空になったとき、部品が外れたときは予約を消す。時刻は `performance.now()`
- **部品**（新しい `src/components/ElapsedTime.tsx`）：`useElapsed` を使い、`<span data-testid="status-line-elapsed">` に `t("status.elapsed", { time })` を出す。running の間は `aria-hidden="true"`。null なら何も出さない。毎秒の描き直しはこの部品の中だけで起きる（NFR5.3）
- **置き場所**（`src/components/StatusLine.tsx`）：`renderStatus` → `renderCache` → `<ElapsedTime>` → `renderFilter` の順。段階が Idle のとき、`session` が null のときは出さない
- **文言**（`src/i18n/messages.ts`）：`status.elapsed` を英語「Elapsed {time}」、日本語「経過 {time}」で足す

## 3. 確かめること

| 確かめること | どこで | 要件 |
|--------------|--------|------|
| 始めて 31.4 秒後に終わる → `{ running: false, elapsedMs: 31400 }`、その後に時計を進めても同じ | session.rs のテスト | NFR2.2、NFR6.11 |
| 始めて 7 秒後の view → `{ running: true, elapsedMs: 7000 }` | session.rs のテスト | BR4.5 |
| Aborted で null、接続の変更で null、ロググループの選び直しで残る、起動直後は null | session.rs のテスト | NFR6.10 |
| 取得中に begin_fetch がもう一度来ても始まりは変わらない（Busy） | session.rs のテスト | BR4.1 |
| 2 回目の取得を始めると 1 回目の最終の値が消える | session.rs のテスト | NFR6.4 |
| abort_fetch_with_failure の後は running false | session.rs のテスト | NFR6.11 |
| 時計の差が負になる場合は 0 | session.rs のテスト（時計を戻す） | NFR6.8 |
| SessionView を JSON にすると `elapsed` が `{"running":…,"elapsedMs":…}` か `null` | session.rs のテスト | NFR8.10 |
| FetchProgressUpdate の項目が今と同じ | 既存のテストのまま | NFR5.4 |
| 表記の境目（0、999、7,999、3,599,999、3,600,000、75 分、10 時間、100 時間、負） | `src/elapsed.test.ts`（テスト先行） | NFR6.3、AC2.1.5 |
| 合わせ直し：running→running で小さい値は保持、新しい取得の 0 で 00:00、running false で置き換え、null で消える | `src/elapsed.test.ts`（テスト先行） | NFR6.6 |
| 1 回受け取って 16 秒進めると 16 秒分進む。running false の後は進まない | `ElapsedTime` のテスト（偽のタイマー） | NFR6.5 |
| fetch-progress を合成しても戻らない。getSession の running で時計が始まる | `StatusLine`・`App` のテスト | NFR6.9 |
| 並び・英日の文言・Idle で出ない | `StatusLine` のテスト | NFR9.1、BR4.8 |
| running の間は `aria-hidden`、終わったら属性なし（終わりの表示と同じ描き直し） | `StatusLine` のテスト | NFR9.2 |
| 毎秒の描き直しが経過の部品だけ | `ElapsedTime` を含む描画回数のテスト | NFR5.3 |

## Assumptions & Open Questions

None.
