# Business Rules — アプリから SSO ログインする

```yaml
rules:
  # BR1: ログインできるかの判定
  - id: BR1.1
    statement: プロファイルが SSO のものかは、設定ファイルのそのセクションに sso_session または sso_start_url のキーがあるかで決める
    category: validation
    applies_to: ProfileAuthInfo
    trigger: 設定ファイルの読み込み
    logic: IF セクション内のキーが sso_session または sso_start_url THEN hasSsoSettings = true。値は読まずに捨てる
    violation: なし（判定のみ）
    source: FR1.4、NFR3
  - id: BR1.2
    statement: 設定ファイルから残すのは、セクション名・region・SSO のキーの有無・source_profile の名前だけとする
    category: constraint
    applies_to: ProfileAuthInfo
    trigger: 設定ファイルの読み込み
    logic: IF キーがこの 4 つ以外 THEN その行は保持しない。credentials ファイルからは SSO の情報を読まない
    violation: なし
    source: FR1.4、NFR3、project.md Forbidden
  - id: BR1.3
    statement: 「デフォルト」が指すプロファイルは、環境変数 AWS_PROFILE があればその名前、なければ default とする
    category: policy
    applies_to: SsoLoginTarget.effectiveProfileName
    trigger: ログイン先の決定
    logic: IF 選択が SdkDefault AND AWS_PROFILE が空でない THEN その名前 ELSE "default"
    violation: なし
    source: FR1.2、レビュー指摘（デフォルトと AWS_PROFILE）
  - id: BR1.4
    statement: ログインするプロファイルは、effectiveProfileName から source_profile をたどって最初に SSO の設定を持つプロファイルとする
    category: calculation
    applies_to: SsoLoginTarget.loginProfileName
    trigger: ログイン先の決定
    logic: IF 今のプロファイルが hasSsoSettings THEN それ ELSE IF sourceProfile があり一覧にある THEN そこへ進む（最大 5 段） ELSE ログイン先なし
    violation: 循環・5 段を超える・途中のプロファイルが一覧にない場合は「ログイン先なし」
    source: FR1.2、FD Q2
  - id: BR1.5
    statement: 「SSO にログイン」ボタンは、認証が必要なエラーで、ログイン先があるときだけ出す
    category: policy
    applies_to: 一覧のエラー表示、取得の失敗のアラート
    trigger: エラーの表示
    logic: IF エラーの種類が AuthRequired AND ログイン先あり THEN ボタンを出す ELSE 出さない（今の文言のまま）
    violation: なし
    source: FR1.1、FR1.3

  # BR2: CLI の起動
  - id: BR2.1
    statement: AWS CLI は /opt/homebrew/bin/aws、/usr/local/bin/aws の順に、実行できるファイルを探す。PATH は使わない
    category: policy
    applies_to: AwsCliLocation
    trigger: ボタンの押下
    logic: IF 候補が実行できるファイル THEN それを使う。どれもなければ CliNotFound
    violation: CliNotFound の案内（インストールとターミナルでの代わりの手順）を出す
    source: FR4.1、FR4.2
  - id: BR2.2
    statement: CLI はシェルを通さずに起動し、引数は sso login と、必要なときの --profile <loginProfileName> だけとする
    category: constraint
    applies_to: CLI の起動
    trigger: ボタンの押下
    logic: IF profileArgument あり THEN [sso, login, --profile, 名前] ELSE [sso, login]。名前はプロファイル一覧にあるものに限る
    violation: 一覧にない名前なら起動しない
    source: FR2.1、NFR2
  - id: BR2.3
    statement: --profile は、CLI が自分で同じプロファイルを選ぶ場合は渡さない
    category: calculation
    applies_to: SsoLoginTarget.profileArgument
    trigger: ログイン先の決定
    logic: IF 選択が SdkDefault AND loginProfileName = effectiveProfileName THEN 渡さない ELSE loginProfileName を渡す
    violation: なし
    source: FR1.2、BR1.3
  - id: BR2.4
    statement: 起動する CLI には、アプリの環境変数をそのまま引き継ぐ（AWS_PROFILE などを変えない）
    category: policy
    applies_to: CLI の起動
    trigger: ボタンの押下
    logic: 環境変数は追加・削除しない
    violation: なし
    source: BR1.3（CLI と SDK が同じプロファイルを使うため）
  - id: BR2.5
    statement: ログインは同時に 1 つだけ。ログイン中・取得中・一覧の読み込み中は始めない
    category: constraint
    applies_to: SsoLoginAttempt
    trigger: ボタンの押下
    logic: IF ログイン中 OR 前の試行の CLI がまだ終わっていない（キャンセル後、止まるまでの最大 2 秒を含む） OR 取得中 OR 一覧の読み込み中 THEN ボタンを押せない
    violation: 押せない（無効表示）
    source: FR1.5、FR2.6

  # BR3: 待機
  - id: BR3.1
    statement: CLI の出力から画面に出すのは、AWS のサインインページの URL（許可リストに合うもの）と確認コードの形の文字列だけとする
    category: validation
    applies_to: SsoLoginAttempt.verificationUrl、userCode
    trigger: CLI が 1 行出力するたび
    logic: 行の先頭 4096 バイトを空白で語に分け、各語の前の引用符・括弧と後ろの引用符・括弧・句読点（, . ; :）を取り除く。IF 語が許可リストに合う URL THEN 最初の 1 つを URL とする。許可リストに合う URL とは、https:// で始まり、ユーザー情報もポートも含まず、ホストが英数字・ドット・ハイフンだけで .amazonaws.com または .awsapps.com で終わり（そのものは不可）、全体が 2048 文字以内のもの。IF 英大文字 4 字-英大文字 4 字の語がある THEN 最初の 1 つをコードとする。それ以外は捨てる。URL を開く前にも同じ許可リストで確かめる
    violation: どちらも見つからなければ案内だけを出す
    source: FR2.3、NFR3、レビュー指摘（許可リスト）
  - id: BR3.2
    statement: ログインは 5 分で打ち切る
    category: policy
    applies_to: SsoLoginAttempt
    trigger: startedAt から 5 分
    logic: IF 5 分たっても CLI が終わらない THEN CLI を終了させ TimedOut
    violation: 時間切れの案内を出す
    source: FR2.5
  - id: BR3.3
    statement: キャンセル・時間切れ・アプリの終了では、起動した CLI を必ず終了させる
    category: constraint
    applies_to: CLI のプロセス
    trigger: キャンセル、時間切れ、アプリの終了
    logic: CLI のプロセスを終了させ、終了を待つ
    violation: なし
    source: FR2.4、NFR4
  - id: BR3.4
    statement: ログイン中は、プロファイル・リージョンの切り替え、取得、一覧の再読み込み、設定ダイアログを操作できない
    category: constraint
    applies_to: 上部バー、条件エリア、左ペイン
    trigger: ログイン中
    logic: IF ログイン中 THEN これらを無効にする
    violation: 押せない（無効表示）
    source: FD Q1、FD Q3

  # BR4: 結果
  - id: BR4.1
    statement: ログインが成功したら、キャッシュしている SDK のクライアントを捨て、一覧を自動で読み込み直す。取得は自動ではやり直さない
    category: policy
    applies_to: SsoLoginAttempt.Succeeded
    trigger: CLI が終了コード 0 で終わる
    logic: クライアントを全部捨てる → 「ログインしました」を出す → 一覧の再読み込みを始める
    violation: なし
    source: FR3.1、FR3.2、FR3.4
  - id: BR4.2
    statement: ログインが失敗したら、失敗の旨と終了コードを出し、もう一度押せる状態に戻す。CLI の出力の文章は出さない
    category: policy
    applies_to: SsoLoginAttempt.Failed
    trigger: CLI が 0 以外の終了コードで終わる、または起動できない
    logic: 失敗の旨・終了コード・ターミナルでの代わりの手順を出し、ボタンを押せる状態に戻す
    violation: なし
    source: FR3.3、NFR3
  - id: BR4.3
    statement: SSO の API を呼ばず、~/.aws/sso/cache を読み書きしない
    category: constraint
    applies_to: アプリ全体
    trigger: 常に
    logic: AWS の API 呼び出しは CloudWatch Logs の 3 つのまま。トークンの扱いはすべて CLI と SDK に任せる
    violation: なし
    source: NFR1、project.md
```

## 要約

| ID | 内容 | 出典 |
|---|---|---|
| BR1.1 | SSO かどうかは sso_session / sso_start_url のキーの有無で決める | FR1.4 |
| BR1.2 | 設定ファイルから残すのは 4 種類の情報だけ | FR1.4、NFR3 |
| BR1.3 | 「デフォルト」は AWS_PROFILE、なければ default | FR1.2 |
| BR1.4 | source_profile をたどって SSO のプロファイルに行き着く（最大 5 段） | FD Q2 |
| BR1.5 | ボタンは AuthRequired でログイン先があるときだけ | FR1.1、FR1.3 |
| BR2.1 | CLI は 2 か所を順に探し、PATH は使わない | FR4 |
| BR2.2 | シェルなし、引数は固定 | FR2.1、NFR2 |
| BR2.3 | --profile は必要なときだけ | FR1.2 |
| BR2.4 | 環境変数はそのまま引き継ぐ | BR1.3 |
| BR2.5 | ログインは同時に 1 つ、作業中は始めない | FR1.5、FR2.6 |
| BR3.1 | 画面に出すのは URL と確認コードだけ | FR2.3、NFR3 |
| BR3.2 | 5 分で打ち切る | FR2.5 |
| BR3.3 | 終わり方によらず CLI を終了させる | FR2.4、NFR4 |
| BR3.4 | ログイン中は他の操作を止める | FD Q1、Q3 |
| BR4.1 | 成功したらクライアントを捨てて一覧を読み直す | FR3.1、FR3.2、FR3.4 |
| BR4.2 | 失敗したら終了コードを出して押し直せるように | FR3.3 |
| BR4.3 | SSO の API とトークンに触れない | NFR1 |
