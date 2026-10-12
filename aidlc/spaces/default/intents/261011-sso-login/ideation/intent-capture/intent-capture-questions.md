# Intent Capture Questions — アプリから SSO ログインする

## Sources

- [desc] Initial description: "アプリから SSO ログインしたい、1の方向性で。"
- [scope] Workflow-selected scope: `refactor`.
- [memory:M1] `aidlc/spaces/default/memory/project.md#Forbidden`: "NEVER 秘密の認証情報（シークレットアクセスキー・セッショントークン・SSO のトークン）とアクセスキー ID を、リポジトリ・アプリのログ・画面のエラー表示・キャッシュファイルに書き込む（プロファイル名・ロール名／ロール ARN・アカウント ID はログやエラー表示に出してよいが、リポジトリには置かない） (affirmed 2026-10-03)"
- [memory:M2] `aidlc/spaces/default/memory/project.md#Forbidden`: "NEVER CloudWatch Logs の読み取り API（DescribeLogGroups / DescribeLogStreams / GetLogEvents）以外の API を呼ぶ (affirmed 2026-10-03)"

## Q1. いま、いちばん困っているのはどの場面ですか？

SSO が切れているとき、今のアプリは「aws sso login を実行してから再読み込み」と案内を出すだけです。何を解決したいかをはっきりさせるための質問です。複数選べます（select all that apply）。

- A. アプリとターミナルを行き来して `aws sso login` を打つのが手間
- B. どのプロファイルでログインし直せばよいか、毎回考えるのが手間（プロファイル名を打ち間違える）
- C. 朝いちばんや長い休憩のあとなど、SSO が切れるたびに作業が止まる
- D. ターミナルを開かずにアプリの中だけで完結させたい
- E. None（特に困っていない。あると便利という程度）
- X. Other (please specify)

[Answer]: A, B, C, D

## Q2. 「1の方向性」の中身はこの理解で合っていますか？

前回の会話で示した案 1 を、次のように理解しています。アプリが手元の AWS CLI（`aws sso login --profile <名前>`）を起動し、ブラウザでの承認・SSO の API 呼び出し・トークンの保存はすべて CLI が行います。アプリ自身は SSO の API を呼ばず、トークンにも触れません。

- A. その理解で合っている
- B. 合っているが、CLI が手元にない場合の代わりの手段も欲しい（X で書いてください）
- C. 少し違う（X で書いてください）
- D. Not yet defined（要件の段階で詰める）
- X. Other (please specify)

[Answer]: A. その理解で合っている

## Q3. 禁止ルールとの関係をどう扱いますか？

project.md の禁止ルールに「CloudWatch Logs の読み取り API 以外を呼ばない」[memory:M2] と「SSO のトークンなどをキャッシュファイル等に書き込まない」[memory:M1] があります。案 1 ではアプリ自身はどちらもしませんが、「アプリがログインを起こす」ことがルールの趣旨の範囲内かは、決めておく必要があります。

- A. 範囲内とする。ルールはアプリ自身の API 呼び出しとトークンの書き込みを禁じたもので、CLI を起動するのは別扱いと解釈し、その解釈を記録する
- B. 範囲内とする。ただし誤解のないよう、ルールの文面に「CLI の起動は対象外」と補足を足したい
- C. 範囲外かもしれない。要件の段階でもう一度考えたい
- D. Not yet defined
- X. Other (please specify)

[Answer]: B. 範囲内とする。ただし誤解のないよう、ルールの文面に「CLI の起動は対象外」と補足を足したい

## Q4. できあがりは、どう判断しますか？

- A. SSO が切れた状態で、アプリの画面からログインを始めて、ターミナルを開かずに一覧や取得をやり直せる
- B. A に加えて、どのプロファイルでログインするかを自分で選ばずに済む（いま選んでいるプロファイルで自動的に行う）
- C. A に加えて、CLI がない・時間切れ・キャンセルなどの失敗が分かりやすく表示される
- D. B と C の両方
- E. Not yet defined（要件の段階で決める）
- X. Other (please specify)

[Answer]: A. SSO が切れた状態で、アプリの画面からログインを始めて、ターミナルを開かずに一覧や取得をやり直せる

## Q5. 関係者と決める人は、これまでと同じですか？

これまでの作業では「開発者本人がひとりで決める。ほかに気にする人は OSS の利用者だけ。共有は GitHub の PR・README 程度」でした。今回は会社の Mac で使う前提の話が出ています。

- A. 同じ（開発者本人がひとりで決める。共有は GitHub の PR・README 程度）
- B. 決める人は本人のままだが、会社の情報システム担当などに確認したい点がある（X で書いてください）
- C. Not identified（まだ決めていない）
- X. Other (please specify)

[Answer]: A. 同じ（開発者本人がひとりで決める。共有は GitHub の PR・README 程度）

## Q6. この作業で「やらないこと」はどれですか？

進め方はすでに決まっています（`refactor` をもとにした独自プラン sso-login-trigger）。ここで聞くのは、今回の成果物に入れないものです。複数選べます（select all that apply）。

- A. アプリ自身による SSO ログイン（SSO の API を直接呼ぶ方式）は作らない
- B. SSO 以外の認証（アクセスキー、`credential_process` など）のログイン補助は作らない
- C. プロファイルの作成や `~/.aws/config` の編集はアプリから行わない
- D. ログインの状態を常に監視して、切れる前に知らせる仕組みは作らない
- E. None（特に除外するものはない）
- X. Other (please specify)

[Answer]: A, B, C, D
