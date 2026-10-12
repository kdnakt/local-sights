## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-12T02:12:00Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261011-sso-login/construction/functional-design/functional-spec.md > UC6 / rules.md > BR3.3 | 既存の閉じる確認（closeConfirmation）は「取得中」と「キャッシュ書き込み中」だけを対象にしている（src-tauri/src/lib.rs の説明）。ログイン待ち（Waiting）が確認の対象に入るのか、確認なしで CLI を止めて終わるのかが UC6 の「既存の閉じる確認の流れのあと」では決まらない。Dock の Quit は終了を検知も止めもできない既知の制限があり、その経路では CLI が残る。終了も「プロセスを終了させ、終了を待つ」だけで、シグナルの種類、待つ時間の上限、プロセスグループ単位かが未定（上流レビュー R-04 の「機能設計で決める」が未解消）。 | UC6 に、Waiting 中の閉じる操作で確認を出すか出さないかを決めて書く。終了の手順（SIGTERM、待ち時間の上限、SIGKILL へのエスカレーション、孫プロセスを含むプロセスグループ単位の終了）を BR3.3 に足す。Dock の Quit の経路でも CLI が残らないよう、子プロセスを親の終了に連動させる手段（例：Tauri 側のハンドルの Drop で kill）を決め、手動確認（7 章）に Dock の Quit を加える。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261011-sso-login/construction/functional-design/rules.md > BR3.1 / frontend-components.md > SsoLoginModal | URL の許可リストは「https:// で始まる最初の語」だけで、ホストやパースの検証がない。画面の Link で外部のブラウザに渡す方法（Tauri の opener 等）と、そこに通す URL の条件も書かれていない（現在のコードには外部リンクを開く仕組みが見当たらない）。CLI の出力は信頼できる前提でも、行末の句読点や制御文字、userinfo 付きの URL、CLI の出力にほかの https:// が混ざる場合に、そのまま開くと意図しない先へ飛ぶ。また最新の AWS CLI v2 は PKCE の認可コード方式で、確認コードを出さず認可 URL（state と code_challenge を含む長い URL）だけを出すことがある。デバイスコード方式では URL が 2 本（コード入りと無し）出る。どの URL を「最初の 1 つ」とするかが方式で変わる。 | URL を URL としてパースし、scheme が https、host があり、userinfo と制御文字を含まないものだけを受け付ける条件を BR3.1 に足す。ホストの制限をするか（例：amazonaws.com / awsapps.com）しないかを決めて理由を書く。外部で開く手段と、開く前に同じ検証を通すことを frontend-components.md に書く。確認コードが出ない方式を正常系として明記し（ssoLogin.waiting の文言は「確認コードを入力」を前提にしないようにする）、URL が複数ある場合の選び方（コード入りを優先するか、最初か）を決める。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261011-sso-login/construction/functional-design/functional-spec.md > UC1 手順 6 / rules.md > BR4.1 | SDK クライアントのキャッシュは core の AwsCloudWatchLogsGateway が持つ（Mutex の HashMap）が、spec は「Tauri 側が捨てる」としている。ゲートウェイに破棄の API（例：clear_clients）はなく、trait の裏のため Tauri から直接は触れない。破棄だけで十分かも確かめられていない（SDK の資格情報プロバイダが SSO のキャッシュを毎回読み直すかは、クライアントを作り直すと新しいプロバイダになるので足りるはずだが、手動確認に入っていない）。 | CloudWatchLogsGateway（または別の小さな trait）に「キャッシュしたクライアントを捨てる」操作を足すと明記し、置き場所をコアにする。成功後に一覧が読み込めることを 7 章の手動確認に明示する。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261011-sso-login/construction/functional-design/functional-spec.md > 3 状態の遷移 | 遷移表に抜けがある。(1) Starting 中のキャンセル・アプリ終了。(2) Waiting で CLI の終了、キャンセル、5 分経過がほぼ同時に起きたときの勝ち負け（先に起きた 1 つを採用し、あとは無視する、と決めていない）。(3) 成功の知らせのあと一覧の再読み込みが失敗した場合（AuthRequired のままなら再びボタンが出る。成功の Alert と失敗の表示が同時に並ぶ）。(4) 一覧が表示中の状態で再読み込みしたとき、選択中のロググループやストリームがどうなるか。 | 遷移表に Starting 中のキャンセルの行を足すか、Starting ではキャンセルを受けないと書く。最初に起きた終了要因だけを採用する規則を足す。再読み込み失敗時は成功の Alert を消すか、失敗の表示を優先するかを決める。再読み込みで選択を保つか捨てるかを 1 行で決める。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261011-sso-login/construction/functional-design/rules.md > BR2.2 / BR3.1 | CLI の起動条件が足りない。標準入力を閉じる（Stdio::null）、標準出力と標準エラーの両方を読む（片方だけ読むとパイプが詰まってブロックしうる）、CLI が出力を行単位でフラッシュするか（パイプ接続だと出力が遅れうる）が書かれていない。出力が遅れて URL が見えない場合の挙動が「案内だけを出す」に落ちるため、致命的ではないが、確認用の手動試験が必要。 | BR2.2 に stdin を null にすること、stdout と stderr の両方を並行して読み捨てること（許可リストを通す）を足す。7 章に「URL がダイアログに出るまでの時間」を確かめる項目を足す。 | New |
| R-06 | Minor | aidlc/spaces/default/intents/261011-sso-login/construction/functional-design/rules.md > BR1.1 / BR1.2、既存の catalog/mod.rs の SectionReader | 現在の SectionReader は region 以外の行を全部捨てる作りで、source_profile と SSO のキーの有無を持つにはこの関数を拡張する必要がある。BR1.2 は「値は読まずに捨てる」と書くが、拡張後も ProfileSections の型（キーの有無だけを持つ形）が秘密を持たないことを保つテストの方針が書かれていない。 | 拡張の方針（hasSsoSettings は bool だけ、sourceProfile は名前だけ）に対する純粋ロジックのテストを先に書く、とコード生成への申し送りに足す。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| なし（このステージには検証ツールの指定がないため実行していない） | — | 参照の確認は手作業で行った。traceability.json の FR1.1〜NFR4 の対応先（BR1.1〜BR4.3）は rules.md に存在し、内容も合っている。FR3.3 の変更（許可リスト方式との両立のため終了コードと代わりの手順を出す）は spec の 6 章に記録されていて妥当。 |

### Summary

境界（コア＝判定と状態、Tauri 側＝プロセスと外部、trait の裏で本物の CLI と AWS を呼ばない）は既存の設計に収まっていて、AWS_PROFILE・デフォルト・source_profile の扱い（BR1.3、BR1.4、BR2.3）も SDK と CLI で一致する。上流レビューの Major 2 件（デフォルトと AWS_PROFILE、ログイン中の切り替え）は BR1.3 と BR3.4 で解消された。Major 2 件（アプリ終了時の確認と CLI の確実な停止、URL の検証と外部で開く手段）は実装前に決めておくのが望ましいが、Major は 2 件以下で実装を止めるほどではないため READY とする。
