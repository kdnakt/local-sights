# Code Summary — アプリから SSO ログインする

計画は `code-generation-plan.md`。認証エラーの表示に「SSO にログイン」ボタンを足し、アプリが AWS CLI の `aws sso login` をシェルなしで起動して、画面中央のダイアログで待機・URL・確認コード・キャンセル・5 分の時間切れを扱う。成功したら SDK のクライアントを捨てて一覧を読み直す。

## 作ったファイル・変えたファイル

### 新規

| ファイル | 中身 |
|---|---|
| `crates/local-sights-core/src/sso_login.rs` | `resolve_login_target`、`cli_args`、`extract_login_hints`、`is_allowed_login_url`、`SsoLoginOutcome` などの純粋ロジック |
| `src-tauri/src/sso_cli.rs` | CLI の探索（`locate_cli`）、起動（`spawn_login`）、出力の読み取り、時間切れ、プロセスグループの停止 |
| `src/components/SsoLoginButton.tsx`・`.test.tsx` | 「SSO にログイン」ボタン |
| `src/components/SsoLoginModal.tsx`・`.test.tsx` | 待機・結果のダイアログ |

### 変更

| 層 | ファイル |
|---|---|
| コア | `crates/local-sights-core/src/catalog/mod.rs`、`catalog/files.rs`、`session.rs`、`gateway/aws.rs`、`lib.rs` |
| Tauri 側 | `src-tauri/src/lib.rs`、`src-tauri/Cargo.toml`（`libc = "0.2"`、開発用の `tempfile`） |
| ロックファイル | `Cargo.lock`（`local-sights` の依存に `libc` と `tempfile` が加わっただけ。どちらも既にロック済みの crate で、新しい crate は増えていない） |
| 画面 | `src/api.ts`、`src/App.tsx`、`src/components/LogGroupPane.tsx`、`FetchErrorBanner.tsx`、`TopBar.tsx`、`src/i18n/messages.ts`、`src/styles.css`、`src/test/fixtures.ts` |
| 既存テストへの追加 | `src/App.test.tsx`、`src/components/LogGroupPane.test.tsx`、`FetchErrorBanner.test.tsx`、`TopBar.test.tsx`、`src/i18n/messages.test.ts` |
| 文書 | `README.md`（「Sign in with SSO」の節と、手動確認のチェックリスト 11 項目） |

削除したファイルはない。

## 主な判断

- **SSO の判定**：config ファイルの `sso_session`／`sso_start_url` は有無だけ、`source_profile` は名前だけを記録する。値（開始 URL など）と credentials ファイルの行は保持しない。`ConnectionCatalog` が `ProfileAuthInfo` を持ち、`load_catalog` で設定する。
- **ログイン先**：選択中のプロファイルから `source_profile` を最大 5 段たどる。循環、一覧にないプロファイル、6 段以上は対象外とする。`--profile` を付けないのは「デフォルト」を選んでいて、CLI が自分で同じプロファイルを選ぶときだけ。
- **URL の許可リスト**：`https://` で始まり、ユーザー情報もポートもなく、ホストが `.amazonaws.com` または `.awsapps.com` で終わり、2048 文字以内のものだけを採る。1 行は先頭の 4096 バイトまでしか見ない。
- **セッションの状態**：試行番号で古い試行への出来事を捨て、先に届いた出来事を採る。ログイン中は接続の変更、一覧の再読み込み、ロググループの選択、取得、設定ダイアログを `SessionError::SsoLoginInProgress` で拒否する。閉じる確認の条件は変えていない。
- **成功時の順序**：`clear_clients()` で SDK クライアントを捨ててから一覧を読み直す。取得はやり直さない（FR3.2）。
- **CLI の起動**：シェルを通さず、標準入力は null、新しいプロセスグループで起動する。キャンセル・時間切れ・アプリ終了（止められなかった `ExitRequested`、`Exit`、ウィンドウの `Destroyed`）でグループに SIGTERM を送り、2 秒待って残っていれば SIGKILL を送る。`killpg` は 1 以下の値には呼ばない。
- **CLI が自分で正常に終わったとき**：グループは止めない。CLI が開いたブラウザを閉じないため。
- **出力の扱い**：標準エラーに出すのは結果の種類だけ。URL・確認コード・CLI の出力の行はログにも出さず、保持もしない。
- **`open_sso_url`**：今の試行の URL と一致し、許可リストも満たすときだけ `/usr/bin/open` を固定パスで起動する。
- **失敗の表示**：FR3.3 の「CLI が出した失敗の理由」は、機能設計（functional-spec.md、BR4.2）の決定どおり、終了コードと代わりの手順を出す形にしている。

## 計画からの逸脱

- **公開 API の追加**：計画になかった `SsoLoginView.canStart`、`AppSession::sso_login_url()`（テスト付き）、`AwsCloudWatchLogsGateway::cached_client_count()`（`clear_clients` のテストのため）、`SessionError` の新しい値 2 つ、`SsoLoginStart`／`SsoLoginFinish` を足した。
- **文言表にない key の追加**：
  - `ssoLogin.failedToStart`：CLI を起動できず、終了コードがない失敗
  - `ssoLogin.openFailed`：URL を開けなかったとき
  - `session.ssoLoginInProgress`、`session.ssoLoginUnavailable`
- **モーダルの ×**：Cloudscape の Modal は × を消せないため、待機中は押しても何もしない。Escape も同じ。
- **成功の知らせの位置**：計画の「右ペインの上」は、条件エリアの下・ログ一覧の上と解釈した。
- **作業の手順**：Step 2 で文書コメントを直すとき、1 回だけファイル編集ツールでなくスクリプトで書き込んだ。内容への影響はない。
- **作業した環境**：計画は開発者の Mac 向けに書いたが、この作業は Linux のコンテナで行った。`/Users/kdnakt/.nvm/...` の node の代わりに PATH 上の Node v22 を使った。Tauri をビルドするため GTK／WebKit の開発ライブラリを入れた。
- **CI の範囲**：CI は `cargo test -p local-sights` を実行しないため、`sso_cli` のテストはローカルでしか動かない。CI に足すかどうかは Build and Test 以降で判断する。

## テスト

### 純粋ロジックのテスト先行（Red → Green）

Red は、既定値を返すだけのひな形を置いた状態で取った。

| ステップ | コマンド | Red | Green |
|---|---|---|---|
| Step 2 SSO の判定 | `cargo +stable test -p local-sights-core catalog` | 19 passed／5 failed | 24 passed |
| Step 3 ログイン先と出力の取り出し | `cargo +stable test -p local-sights-core sso_login` | 8 passed／13 failed | 21 passed |
| Step 4 ログインの状態 | `cargo +stable test -p local-sights-core session` | 89 passed／12 failed | 101 passed |

Red の時点で通っていたのは、「何も返さない」「保持しない」を確かめる否定のテストで、ひな形でも満たされるもの。

### 増えたテストの数

| 対象 | 増えた数 |
|---|---|
| コア catalog | +7 |
| コア sso_login | +21 |
| コア session | +14 |
| コア gateway（`clear_clients`） | +1 |
| Tauri 側 `sso_cli`（偽の CLI スクリプト） | 12 |
| 画面 `SsoLoginButton` | 4 |
| 画面 `SsoLoginModal` | 10 |
| 画面 既存部品（LogGroupPane +2、FetchErrorBanner +1、TopBar +1、App +2、messages +1） | 7 |

`sso_cli` のテストは 3 回続けて実行して安定していた。このテストで実際の不具合を 2 つ見つけて直した。

- CLI がすぐ終わると、読み取り途中の確認コードが失われていた。
- アプリ終了で止めた CLI が、Cancelled ではなく Failed と報告されていた。

### 全体の検査（Step 8）

| 検査 | 結果 |
|---|---|
| prettier | `--write` のあと `--check` で全ファイル一致 |
| eslint | exit 0、警告なし |
| tsc --noEmit | exit 0 |
| vitest run | 31 ファイル、291 passed（前は 270） |
| vite build | 成功。チャンクサイズの警告は以前からのもの |
| `cargo +stable fmt --all --check` | exit 0 |
| `cargo +stable clippy --workspace --all-targets` | 警告 0 |
| `cargo +stable test --workspace` | 全通過：local_sights_app 12、core lib 359、u1 11、u2 10、u3 14、u5 4、u6 3。ignored 5 件は以前からのもの |
| `npm audit --audit-level=high` | 0 vulnerabilities |

### NFR1 の確認（grep）

対象は `crates/*/src`、`crates/*/tests`、`src-tauri/src`、`src`、各 `Cargo.toml` と `package.json`。

| 調べたもの | 結果 |
|---|---|
| SSO・SSO-OIDC の API と crate（`aws_sdk_sso`、`aws_sdk_ssooidc`、`ssooidc`、`CreateToken`、`StartDeviceAuthorization`、`RegisterClient`、`GetRoleCredentials` など） | 0 件 |
| トークンキャッシュのパス（`sso/cache`、`.aws/sso`） | 0 件 |
| AWS の API 呼び出し | `gateway/aws.rs` の `get_log_events`、`describe_log_groups`、`describe_log_streams` の 3 つだけ |

`Cargo.lock` にある `aws-sdk-sso`／`aws-sdk-ssooidc` は、以前から `aws-config` が認証チェーンのために引き込んでいるもので、今回の変更とは関係ない。

## 手動確認（Mac の実機でのみ確かめられる）

| 項目 | 状態 |
|---|---|
| 直接の SSO プロファイルでログインし、一覧が読み直される | 未確認 |
| AssumeRole のプロファイル（source_profile が SSO）でログインできる | 未確認 |
| ブラウザが開かないとき、ダイアログの URL と確認コードでログインできる | 未確認 |
| キャンセルで `aws` のプロセスが残らない | 未確認 |
| 5 分の時間切れで `aws` のプロセスが残らない | 未確認 |
| ログイン中の Cmd+Q で `aws` のプロセスが残らない | 未確認 |
| ログイン中の Dock の「終了」で `aws` のプロセスが残らない（`RunEvent::Exit` が届くか） | 未確認 |

手順は README.md のチェックリストにある。
