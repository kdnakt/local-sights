# Initiative Brief — Logs Insights 互換のクエリ（IB-11）

上流：`ideation/intent-capture/intent-statement.md`（intent-statement）、`ideation/intent-capture/stakeholder-map.md`（stakeholder-map）、`ideation/scope-definition/scope-document.md`（scope-document）、`ideation/scope-definition/intent-backlog.md`（intent-backlog）。この段の回答は `approval-handoff-questions.md` の Q1〜Q6 を指す。

## 意図と課題

いまの絞り込みは、メッセージに指定した文字列を含む行だけを残す仕組みで、条件を 1 つしか書けない。いちばん困っているのは、条件を組み合わせられないこと（例：「ERROR を含み、かつ timeout を含まない」）である。この作業は、CloudWatch Logs Insights に似たクエリで、取得したログを絞り込めるようにする。今の部分一致の絞り込みを発展させる位置づけで、クエリは手元で実行する。（intent-statement の Problem Statement）

利用者は開発者本人・同じ会社の同僚・OSS の利用者で、範囲と優先順位は開発者本人が決める。進捗の共有は GitHub の PR と README 程度で足りる。（stakeholder-map）

## 市場での裏付け

市場調査は、この計画では行っていない。実際に使っていて今の絞り込みでは足りない場面が増えたことがきっかけで、利用者の困りごとは開発者本人の経験に基づく。（intent-statement の Initiative Trigger）

## 実現性とリスク

実現性の検討は、この計画では行っていない。前提の制約と、この段で確かめたリスクは次のとおり。

| 区分 | 内容 | 扱い | 出典 |
|------|------|------|------|
| 制約 | 呼べるのは読み取りの 3 つ（DescribeLogGroups / DescribeLogStreams / GetLogEvents）だけ。StartQuery などは使わず、クエリは手元で実行する。追加の取得も同じ 3 つで行う | 変えない | scope-document の「変えないもの」、project.md の Forbidden |
| 制約 | 速さの基準（10 万件で 10 秒以内、100 万件で 100 秒以内）と、macOS・Windows の両方で動くこと | 変えない | scope-document の「変えないもの」 |
| リスク | JSON の項目の取り出しや条件の組み合わせを手元で実行しても、速さの基準に収まるかはまだ確かめていない | 受け入れる。要件で速さの条件を決め、作る段階で測る | Q1 |
| リスク | クエリの中で期間やロググループを指定すると、足りない分の追加の取得が走り、待ち時間が長くなることがある | 受け入れる。今の取得と同じく、処理中の表示と中断を使えるようにする（B5） | Q1 |
| リスク | この作業が終わるまで `v0.2.0` のタグを付けないため、取得の高速化（PR #22）と Windows 版を配るのも遅れる | 受け入れる | Q1 |
| リスク | コンソールの Logs Insights との互換を、どのクエリで確かめ、何を同じとみなすかが決まっていなかった | 受け入れず、この段で確かめ方を決めた（次の節） | Q1a |

## コンソールとの互換の確かめ方

scope-document の「成功の判断」で後回しにしていた、互換の確かめ方をこの段で決めた。

| 項目 | 決めたこと | 出典 |
|------|------------|------|
| 確かめるクエリの組 | AWS の資料にあるクエリの例のうち、このアプリが対応する範囲（B6 の一覧。`stats` を使わないもの）のものを使う | Q4 |
| 「同じ結果」の意味 | 結果に出る行（ログイベント）の集まりが同じであること。並び順は、クエリに `sort` を書いたときだけ比べる | Q5 |
| 比べ方 | 同じロググループ・同じ期間について、コンソールで実行して書き出した結果と、同じログに対するアプリの結果を、手元の確認用プログラムで自動で比べる | Q6 |

この確かめは開発者本人の手元で、本人の AWS 認証情報を使って行う。自動テストと CI からは実際の AWS に接続しない。（team.md の Walking Skeleton、project.md の Forbidden）

## 範囲の境界

| 区分 | 項目 | 出典 |
|------|------|------|
| Must | B1 条件の組み合わせ（`filter`）、B2 `fields`、B3 決まった項目と JSON の項目（入れ子は `a.b`）、B4 `sort`・`limit`、B5 クエリの中での期間とロググループ（複数可）の指定と足りない分だけの追加の取得、B6 README の対応範囲の一覧、B7 対応外の命令・関数を知らせる画面のエラー | scope-document、intent-backlog |
| Should（時間が足りなければ次の版） | B8 入力の補完、B9 構文の色分け、B10 例のクエリ | scope-document、intent-backlog |
| 含めない | 集計（`stats`。次の版以降）、クエリの保存・履歴、結果の書き出し、クエリの書き方の説明 | scope-document の「範囲外」 |
| 保つ | 今の部分一致の絞り込みは残し、クエリは別に書けるようにする | scope-document の「変えないもの」 |

使える関数や演算子の細かい範囲は、要件分析で決める。（scope-document）

## 画面のイメージ

ラフな画面設計は、この計画では行っていない。画面の形は、この後の画面の詰め（Refined Mockups）で決める。

## 体制と進める順

開発者は 1 人で、チーム編成は行っていない。PR のレビューはセルフレビューで、CI が通ってから `main` にスクワッシュマージする。（team.md の Way of Working）

進める順は価値が先とし、最初の一本は、条件を組み合わせた `filter`（B1）を取得済みのログに対して手元で実行し、結果を表示するところまでとする。そのあと B3・B2 → B4 → B5 → B7・B6 → B8〜B10 の順に足す。細かい分け方は作業単位の分割で決める。（intent-backlog の「進める順」）

## 配る版

期限はない。この作業も `v0.2.0` に含め、終わるまで `v0.2.0` のタグを待つ。`v0.2.0` は、取得の高速化（PR #22）・Windows 版・この作業を合わせて配る版になる。team.md の Deployment の記述（今は PR #22 と Windows 版だけ）は、この段の終わりの「次のために覚えておくこと」で、この作業を含む形に書き足す。（scope-document の「期限と配る版」、Q2）

## 判断

**Go**：このまま次のフェーズ（要件定義・設計）に進める。次は、今のコードの読み直し（Reverse Engineering）から始まる。（Q3）

## Assumptions & Open Questions

None.
