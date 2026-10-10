# Constraint Register — CloudWatch Logs ローカルビューア

上流の成果物：`intent-statement`（`aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/intent-capture/intent-statement.md`）。

## Technical Constraints

| ID | 制約 | 影響 | 出典 |
|----|------|------|------|
| C-T1 | ログの取得には GetLogEvents API を使う（利用者が指定した前提） | ストリーム単位の取得になり、ロググループ全体を読むにはストリームを列挙して個別に呼ぶ必要がある | intent-statement、project.md Corrections |
| C-T2 | GetLogEvents は 1 回あたり最大 1 MB または 10,000 件。ページは部分的に埋まる・空のこともある | 件数から呼び出し回数を正確には予測できない | AWS API リファレンス |
| C-T3 | ページングの終端は「渡したトークンと同じトークンが返る」ことで判定する。トークンの有効期限は 24 時間 | 終端判定を誤ると無限ループや取りこぼしになる | AWS API リファレンス |
| C-T4 | startFromHead=true で endTime を指定しないと、ページングが終わらない場合がある | 取得時は終了時刻を常に指定する | AWS API リファレンス |
| C-T5 | GetLogEvents のスロットリング上限はアカウント・リージョンあたり毎秒 25 回（一部リージョンは毎秒 10 回または 30 回）、引き上げ不可 | 大規模な横断取得は時間がかかる。待機・再試行が必須 | CloudWatch Logs クォータ |
| C-T6 | DescribeLogStreams の上限は毎秒 25 回（引き上げ可） | ストリーム数千以上の列挙にも時間がかかる | CloudWatch Logs クォータ |
| C-T7 | 実装言語は Rust（利用者が指定した前提） | ライブラリ選定は Rust のエコシステムから行う | intent-statement、project.md Corrections |
| C-T8 | ローカルで動くクライアントアプリで、AWS 側へのデプロイはしない | AWS リソースを作らない。読み取り API だけを使う | intent-statement（Q8） |

## Organizational Constraints

| ID | 制約 | 影響 | 出典 |
|----|------|------|------|
| C-O1 | 開発者は 1 人。スコープと優先順位も 1 人で決める | 意思決定は速いが、レビューは AI-DLC のレビューに頼る | intent-statement、stakeholder-map |
| C-O2 | MVP は 1 週間以内に必要 | MVP の範囲を範囲指定取得に絞る | Q5、Q9 |
| C-O3 | 開発者は Rust を継続的に書いている | 学習コストは小さい | Q4 |
| C-O4 | OSS として公開する | 認証情報・ログの扱いを利用者に説明できる設計にする。README で必要な IAM 権限を示す | intent-statement |
| C-O5 | 料金はできるだけ抑えたい | 不要な再取得を避ける。任意のキャッシュを用意する | Q7 |

## Environment Constraints

| ID | 制約 | 影響 | 出典 |
|----|------|------|------|
| C-E1 | 対象は複数アカウント・複数リージョン | プロファイルとリージョンの切り替えが必要 | Q1 |
| C-E2 | 認証方式は SSO、アクセスキー、AssumeRole、環境変数のすべてを使う | AWS CLI と同じ認証設定を読めることが必要 | Q2 |
| C-E3 | 対象の古いロググループは、ストリーム数千以上・イベント数百万件以上 | ロググループ全体の横断取得は MVP 後に回す | Q6、Q9 |

## Regulatory / Data Constraints

| ID | 制約 | 影響 | 出典 |
|----|------|------|------|
| C-R1 | ログに機密情報（個人情報・認証情報など）が含まれうる | 既定ではメモリ上だけで扱う。ディスクへのキャッシュは利用者が選んだ場合のみ | Q3 |
| C-R2 | ツールは AWS 認証情報を独自に保存・外部送信しない。テレメトリも送らない | OSS 利用者の信頼を得るための前提 | Q3、intent-statement（OSS 公開） |
| C-R3 | 特定の規制フレームワークへの準拠は求められていない | 規制対応の追加作業は想定しない（前提として RAID ログ A-03 に記録） | 本ステージの回答（規制要件の指定なし） |

## Out of Scope Constraints

| ID | 内容 | 出典 |
|----|------|------|
| C-X1 | 過去ログが見えなかった原因の調査は、この開発に含めない | Q8 |
| C-X2 | ロググループ全体の横断取得は MVP に含めない（MVP 後に再評価） | Q9 |
