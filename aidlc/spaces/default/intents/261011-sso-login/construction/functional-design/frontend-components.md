# Frontend Components — アプリから SSO ログインする

既存の画面（Cloudscape の部品）に、次を足す。状態はコアが持ち、画面はセッションの表示用データから描く（既存の流儀）。

## 部品

| 部品 | 置き場所 | 中身 | 状態の出どころ |
|---|---|---|---|
| SsoLoginButton | 左ペインの一覧のエラー表示の下、取得の失敗のアラートの中 | Cloudscape の `Button`（「SSO にログイン」）。エラーが AuthRequired でログイン先があるときだけ出す（BR1.5）。作業中・ログイン中は無効（BR2.5） | セッションの `ssoLogin.available`、`busy` |
| SsoLoginModal | 画面中央 | Cloudscape の `Modal`。Waiting：案内文、`Spinner`、URL（`Link`、外部で開く）、確認コード（大きめの文字）、[キャンセル]。結果：時間切れ・失敗・CLI なしの文言と [閉じる]。閉じるボタン（×）と Escape は Waiting の間は使えない | セッションの `ssoLogin.state`、`verificationUrl`、`userCode`、`exitCode` |
| 成功の知らせ | 右ペインの上 | Cloudscape の `Alert type="success" dismissible`「ログインしました」 | `ssoLogin.state = Succeeded` |

## 操作の流れ

1. SsoLoginButton を押す → コマンド `start_sso_login` を呼ぶ。
2. コアの状態が Starting・Waiting になり、SsoLoginModal が開く。上部バーのプロファイル・リージョン・設定、条件エリアの [取得]、左ペインの再読み込みと選択が無効になる（BR3.4）。
3. URL と確認コードが届くたびにダイアログが更新される。
4. [キャンセル] → コマンド `cancel_sso_login`。
5. 結果に応じてダイアログを閉じる、または [閉じる] を押すとコマンド `dismiss_sso_login`。

## 文言（英・日）

| key | 日本語 | 英語 |
|---|---|---|
| ssoLogin.button | SSO にログイン | Sign in with SSO |
| ssoLogin.title | SSO にログイン | Sign in with SSO |
| ssoLogin.waiting | ブラウザでログインを承認してください。ブラウザが開かない場合は、次の URL を開いて確認コードを入力してください。 | Approve the sign-in in your browser. If the browser does not open, open this URL and enter the code. |
| ssoLogin.code | 確認コード | Verification code |
| ssoLogin.cancel | キャンセル | Cancel |
| ssoLogin.close | 閉じる | Close |
| ssoLogin.succeeded | ログインしました。ロググループの一覧を読み込み直しています。 | Signed in. Reloading the log groups. |
| ssoLogin.timedOut | 時間切れになりました。もう一度ログインしてください。 | The sign-in timed out. Try again. |
| ssoLogin.failed | ログインできませんでした（終了コード {code}）。 | Could not sign in (exit code {code}). |
| ssoLogin.cliNotFound | AWS CLI が見つかりません。AWS CLI v2 をインストールしてください。 | AWS CLI was not found. Install AWS CLI v2. |
| ssoLogin.fallback | ターミナルで aws sso login {profileOption} を実行してから、[再読み込み] を押してもログインできます。 | You can also run aws sso login {profileOption} in a terminal and press [Reload]. |

## アクセシビリティ

- ボタンとダイアログの操作はキーボードで届く。ダイアログを開いたら焦点はダイアログの中へ移り、閉じたら押したボタン（なくなっていれば左ペインの再読み込み）に戻る（NFR7）。
- 待機中は `Spinner` に加えて文字で状態を示す。
