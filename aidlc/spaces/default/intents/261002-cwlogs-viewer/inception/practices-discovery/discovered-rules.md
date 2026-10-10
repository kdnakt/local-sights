# Discovered Rules

> 人間がインタビュー（Q10、追加質問 F1）で明言した、例外のないハードな制約だけを記録する。それ以外の慣行は `team-practices.md` に置く。各行の `NEVER` の後ろには、してはならない行為を書く。

## Mandated

None.

## Forbidden

- NEVER 秘密の認証情報（シークレットアクセスキー・セッショントークン・SSO のトークン）とアクセスキー ID を、リポジトリ・アプリのログ・画面のエラー表示・キャッシュファイルに書き込む（プロファイル名・ロール名／ロール ARN・アカウント ID はログやエラー表示に出してよいが、リポジトリには置かない）
- NEVER CloudWatch Logs の読み取り API（DescribeLogGroups / DescribeLogStreams / GetLogEvents）以外の API を呼ぶ
- NEVER 自動テストや CI から実際の AWS に接続する、または CI に AWS の認証情報を置く
- NEVER テレメトリやクラッシュレポートを外部に送る
