# NFR Design 質問票 — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/nfr-requirements/`（NFR2.2〜NFR9.2、レビュー 2 回目の新しい指摘）、`construction/u4-elapsed-time/functional-design/functional-spec.md`、`inception/contract-design/contract-summary.md`（C5）、今のコード（`crates/local-sights-core/src/session.rs`、`src/api.ts` の mergeSessionView・withProgress・withFilter、`src/App.tsx`、`src/components/StatusLine.tsx`、`src/i18n/messages.ts`）。

U4 の非機能の決まりは前の段で利用者と決め終えており、ここで決めるのは実現の形だけである。新しく利用者に聞くことはない。

## 前提として決めたこと（質問にしない技術的な決め）

- **合わせ直しの見分け方（NFR6.9）**：画面は `elapsed` のオブジェクトが入れ替わったときに合わせ直す。バックエンドから届く SessionView（`session-changed`、`getSession`、操作の応答）は毎回新しい `elapsed` を持ち、`fetch-progress`・`filter-progress` の合成（`{...view}`）は前の `elapsed` をそのまま引き継ぐため、オブジェクトが同じかどうかで区別できる。
- **新しい取得の見分け方（NFR6.6、非機能要件のレビュー 2 回目の指摘）**：保持（減らさない）は、表示中が running で、受け取ったものも running のときだけ。ライブラリは取得中に次の取得を受け付けず（今の begin_fetch）、取得の終わりと閉じたときは必ず running でない値か空を送るので、running のまま別の取得が始まる経路はない。
- **ライブラリの時計（NFR6.2、NFR6.8）**：「今」を返す小さな時計の型を AppSession に持たせ、組み立てるときに差し替えられるようにする。本番は Rust 標準の戻らない時計、テストは手で進める時計。AppSession と同じく複製でき、別のタスクと共有できる。差は負にならない計算（飽和）で取る。
- **画面の時計（NFR5.3、NFR6.5）**：経過だけを描く小さな部品（ElapsedTime）が自分の中で時計を持ち、表示の秒が変わる時刻に合わせて次の描き直しを予約する。状態行の残りとログの表は毎秒描き直さない。
- **純粋な関数（NFR6.12）**：表記、合わせ直しと保持の判断、表示する値の計算は、副作用のない関数として切り出し、テストを先に書く。

## Consolidated Summary Confirmation

- ライブラリ：AppSession に経過の記録（始まりと最終の値）と差し替えられる時計を持たせる。取得を始めると記録、終わり・異常な停止で最終の値、閉じて止めたときと接続の変更で消す。SessionView に `elapsed`（running と elapsedMs）を足す
- 画面：経過だけを描く小さな部品が毎秒の描き直しを受け持ち、状態行の取得の表示とキャッシュの表示の後ろ、絞り込みの表示の前に置く。取得中は読み上げの対象から外す
- 合わせ直しは `elapsed` のオブジェクトが入れ替わったとき（バックエンドから新しい SessionView が届いたとき）だけ。取得中は数字を戻さず、新しい取得の始まり・終わり・空では受け取った値に置き換える
- 表記・合わせ直し・表示する値は純粋な関数にしてテストを先に書く。英日の文言を足す
- AWS の呼び出し・依存・ログ・保存先は変えない。`elapsed` は真偽値と数値だけ

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
