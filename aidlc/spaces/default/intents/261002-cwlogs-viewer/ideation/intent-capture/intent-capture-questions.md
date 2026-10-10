# Intent Capture 質問票

## Sources

- [desc] Initial description: "ローカルで動くRust製クライアントアプリ。CloudWatch LogsのGetLogEvents APIを使ってロググループ/ログストリームのログを閲覧する（マネコンのFilterLogEventsやLogs Insightsでは古いロググループの過去ログが見えないケースがあるため）。まずはログ表示とフィルター、最終的にはLogs Insightsのような複雑なクエリも実装したい。"
- [scope] Workflow-selected scope: `local-tool`.

## Q1. 解決したい一番の課題はどれですか？

背景：説明では「マネジメントコンソール（FilterLogEvents）では古いロググループの過去ログが見えないが、CLIの get-log-events では見えた」とあります。このツールが何を一番解決するかで、MVPの優先順位が変わります。

A. 古いロググループの過去ログを、マネコンで見えなくても確実に閲覧できること
B. CLIで毎回 get-log-events を叩く手間（ページング・トークン管理・JSON読解）をなくすこと
C. A と B の両方（同程度に重要）
D. ローカルで高速にログを絞り込み・分析できること（閲覧可否より分析体験が主）
E. Not yet defined
X. Other (please specify)

[Answer]: A. 古いロググループの過去ログを、マネコンで見えなくても確実に閲覧できること

## Q2. 主な利用者は誰ですか？

背景：利用者によって、配布形態・認証情報の扱い・UIの作り込み度合いが変わります。

A. 自分ひとり（個人の作業用ツール）
B. 自分＋所属チームの開発者・運用者
C. 自分で使いつつOSSとして公開し、不特定の利用者にも使ってもらう
D. 社内の複数チームに配布する
E. Not yet defined
X. Other (please specify)

[Answer]: C. 自分で使いつつOSSとして公開し、不特定の利用者にも使ってもらう

## Q3. 成功をどう判断しますか？（select all that apply）

背景：後続の要件・テストで「できた」と言える基準にします。測れる形が望ましいです。

A. マネコンで見えなかった特定の古いロググループの過去ログが、このツールで表示できる
B. 指定した期間・ストリームのログを、CLIより少ない操作で表示・フィルタできる
C. 数万〜数十万件規模のイベントでも実用的な時間でフィルタ結果が返る
D. 日常的なログ調査で、マネコン／CLIの代わりにこのツールを使うようになる
E. Not yet defined
X. Other (please specify)

[Answer]: A, B, C

## Q4. なぜ今このツールを作るのですか？

背景：きっかけが「実際の障害調査で困った」のか「学習・趣味」なのかで、スピード優先か作り込み優先かが変わります。

A. 実務の調査で今まさに困っており、早く使えるものが欲しい
B. Rust の学習・技術検証を兼ねた個人プロジェクト
C. A と B の両方
D. AI-DLC による開発プロセスを試すこと自体も目的のひとつ
E. Not applicable
X. Other (please specify)

[Answer]: A. 実務の調査で今まさに困っており、早く使えるものが欲しい

## Q5. 利用者以外に関係者（ステークホルダー）はいますか？

背景：例えばログを持つAWSアカウントの管理者、セキュリティ担当、レビューしてくれる同僚など。いない場合はそのまま「None」を選んでください。

A. None（自分だけで完結する）
B. AWSアカウント／IAMの管理者（読み取り権限の付与などで関わる）
C. 一緒に使う・レビューするチームメンバー
D. セキュリティ・コンプライアンス担当（ログ内容の取り扱いに関心がある）
E. Not identified
X. Other (please specify)

[Answer]: A. None（自分だけで完結する）

## Q6. スコープや優先順位は誰が決めますか？

背景：要件の優先度判断を誰に確認すべきかを明確にします。

A. 自分ひとりで決める
B. 自分が決めるが、チームの意見も参考にする
C. 上長・プロダクトオーナーなど他の人が決める
D. Not yet defined
X. Other (please specify)

[Answer]: A. 自分ひとりで決める

## Q7. 進捗報告や共有の必要はありますか？

背景：報告の有無で、成果物の書き方（他人向けの説明が要るか）が変わります。

A. None（不要）
B. GitHub上のPR・README程度で十分
C. 定期的にチームへ進捗共有する
D. Not yet defined
X. Other (please specify)

[Answer]: B. GitHub上のPR・README程度で十分

## Q8. このワークフローは `local-tool`（ローカルで動く読み取り専用ツール、デプロイ・運用ステップなし）として始めています。製品の範囲はこれで合っていますか？

背景：選んだ進め方と、あなたが考える製品の範囲がずれていないかの確認です。

A. 合っている：ローカルで動くクライアントアプリで、AWSへのデプロイは不要
B. 範囲は合っているが、将来はリリースバイナリ配布（GitHub Releases や crates.io）もしたい
C. 範囲が違う：ローカルツールではなく、サーバー／Webサービスとして作りたい
D. 範囲が違う：その他の形（範囲を具体的に記入）
E. Not yet defined
X. Other (please specify)

[Answer]: A. 合っている：ローカルで動くクライアントアプリで、AWSへのデプロイは不要

## Q9. 実際に観測した事象として、次の内容で合っていますか？

背景：最初の相談で「Logs Insights が使えない古いロググループをマネコンで見たところ、過去にあったログが閲覧できなくなっていた（マネコンは FilterLogEvents を使用）。ログ削除の記録はなく、CLI から get-log-events を使うと過去のログが閲覧できた」と伺いました。成果物の課題記述の根拠として確認済みの事実にしたいため、確認させてください。原因（保持期間、取り込み時刻、API の仕様差など）はまだ特定していない前提です。

A. 合っている：マネコン（FilterLogEvents）では見えず、CLI の get-log-events では見えた。ログ削除の記録はない。原因は未特定
B. 概ね合っているが補足がある（補足を記入）
C. 違う（実際の事象を記入）
D. Not yet defined
X. Other (please specify)

[Answer]: A. 合っている：マネコン（FilterLogEvents）では見えず、CLI の get-log-events では見えた。ログ削除の記録はない。原因は未特定

## Q10. MVP と将来目標の境界はいつ確定させますか？

背景：説明では「まずはログ表示とフィルター、最終的には Logs Insights のような複雑なクエリ」とありますが、MVP にどこまで入れるかはまだ確認していません。

A. このステージでは確定させず、スコープ定義ステージで確認する
B. 今ここで決める：MVP は「ログ表示＋フィルター」まで、Insights 風クエリは将来
C. Not yet defined
X. Other (please specify)

[Answer]: A. このステージでは確定させず、スコープ定義ステージで確認する

## Consolidated Summary Confirmation

回答のまとめ：

- 一番の課題：古いロググループの過去ログを、マネコンで見えなくても確実に閲覧できること（Q1）
- 主な利用者：自分で使いつつOSSとして公開し、不特定の利用者にも使ってもらう（Q2）
- 成功基準：古いログが表示できる／CLIより少ない操作で表示・フィルタできる／数万〜数十万件でも実用的な時間でフィルタできる（Q3）
- きっかけ：実務の調査で今まさに困っており、早く使えるものが欲しい（Q4）
- 利用者以外の関係者：None（Q5）
- スコープ・優先順位の決定者：自分ひとり（Q6）
- 進捗共有：GitHub上のPR・README程度（Q7）
- 製品範囲：ローカルで動くクライアントアプリで、AWSへのデプロイは不要（Q8）
- 観測した事象：Logs Insights が使えない古いロググループで、マネコン（FilterLogEvents）では過去ログが見えず、ログ削除の記録はなく、CLI の get-log-events では見えた。原因は未特定（Q9）
- MVP と将来目標の境界：スコープ定義ステージで確認する（Q10）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct

## Assumption Confirmation

成果物に次の前提（assumption）が残っています。

- [assumption] 成功指標「数万〜数十万件規模のイベントでも実用的な時間でフィルタ結果が返る」の「実用的な時間」の具体的な目標値（例：何秒以内か）は、要件分析ステージで数値として確定させる前提とする。

A. Accept assumptions
B. Convert to follow-up questions

[Answer]: A. Accept assumptions
