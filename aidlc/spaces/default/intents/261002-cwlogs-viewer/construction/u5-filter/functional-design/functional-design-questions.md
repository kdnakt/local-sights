# Functional Design 質問票 — U5 絞り込み（u5-filter）

上流の成果物：`inception/units-generation/unit-of-work.md`（U5 の範囲）、`unit-of-work-story-map.md`、`inception/requirements-analysis/requirements.md`（FR6.1〜FR6.5、NFR1、NFR2）、`inception/domain-design/components.md`（FilterEngine・FilterCondition・FilterResult・AppSession・DesktopUi）。

U5 は、取得済みのログをメッセージの部分一致で絞り込む。AWS の API は呼ばない。絞り込み中は「絞り込み後の件数 / 全件数」を出し、取り直しても絞り込み文字列を引き継ぐ。10 万件で 10 秒以内、100 万件で 100 秒以内。絞り込み結果の行だけを、U3 の表示範囲の取り寄せと同じ形で返す。

前の作業単位から U5 に持ち越した点はない。

ここでは U5 で決める必要のある振る舞いだけを確認する。

## Q1. 絞り込みで大文字・小文字を区別しますか？

背景：FR6.5 は Functional Design で決めるとしています。U2 のロググループ一覧の絞り込みは、大文字・小文字を区別しない部分一致にしました。

A. 区別しない（`error` で `ERROR` も `Error` も見つかる。U2 と同じ）
B. 区別する（入力したとおりの文字だけを見つける）
C. 区別するかどうかを切り替えられるようにする（既定は区別しない）
X. Other (please specify)

[Answer]: A. 区別しない（`error` で `ERROR` も `Error` も見つかる。U2 と同じ）

## Q2. 絞り込みは、いつかけますか？

背景：100 万件では 1 回の絞り込みに時間がかかることがあります（NFR2 は 100 秒以内）。

A. 入力するたびに、打ち終わってから少し（0.3 秒ほど）待ってかける
B. Enter キーを押したときだけかける
X. Other (please specify)

[Answer]: A. 入力するたびに、打ち終わってから少し（0.3 秒ほど）待ってかける

## Q3. 取得中に絞り込み文字列が入っているとき、取得したログをどう表示しますか？

背景：U3 では取得中もログが逐次一覧に加わります。FR6.4 は、取り直しても絞り込み文字列を引き継ぐと決めています。

A. 取得中も、届いたログに逐次同じ絞り込みをかけて、合うものだけを一覧に加える（件数も「絞り込み後 / 全件」で増えていく）
B. 取得中は絞り込みをかけずに全件を出し、取得が終わってから絞り込む
X. Other (please specify)

[Answer]: A. 取得中も、届いたログに逐次同じ絞り込みをかけて、合うものだけを一覧に加える（件数も「絞り込み後 / 全件」で増えていく）

## Q4. 大量のログで絞り込みに時間がかかっているあいだ、入力をどう扱いますか？

背景：絞り込み中に文字列を打ち直すことがあります。

A. 絞り込み中であることをステータス行に出し、入力は止めない。新しい文字列が入ったら、前の絞り込みはやめて新しい文字列でかけ直す
B. 絞り込みが終わるまで入力欄を使えないようにする
X. Other (please specify)

[Answer]: A. 絞り込み中であることをステータス行に出し、入力は止めない。新しい文字列が入ったら、前の絞り込みはやめて新しい文字列でかけ直す

## Consolidated Summary Confirmation

回答のまとめ：

- 大文字・小文字は区別しない部分一致（Q1）
- 入力するたびに、打ち終わってから 0.3 秒ほど待って絞り込む（Q2）
- 取得中も届いたログに逐次同じ絞り込みをかけ、件数は「絞り込み後 / 全件」で増えていく（Q3）
- 絞り込み中は入力を止めず、ステータス行に絞り込み中と出す。新しい文字列が入ったら前の絞り込みをやめてかけ直す（Q4）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
