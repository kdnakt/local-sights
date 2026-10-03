# Feasibility & Constraints 質問票

上流の成果物：`aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/intent-capture/intent-statement.md`

## 調査メモ（質問の前提として共有）

AWS 公式ドキュメントで確認した事実です（出典 URL つき）。質問の背景として使います。

- GetLogEvents は 1 回の呼び出しで最大 1 MB または 10,000 件を返す。ページが空でもトークンが変わる限り続きがありうる。終端では渡したトークンと同じトークンが返る。トークンの有効期限は 24 時間。（https://docs.aws.amazon.com/AmazonCloudWatchLogs/latest/APIReference/API_GetLogEvents.html）
- GetLogEvents はログストリーム単位の API で、ロググループ全体を横断する場合はストリームごとに呼ぶ必要がある。（同上。必須パラメータは logStreamName）
- GetLogEvents のスロットリング上限はアカウント・リージョンあたり毎秒 25 回（us-west-2、eu-west-1、eu-central-1 などは毎秒 10 回、eu-west-3 は毎秒 30 回）。引き上げ不可。DescribeLogStreams は毎秒 25 回（引き上げ可）。（https://docs.aws.amazon.com/AmazonCloudWatch/latest/logs/cloudwatch_limits_cwl.html）
- AWS の re:Post ナレッジセンターには「ロググループ作成日時より前のタイムスタンプを持つログイベントは Logs Insights のクエリ対象にならない」と記載がある。（https://repost.aws/knowledge-center/cloudwatch-logs-query-errors）今回の事象の原因かどうかは未確認（仮説）。

## Q1. 閲覧対象の AWS 環境はどのような構成ですか？

背景：アカウント／リージョンの数によって、プロファイル切り替えやリージョン選択の機能が必要かが決まります。

A. 1 アカウント・1 リージョンだけ
B. 1 アカウント・複数リージョン
C. 複数アカウント（プロファイルを切り替えて使う）
D. 複数アカウント＋複数リージョン
E. Not yet defined
X. Other (please specify)

[Answer]: D. 複数アカウント＋複数リージョン

## Q2. AWS の認証情報は普段どのように使っていますか？（select all that apply）

背景：AWS CLI と同じ認証情報の読み込み方式（~/.aws/config のプロファイル、SSO など）に対応できれば、追加設定なしで使えます。OSS 利用者も同様です。

A. IAM Identity Center（AWS SSO）のプロファイル
B. ~/.aws/credentials のアクセスキー
C. AssumeRole（role_arn を指定したプロファイル）
D. 環境変数（AWS_ACCESS_KEY_ID など）
E. Not yet defined
X. Other (please specify)

[Answer]: A, B, C, D

## Q3. 閲覧するログに機密情報（個人情報・認証情報など）が含まれる可能性はありますか？

背景：ツールが取得したログをローカルディスクにキャッシュ・保存するかどうかの判断に影響します。OSS として公開するため、利用者側のデータの扱いも方針が必要です。

A. 含まれうる。ログをディスクに保存せず、メモリ上だけで扱いたい
B. 含まれうるが、ローカルキャッシュ（ディスク保存）は利用者の選択で許容する
C. 含まれない（社内のアプリケーションログのみ）
D. Not yet defined
X. Other (please specify)

[Answer]: B. 含まれうるが、ローカルキャッシュ（ディスク保存）は利用者の選択で許容する

## Q4. Rust の経験はどの程度ですか？

背景：非同期処理（AWS SDK for Rust は非同期）や TUI／GUI ライブラリの選定で、学習コストをどこまで許容できるかを見積もります。

A. 業務・個人で継続的に書いている
B. 少し書いたことがある
C. ほぼ初めて（学習も兼ねる）
D. Not applicable
X. Other (please specify)

[Answer]: A. 業務・個人で継続的に書いている

## Q5. 最初に使えるもの（MVP）はいつまでに欲しいですか？

背景：実務で困っているとのことなので、期限に応じて MVP の範囲を絞ります。

A. 1 週間以内
B. 2〜4 週間
C. 1〜3 か月
D. 期限は特にない
E. Not yet defined
X. Other (please specify)

[Answer]: A. 1 週間以内

## Q6. 閲覧対象の古いロググループの規模感はどのくらいですか？

背景：GetLogEvents はストリーム単位・毎秒 25 回までなので、ストリーム数とイベント数で取得時間が大きく変わります（例：1,000 ストリームを 1 回ずつ読むだけで最短 40 秒）。

A. ストリーム数は少ない（〜100 程度）、イベントは〜数十万件
B. ストリーム数が多い（数百〜数千）、イベントは〜数十万件
C. ストリーム数・イベント数とも非常に多い（数千以上、数百万件以上）
D. Not yet defined（調べないとわからない）
X. Other (please specify)

[Answer]: C. ストリーム数・イベント数とも非常に多い（数千以上、数百万件以上）

## Q7. API 呼び出しの料金について制約はありますか？

背景：CloudWatch Logs の API 呼び出しやデータ転送に料金がかかる場合があります（料金は要件分析以降で公式価格表を確認します）。大量取得を繰り返す使い方を許容できるかを確認します。

A. 少額なら気にしない
B. できるだけ抑えたい（取得済みデータの再利用などを重視）
C. 厳しい制約がある（具体的に記入）
D. Not yet defined
X. Other (please specify)

[Answer]: B. できるだけ抑えたい（取得済みデータの再利用などを重視）

## Q8. 今回の事象の原因調査をこのツール開発に含めますか？

背景：re:Post には「ロググループ作成日時より前のタイムスタンプのイベントは Logs Insights の対象外」という記載があり、ロググループの再作成などが原因の可能性があります（未確認の仮説）。原因が分かれば、ツールが必要な範囲も明確になります。

A. 含める：原因の仮説を検証する手順（ロググループ作成日時とイベント時刻の比較など）を実現性評価に入れる
B. 含めない：原因は問わず、GetLogEvents で見えることだけを前提に進める
C. 原因調査は別途自分で行う
D. Not yet defined
X. Other (please specify)

[Answer]: B. 含めない：原因は問わず、GetLogEvents で見えることだけを前提に進める

## Q9. （追加質問）MVP の期限と対象規模の折り合いをどうつけますか？

背景：MVP は 1 週間以内に欲しい（Q5）一方で、対象は複数アカウント・複数リージョン（Q1）、ストリーム数千以上・数百万件以上（Q6）です。GetLogEvents はストリーム単位で毎秒 25 回までなので、ロググループ全体を毎回すべて取得すると数分以上かかり、料金も抑えたい（Q7）方針とぶつかります。また、意図整理での成功指標は「数万〜数十万件でも実用的な時間でフィルタできる」でした。

A. MVP は「ロググループとストリーム（または時間範囲）を選んで、その範囲だけ取得・表示・フィルタ」に絞る。ロググループ全体の横断取得は MVP 後
B. MVP でもロググループ全体の横断取得は必須。1 週間は目安で、延びてもよい
C. MVP でも横断取得は必須で、期限も 1 週間を守る（取得の高速化・キャッシュは後回しで、遅くてもよい）
D. Not yet defined
X. Other (please specify)

[Answer]: A. MVP は「ロググループとストリーム（または時間範囲）を選んで、その範囲だけ取得・表示・フィルタ」に絞る。ロググループ全体の横断取得は MVP 後

## Consolidated Summary Confirmation

回答のまとめ：

- 対象環境：複数アカウント＋複数リージョン（Q1）
- 認証方式：SSO、アクセスキー、AssumeRole、環境変数のすべてを使う（Q2）
- 機密性：ログに機密情報が含まれうる。ディスクへのキャッシュは利用者が選んだ場合のみ許容（Q3）
- Rust 経験：業務・個人で継続的に書いている（Q4）
- MVP 期限：1 週間以内（Q5）
- 規模：ストリーム数千以上・イベント数百万件以上（Q6）
- 料金：できるだけ抑えたい（Q7）
- 原因調査：開発には含めない。GetLogEvents で見えることだけを前提にする（Q8）
- 期限と規模の折り合い：MVP はロググループとストリーム（または時間範囲）を選んだ範囲だけを取得・表示・フィルタする。ロググループ全体の横断取得は MVP 後（Q9）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
