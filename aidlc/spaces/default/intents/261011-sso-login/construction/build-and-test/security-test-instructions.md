# Security Test Instructions — アプリから SSO ログインする

テスト戦略は Standard なので必須ではないが、この変更は外部コマンドの起動と認証に触れ、`requirements.md` にセキュリティの NFR（NFR1〜NFR3）があるため、確認の手順を残す。

## 自動の確認

| 要件 | 確認 | コマンド |
|---|---|---|
| NFR1 アプリは SSO の API を呼ばず、トークンを読み書きしない | SSO・SSO-OIDC の API 名と crate、トークンキャッシュのパスがコードにないこと | `grep -rnE "aws_sdk_sso\|ssooidc\|CreateToken\|StartDeviceAuthorization\|RegisterClient\|GetRoleCredentials\|sso/cache\|\.aws/sso" crates/*/src crates/*/tests src-tauri/src src crates/*/Cargo.toml src-tauri/Cargo.toml package.json` が 0 件 |
| NFR1 AWS の API は CloudWatch Logs の 3 つだけ | `gateway/aws.rs` の SDK 呼び出しが `get_log_events`・`describe_log_groups`・`describe_log_streams` だけ | `grep -rnE "\.(get_log_events\|describe_log_groups\|describe_log_streams\|[a-z_]+)\(\)\s*$" crates/local-sights-core/src/gateway/aws.rs` を目で確かめる |
| NFR2 シェルを通さない・起動するのは見つけた `aws` だけ | `Command::new` の引数が探索結果のパスで、`sh -c` がないこと。引数は `cli_args` の結果だけ | `grep -rn "Command::new" src-tauri/src` と `sso_login` のテスト |
| NFR2 URL を開く前の検証 | 許可リスト（`https://`、ユーザー情報なし、ポートなし、ホストが `.amazonaws.com`／`.awsapps.com`、2048 文字以内） | `cargo +stable test -p local-sights-core sso_login` |
| NFR3 CLI の出力を残さない | 行を保持もログもせず、取り出した URL とコードだけを渡す | `cargo +stable test -p local-sights sso_cli`（許可されない URL は届かない）と、`sso_cli.rs` の `eprintln!` が結果の種類だけを出すことを目で確かめる |
| 依存関係の脆弱性 | 画面側 | `npm audit --audit-level=high` |
| 依存関係の脆弱性 | Rust 側 | `cargo deny check`（CI の cargo-deny ジョブ） |

## 手動の確認

- 実機でログインしたあと、アプリのログ（標準エラー）に URL・確認コード・トークンが出ていないことを確かめる。
- `open_sso_url` に今の試行と違う URL を渡しても `/usr/bin/open` が起動しないこと。自動テストは URL の判定だけを確かめ、`open` の起動は確かめていない。

## 権限の境界

Tauri の capabilities は、ウィンドウが呼べるコマンドを絞る仕組み。新しい 4 つのコマンドは、今はどこにも登録されていない（レビュー R-01）。登録するときは、`open_sso_url` が外部プロセスを起動するコマンドであることに注意し、`allow-open-sso-url` 以外の広い権限を足さない。
