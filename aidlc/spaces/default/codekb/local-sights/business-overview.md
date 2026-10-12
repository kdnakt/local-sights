# Business Overview

> リポジトリ `local-sights` のビジネス面の概要。2026-10-12 の全体スキャン（コミット `9d08cbf`）に基づく。

## Business Domain

Amazon CloudWatch Logs に保存されたログを、開発者や運用担当者が自分の PC（macOS、Apple Silicon）の上で素早く読むための**ローカル専用デスクトップビューア**。アプリの名前は「Local Sights」、パッケージ・クレート・バイナリ名は `local-sights`。

- 対象の利用者: 1 人の開発者が作り、会社の PC に配って同僚も使う想定（team.md Deployment）。
- 置き換える作業: AWS マネジメントコンソールのログ画面や `aws logs` CLI で、ログストリームを順に開いて時刻を追う手作業。
- サーバーもバックエンドも持たない。アプリは利用者本人の AWS 認証情報で、利用者の PC から直接 AWS を呼ぶ。

## Purpose

1. **ログを時刻順に一望する**: 選んだロググループの、指定した時刻範囲に関係するすべてのストリームを 1 つずつ読み、イベントを時刻順にマージして 1 つの表に並べる。100 万件規模でも仮想スクロールで滑らかに読める。
2. **安全に使える**: 呼ぶ AWS API は読み取り専用の `DescribeLogGroups` / `DescribeLogStreams` / `GetLogEvents` の 3 つだけ。認証情報を読まず、保存せず、ログにも出さない。テレメトリやクラッシュレポートは送らない（project.md Forbidden）。
3. **会社の PC に配れる**: GitHub Releases に置いた `.dmg` から手でインストールし、手で更新・戻す。自動更新は無い。Apple Developer Program には参加しておらず、Apple による署名・公証は行っていない（`docs/install.md` が初回起動の警告の開け方を案内する）。

## Key Functionality

| 機能 | 内容 | 由来する作業単位 |
|---|---|---|
| 接続の選択 | `~/.aws/config` / `~/.aws/credentials` のプロファイル名と `region` だけを読み、プロファイルと リージョンを選ぶ。表示中のログがあるときだけ接続変更を確認する | U2 |
| ロググループ一覧 | `DescribeLogGroups` を全ページ読み、名前で絞り込み、1 つ選ぶ | U2 |
| 全ストリーム取得 | `DescribeLogStreams`（最終イベント時刻の新しい順）で範囲に関係するストリームを選び、1 つずつ `GetLogEvents` でページを辿る。スロットリングと通信エラーは要求ごとに最大 5 回再試行し、失敗したストリームは別に一覧する。途中のエラーでも取得済みページは残す | U1, U3 |
| 時刻範囲とタイムゾーン | `yyyy/mm/dd` + `hh:mm:ss` の開始・終了（終了秒の 999 ミリ秒まで含む）、Local（OS のタイムゾーン、DST 対応）と UTC の切替 | U1, U4 |
| ログフィルタ | 取得済みイベントを、メッセージの部分一致（大文字小文字を無視）で PC 内で絞り込む。AWS は呼ばない | U5 |
| ディスクキャッシュ | 任意（既定は無効）。全ストリーム成功した取得のうち、取得開始の 5 分前までを `~/Library/Caches/dev.local-sights.app/log-cache` に JSON Lines で保存し、同じ範囲は AWS を呼ばずに表示する | U6 |
| 画面の仕上げ | 行の展開表示、キーボード操作、取得中の終了確認、日本語／英語、ライト／ダーク | U7 |
| コンソール風の見た目 | Cloudscape Design System（AWS マネジメントコンソールの部品）で描く | 261010-console-ui-polish |
| SSO サインイン | 期限切れの SSO セッションを検出したとき、アプリから AWS CLI の `aws sso login` を起動する。アプリ自身は SSO API を呼ばず、トークンにも触れない | 261011-sso-login |
| リリースと配布 | `vX.Y.Z` タグで GitHub Actions が Apple Silicon の `.dmg` を作り GitHub Releases に公開する。4 ファイルの版の一致をスクリプトで検査する | 261011-local-release |

## Constraints and Policies

- **読み取り専用**: CloudWatch Logs の読み取り API 以外は呼ばない（コードでは `CloudWatchLogsGateway` trait がそれ以外の操作を持たないことで保証）。
- **秘密情報を書かない**: アクセスキー ID・シークレット・セッショントークン・SSO トークンを、リポジトリ・ログ・画面・キャッシュに書かない。エラー表示は許可リストのフィールド（種別、API 名、リクエスト ID、プロファイル名、ロググループ、ストリーム）だけから組み立てる。
- **自動テストと CI は AWS に接続しない**: AWS 呼び出しは trait の裏に置き、テストは偽物に差し替える。CI に AWS 認証情報は置かない。
- **配布先は macOS（Apple Silicon）のみ**: Intel Mac と Windows には配らない。
- **Apple の署名・公証はしない**: 現状は未署名のまま `.dmg` を配り、Gatekeeper の警告の開け方を手順書に書く方針（team.md Deployment）。本インテント（261012-adhoc-signing）は、Developer Program に参加せずに CI でアドホック署名を行い、macOS 15 Sequoia で「壊れているため開けません」と出る状態を「開発元を確認できません」系の警告に変えることを目指す。

## Related Artifacts

- アーキテクチャと取得の流れ: `architecture.md`
- API 面（Tauri コマンド、trait、AWS API）: `api-documentation.md`
- 品質・CI・技術的負債: `code-quality-assessment.md`
