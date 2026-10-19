# Intent Capture Questions — Logs Insights 互換のクエリ（IB-11）

## Sources

- [desc] Initial description: "Logs Insights 互換のクエリ（IB-11）をやりたい。取得済みのログに対して、CloudWatch Logs Insights に似たクエリ（fields / filter / sort / limit / stats など）を書いて絞り込み・集計できるようにしたい。\n\n前提メモ：\n- project.md の Forbidden で、呼べる AWS API は DescribeLogGroups / DescribeLogStreams / GetLogEvents の 3 つだけ。StartQuery などの Insights の API は使えないので、クエリは手元（取得済みのログ）で実行する。\n- 今の部分一致の絞り込み（FR6）を発展させる位置づけ。261002-cwlogs-viewer の domain-design/components.md（225 行目付近）で、絞り込みは保持（EventTimeline）と別の部品にしてあり、差し替えられる作り。\n- 速さの基準は既存の 10 万件 10 秒以内・100 万件 100 秒以内（261002 の NFR1・NFR2）。\n- macOS と Windows の両方で動くこと（IB-16 で両 OS 対応済み、v0.2.0 はまだタグ未作成）。\n- 対応する構文の範囲は面談で決めたい。"
- [scope] Workflow-selected scope: `refactor`.
- [memory:M1] `aidlc/spaces/default/memory/project.md#Forbidden`: "NEVER CloudWatch Logs の読み取り API（DescribeLogGroups / DescribeLogStreams / GetLogEvents）以外の API を呼ぶ (affirmed 2026-10-03)"
- [memory:M2] `aidlc/spaces/default/memory/project.md#Forbidden`: "NEVER テレメトリやクラッシュレポートを外部に送る (affirmed 2026-10-03)"

## Q1. いまの絞り込みで、どんなときに困っていますか？

背景：今の絞り込み（FR6）は、メッセージに文字列を含む行だけを残す仕組みで、大文字と小文字は区別しません。どの不足がいちばん効いているかで、最初の版で優先する機能が変わります。

A. 条件を組み合わせられない（「ERROR を含み、かつ timeout を含まない」など）
B. JSON 形式のログの特定の項目（例：`level`、`status`）で絞り込みたい・並べ替えたい
C. 件数を数えたい・まとめたい（例：5 分ごとのエラーの件数、項目の値ごとの件数）
D. 上のいくつか（具体的に書いてください）
E. Not yet defined
X. Other (please specify)

[Answer]: A

## Q2. この機能の恩恵を受けるのは誰ですか？

背景：これまでの作業では、開発者本人・同じ会社の同僚・OSS として使う人の全員を利用者としてきました。今回も同じかを確かめます。

A. 開発者本人だけ
B. 開発者本人と、同じ会社でこのアプリを使う同僚
C. 開発者本人と、OSS として使う不特定の利用者
D. 上の全員（これまでと同じ）
E. Not identified
X. Other (please specify)

[Answer]: D

## Q3. 「できた」と言える目安は何ですか？

背景：成功の基準を測れる形で決めておくためです。数値や細かい条件は後のステージで詰めても構いません。

A. マネジメントコンソールの Logs Insights で書いていたクエリが、対応する範囲ではそのまま書けて、同じ結果になる
B. いつも確かめたいこと（例：時間帯ごとのエラーの件数）が、コンソールを開かずにアプリの中で分かる
C. クエリの実行が、今の絞り込みと同じ速さの基準（10 万件で 10 秒以内、100 万件で 100 秒以内）に収まる
D. 上のいくつか（具体的に書いてください）
E. Not yet defined
X. Other (please specify)

[Answer]: A

## Q4. なぜ今これに取り組むのですか？

背景：取り組むきっかけを記録します。最初の企画では、取得の高速化（IB-07）の次に取り組む項目としていました。

A. 最初の企画で決めた順番どおり（取得の高速化と Windows 対応が済んだので次に進む）
B. 実際に使っていて、今の絞り込みでは足りない場面が増えたから
C. コンソールの Logs Insights を使う手間（料金・画面の切り替え）を減らしたいから
D. 上のいくつか（具体的に書いてください）
E. Not identified
X. Other (please specify)

[Answer]: B

## Q5. 誰が決め、どう共有しますか？

背景：これまでは開発者本人が決め、共有は PR と README 程度でした。今回も同じかを確かめます。

A. これまでと同じ（本人が決め、共有は PR と README 程度）
B. 本人が決めるが、使い方の説明（クエリの書き方）を手順書や README に詳しく書く
C. 同僚の意見も聞いて決める
D. 上のいくつか（具体的に書いてください）
E. Not identified
X. Other (please specify)

[Answer]: A

## Q6. 最初の版で、どこまでのクエリの命令を使えるようにしますか？

背景：Logs Insights には `fields`・`filter`・`sort`・`limit`・`stats`・`parse`・`dedup` などの命令があります。最初の版でどこまでを対象にするかの大枠を決めます。細かい書き方（使える関数や演算子）は要件分析で決めます。

A. `fields`・`filter`・`sort`・`limit` だけ（行を絞り込む・並べる）
B. A に加えて `stats`（件数・合計・平均などの集計と、`by` によるまとめ）
C. B に加えて `parse`（メッセージから項目を取り出す）
D. Logs Insights の命令をできるだけすべて
E. Not yet defined（要件分析で決める）
X. Other (please specify)

[Answer]: A

## Q7. 今の部分一致の絞り込みは、どう扱いますか？

背景：今の絞り込みの入力欄は、文字を打つだけで手軽に絞り込めます。クエリと並べて残すか、クエリに置き換えるかで、画面と使い方が変わります。

A. 今の絞り込みを残し、クエリは別に書けるようにする（手軽な絞り込みとクエリの両方を使える）
B. 今の絞り込みをクエリに置き換える（入力欄は 1 つ）
C. 今の入力欄に、文字だけならこれまでどおりの部分一致、クエリならクエリとして扱う
D. Not yet defined（画面の設計で決める）
E. Not applicable
X. Other (please specify)

[Answer]: A

## Q8. 今回の作業の進め方は `refactor` を元にした計画に決まっています。最初の版に含めないものはどれですか？（select all that apply）

背景：作業の進め方はもう決まっています。そのうえで、この版で作らないものを決めます。選ばなかった項目は、含めるかどうかを後で確かめます。

A. 集計結果のグラフ表示（表だけにする）
B. クエリの保存や履歴
C. 取得していないログへのクエリ（クエリのために AWS から追加で取得すること）
D. クエリの結果をファイルに書き出すこと
E. None（特に決めていない）
X. Other (please specify)

[Answer]: B, D

## Q9. 集計（`stats`）は、最初の版に含めますか？（Q6 の確認）

背景：依頼文には「絞り込み・集計できるようにしたい」とあり、命令の例に `stats` も挙がっていました。一方、Q6 では最初の版を `fields`・`filter`・`sort`・`limit`（行の絞り込みと並べ替え）だけにすると答えています。どちらを優先するかを確かめます。

A. 最初の版には含めず、次の版以降に回す（今回は行の絞り込みと並べ替えだけ）
B. 最初の版に含める（Q6 の答えを B に改める）
C. 件数を数えることだけ（`stats count(*)` と `by`）は最初の版に含める
D. Not yet defined（要件分析で決める）
E. Not applicable
X. Other (please specify)

[Answer]: A

## Q10. 取得していないログに対するクエリ（クエリのために AWS から追加で取得すること）は、最初の版に含めますか？（Q8 で選ばれなかった項目の確認）

背景：Q8 ではこの項目を「含めないもの」に選んでいません。含めるとも含めないとも決めつけずに確かめます。依頼文では、クエリは取得済みのログに対して手元で実行する前提でした。

A. 含めない（クエリは取得済みのログだけに対して実行する）
B. 含める（クエリに合わせて足りないログを取得する）
C. Not yet defined（要件分析で決める）
D. Not applicable
E. None
X. Other (please specify)

[Answer]: B

## Q11. 「足りないログを取得する」とは、どんな動きを考えていますか？（Q10 の確認）

背景：Q10 で、クエリに合わせて足りないログを取得することを最初の版に含めると答えています。どんなときに、何を追加で取得するのかで、作る範囲が大きく変わります。なお、追加の取得も呼べる API は DescribeLogGroups / DescribeLogStreams / GetLogEvents の 3 つだけで、クエリそのものは手元で実行します。

A. クエリを実行すると、今の条件（プロファイル・リージョン・ロググループ・期間）でまだ取得していなければ取得し、そのログに対して手元でクエリを実行する（取得とクエリを 1 回の操作にする）
B. クエリの中で期間やロググループを指定でき、取得済みの範囲に足りない分だけを追加で取得する
C. `limit` の件数に届くまで、期間をさかのぼって取得を続ける
D. 上のいくつか（具体的に書いてください）
E. Not yet defined（要件分析で決める）
X. Other (please specify)

[Answer]: B
