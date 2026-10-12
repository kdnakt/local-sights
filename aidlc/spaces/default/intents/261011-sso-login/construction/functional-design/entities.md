# Entities — アプリから SSO ログインする

上流：`inception/requirements-analysis/requirements.md`（FR1〜FR4、NFR1〜NFR7）、`functional-design-questions.md`（FD Q1〜Q3）。作業単位は 1 つ。既存のコードのプロファイル一覧（ConnectionCatalog）とセッション（AppSession）を拡張する。

```yaml
entities:
  - name: ProfileAuthInfo
    description: >
      プロファイル 1 つ分の、ログインに関わる設定の要約。AWS の設定ファイル
      （~/.aws/config）の該当セクションから作る。秘密情報や SSO の設定値は持たない。
    owner: プロファイル一覧（コアの catalog）
    attributes:
      - name: profileName
        type: text
        required: true
        constraints: 設定ファイルの [default] または [profile <名前>] の名前
      - name: hasSsoSettings
        type: boolean
        required: true
        default: false
        constraints: セクションに sso_session または sso_start_url のキーがあれば true。値は読まずに捨てる
      - name: sourceProfile
        type: text
        required: false
        constraints: セクションの source_profile の値（プロファイル名）。名前だけを持ち、それ以外の値は持たない
    relationships:
      - to: ProfileAuthInfo
        via: sourceProfile
        cardinality: many-to-one
        direction: このプロファイル → 元のプロファイル

  - name: SsoLoginTarget
    description: >
      「SSO にログイン」を押したときに、CLI に渡すログイン先。選択中のプロファイルから
      source_profile をたどって、SSO の設定を直接持つプロファイルに行き着いたもの。
    owner: コアの SSO ログイン判定
    attributes:
      - name: selectedProfile
        type: enum-or-text
        required: true
        allowed_values: [SdkDefault, Named(<名前>)]
        constraints: 上部バーで選んでいるプロファイル
      - name: effectiveProfileName
        type: text
        required: true
        constraints: SdkDefault のときは環境変数 AWS_PROFILE、なければ "default"。Named のときはその名前
      - name: loginProfileName
        type: text
        required: true
        constraints: effectiveProfileName から source_profile をたどって最初に hasSsoSettings が true になったプロファイル
      - name: profileArgument
        type: text
        required: false
        constraints: CLI に --profile で渡す名前。CLI が自分で同じプロファイルを選ぶ場合（SdkDefault で loginProfileName が effectiveProfileName と同じ）は渡さない

  - name: SsoLoginAttempt
    description: 1 回のログインの試み。同時に 1 つだけ存在する。
    owner: コアのセッション（状態）と Tauri 側の CLI 起動（実行）
    attributes:
      - name: target
        type: SsoLoginTarget
        required: true
      - name: state
        type: enum
        required: true
        allowed_values: [Starting, Waiting, Succeeded, Failed, TimedOut, Cancelled, CliNotFound]
        default: Starting
      - name: verificationUrl
        type: text
        required: false
        constraints: CLI の出力から許可リストで取り出した https:// の URL。最初の 1 つだけ
      - name: userCode
        type: text
        required: false
        constraints: CLI の出力から取り出した確認コード（英大文字 4 字-英大文字 4 字）。最初の 1 つだけ
      - name: startedAt
        type: instant
        required: true
        constraints: 時間切れ（5 分）の起点
      - name: exitCode
        type: integer
        required: false
        constraints: CLI が失敗したときの終了コード
    entity_constraints:
      - SsoLoginAttempt は同時に 1 つだけ（FR2.6）
      - verificationUrl と userCode はアプリのログ・キャッシュに書かない（NFR3）

  - name: AwsCliLocation
    description: 見つかった AWS CLI の実行ファイルの場所。
    owner: Tauri 側の CLI 起動
    attributes:
      - name: path
        type: path
        required: true
        allowed_values: [/opt/homebrew/bin/aws, /usr/local/bin/aws]
```

## 要約

| エンティティ | 役割 | 持たないもの |
|---|---|---|
| ProfileAuthInfo | プロファイルが SSO か、元のプロファイルは何か | SSO の開始 URL・アカウント ID・ロール名などの値、秘密情報 |
| SsoLoginTarget | 選択中のプロファイルから決まる、実際にログインするプロファイル | — |
| SsoLoginAttempt | 1 回のログインの状態、画面に出す URL と確認コード | CLI の出力のその他の行 |
| AwsCliLocation | 見つかった CLI の場所 | — |
