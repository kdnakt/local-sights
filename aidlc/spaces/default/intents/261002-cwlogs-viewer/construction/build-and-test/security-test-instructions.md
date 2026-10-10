# Security Test Instructions — セキュリティの確認の手順

対象の目標は `inception/requirements-analysis/requirements.md` の NFR5〜NFR8 と NFR16、`memory/project.md` の Forbidden（秘密の認証情報とアクセスキー ID を書かない、読み取り API 3 つ以外を呼ばない、テストと CI から AWS に接続しない、テレメトリを送らない）。ローカルで動くデスクトップアプリで、サーバーも認証の仕組みも持たないため、DAST・認証テスト・インジェクションのテストは対象外とし、静的な確認と既存の単体テストで確かめる。

## 目標と確かめ方

| ID | 目標 | 確かめ方 | 期待 |
|----|------|----------|------|
| NFR5 | 秘密の認証情報とアクセスキー ID を、リポジトリ・ログ・エラー表示・キャッシュに書かない。独自に保存しない | (1) `failure.rs` の `redact_access_key_ids` と `catalog/mod.rs` の `credential_values_are_never_kept`、`gateway/aws.rs` の分類テスト、`cache/mod.rs` のテスト（`cargo test -p local-sights-core`）。(2) リポジトリの検索 `grep -rnE 'AKIA[0-9A-Z]{16}' --include=*.rs --include=*.ts --include=*.tsx --include=*.json --include=*.md . \| grep -v node_modules \| grep -v target`。(3) GitHub の secret scanning と push protection を有効にする（リポジトリの設定） | (1) すべて成功。(2) テストのダミーと AWS 公式の例示用キー以外に一致なし。(3) 有効 |
| NFR6 | 呼ぶ API は DescribeLogGroups・DescribeLogStreams・GetLogEvents だけ。README に IAM 権限を書く | `grep -rnoE '\.[a-z_]+\(\)' crates/local-sights-core/src/gateway/aws.rs` で SDK の操作を列挙し、`aws_sdk_cloudwatchlogs` を使うファイルが `gateway/aws.rs` だけであることを `grep -rln aws_sdk_cloudwatchlogs crates src-tauri/src` で確かめる。README の「Required IAM permissions」を読む | 操作は 3 つだけ。SDK の利用は 1 ファイル。README に 3 つの権限 |
| NFR7 | テレメトリやクラッシュレポートを送らない。通信先は AWS の API と SSO のエンドポイントだけ | 画面側 `grep -rnE 'fetch\(\|XMLHttpRequest\|sendBeacon\|new WebSocket' src --include=*.ts --include=*.tsx`、`src-tauri/tauri.conf.json` の CSP（`connect-src ipc: http://ipc.localhost`）、`src-tauri/Cargo.toml` の依存に通信系のプラグイン（`tauri-plugin-http` など）がないこと、`Cargo.toml` の依存に AWS SDK 以外の HTTP クライアントの直接利用がないこと | 画面に外部通信なし。CSP が IPC 以外を許さない。通信系プラグインなし |
| NFR8 | キャッシュが無効のとき、取得したログをディスクに書かない | `coordinator.rs` の `a_disabled_cache_reads_and_writes_nothing_and_fetches_as_u3`、`cache/settings.rs` の `no_settings_file_means_disabled`、`a_broken_or_unknown_settings_file_means_disabled_and_is_kept`、`tests/u6_cache_flow.rs` | すべて成功。既定は無効 |
| NFR16 | 診断ログは標準エラー出力だけ。ファイルには書かない | `grep -rnE 'eprintln!\|println!\|log::\|tracing\|env_logger\|tauri_plugin_log' crates/local-sights-core/src src-tauri/src`（テストのモジュール以外） | `eprintln!` だけ。ログのファイル出力やログのクレートなし |
| Forbidden | 自動テストと CI から AWS に接続しない。CI に認証情報を置かない | 結合テストが偽物の gateway だけを使うこと（`tests/support/fake_gateway.rs`）、接続先の解決のテストが `AWS_CONFIG_FILE`・`AWS_SHARED_CREDENTIALS_FILE` を一時ファイルに向け `AWS_EC2_METADATA_DISABLED=true` にしていること。CI の定義（次のステージ）に AWS の secrets を置かない | 実際の AWS に出る経路なし |
| 画面の権限 | 画面が呼べるのはこのアプリ自身のコマンドだけ | `src-tauri/capabilities/default.json` を読む | `core:event:default` とこのアプリの `allow-*` コマンドだけ。shell・fs・http などの権限なし |
| 依存関係 | 既知の脆弱性がない | `npm audit`、`cargo deny check`（`cargo-deny` と `deny.toml` は次の CI Pipeline のステージで用意する） | 脆弱性 0 |

## 実行方法

```bash
# 単体・結合テスト（NFR5・NFR8・Forbidden の確認を含む）
CARGO_INCREMENTAL=0 cargo test -p local-sights-core

# 静的な確認
grep -rnoE '\.(get_log_events|describe_log_groups|describe_log_streams|filter_log_events|start_query|put_[a-z_]+|create_[a-z_]+|delete_[a-z_]+)\(\)' crates/local-sights-core/src src-tauri/src
grep -rln aws_sdk_cloudwatchlogs crates/local-sights-core/src src-tauri/src
grep -rnE 'eprintln!|println!|log::|tracing|env_logger|tauri_plugin_log' crates/local-sights-core/src src-tauri/src
grep -rnE 'fetch\(|XMLHttpRequest|sendBeacon|new WebSocket' src --include=*.ts --include=*.tsx
grep -rnE 'AKIA[0-9A-Z]{16}' --include=*.rs --include=*.ts --include=*.tsx --include=*.json --include=*.md . | grep -v node_modules | grep -v '^./target'
cat src-tauri/capabilities/default.json
grep -n 'csp' src-tauri/tauri.conf.json

# 依存関係
npm audit
cargo deny check   # cargo-deny を入れてから。deny.toml は CI Pipeline のステージで作る
```

## 手元で確かめること

- 認証に失敗したとき（SSO のログイン切れ）の画面のエラー表示に、アクセスキー ID や秘密の値が出ないこと。プロファイル名・ロール ARN・アカウント ID は出てよい（FR8.3）。
- キャッシュを有効にしたときの保存場所（`~/Library/Caches/` 配下）に、認証情報が書かれていないこと。キャッシュのファイルはログのイベントとヘッダ（プロファイル名・リージョン・ロググループ・範囲・件数）だけ。

## 期待する範囲

- 上の表のすべての行が期待どおり。1 行でも違えばこのステージの失敗として扱い、弱めない。
- 秘密の認証情報を扱うコード（`catalog/`、`gateway/aws.rs`、`failure.rs`、`cache/`）には、漏れないことを確かめるテストが少なくとも 1 件ある。
