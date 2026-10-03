# Unit of Work — CloudWatch Logs ローカルビューア

上流の成果物：`inception/domain-design/components.md`（部品 12 個）、`decisions.md`（ADR-001〜ADR-008）、`inception/requirements-analysis/requirements.md`（FR1〜FR8、NFR1〜NFR16）。質問票の回答は `units-generation-questions.md` の Q1〜Q3、F1、F2 を指す。依存関係は `unit-of-work-dependency.md`、要件との対応は `unit-of-work-story-map.md`。アーキテクチャレビューの指摘（R-01〜R-06）を受けた人間の判断を反映済み。

このファイルは作業単位の中身と依存の形だけを決める。どの順に作るか（優先度）は決めない。

## 全体の前提

- **配置**：1 つのデスクトップアプリとして、ソースからビルドする（team.md の Deployment）。どの作業単位も、同じアプリに機能を足していく。
- **構成**：GUI に依存しない Rust のライブラリと、Tauri のアプリに分ける（ADR-001、team.md の Code Style）。部品のうち DesktopUi だけが画面側（TypeScript と React）にあり、それ以外はライブラリ側の Rust で書く。
- **GUI フレームワーク**：Tauri（Q3）。画面側は TypeScript と React で書き、大量行の一覧には既存の仮想スクロール部品を使う。CI で型チェック（`tsc --noEmit`）・Prettier・ESLint を必須にする（F2）。Rust 側は team.md のとおり rustfmt 必須・clippy 実行（警告はエラーにしない）。
- **薄い一本**：team.md の Walking Skeleton に従い、最初の作業単位 U1 を、端から端まで動く最小版にする。
- **種類（Kind）**：U1〜U6 は、1 つの実行ファイル（デスクトップアプリ）に機能を足していく単位のため、すべて service とする。単独で動くライブラリだけの単位はない。U7 は主に画面側の仕上げを行うため ui とする。Rust 側で手を入れるのは終了確認のルール（AppSession）だけ（レビュー R-06、R-08）。
- **エラー表示の暫定扱い**：U7 より前の単位では、エラーは種類名と安全な詳細をそのまま表示する暫定の表示にする（秘密の認証情報とアクセスキー ID は出さない）。U7 で「何が起きたか」と「次の行動」の文に仕上げる（レビュー R-08）。
- **ログの置き場所**：取得したログは Rust 側（EventTimeline）だけが持つ。画面側は、表示範囲の行だけを Tauri のコマンドで取り寄せて仮想スクロールの一覧に描く（NFR2、レビュー R-01）。
- **英日の文言とキーボード操作**：U1 で文言をキーで引く仕組みとキーボード操作・Escape の土台を作り、以降の各単位は自分の画面の英日の文言とキー操作を自分で足す（レビュー R-02）。

## 作業単位の一覧

| Unit ID | Directory | 名前 | Kind | 規模 | 配置 |
|---------|-----------|------|------|------|------|
| U1 | u1-walking-skeleton | 薄い一本 | service | M | 単体のデスクトップアプリ（以降の単位の土台） |
| U2 | u2-connection-selection | 接続とロググループの選択 | service | S | U1 のアプリに組み込む |
| U3 | u3-fetch-robustness | 取得の作り込み | service | L | U1 のアプリに組み込む |
| U4 | u4-time-range | 時間範囲とタイムゾーン | service | M | U1 のアプリに組み込む |
| U5 | u5-filter | 絞り込み | service | S | U1 のアプリに組み込む |
| U6 | u6-disk-cache | ディスクキャッシュ | service | M | U1 のアプリに組み込む（外せる） |
| U7 | u7-ui-polish | 画面の仕上げ | ui | M | U1 のアプリの画面側に組み込む |

## U1：薄い一本（u1-walking-skeleton）

- **説明**：GUI を起動し、手入力した条件で GetLogEvents を呼んで結果を表示する最小版（Q2）。GUI・AWS SDK・認証のつなぎ込みの不安を最初に潰す。
- **範囲**：
  - Rust のワークスペース（GUI 非依存ライブラリ＋Tauri アプリ）と、画面側（TypeScript＋React）の土台を作る。
  - プロファイル名・ロググループ名・ストリーム名を手入力し、開始・終了日時を UTC（`yyyy-mm-dd hh:mm:ss`）で入力して [Fetch] を押すと、その 1 つのストリームを GetLogEvents で最後のページまで取得し、時刻・メッセージを一覧表示する。
  - AWS の認証は AWS SDK の仕組み（SSO・アクセスキー・AssumeRole・環境変数）に任せる。
  - 実際の AWS に対して取得結果を表示する、手元用の確認用プログラムを用意する（team.md の Walking Skeleton）。
  - 取得に使うミリ秒範囲の算出（終了日時はその秒の終わりまで含む、FR3.5）をここで作り、U3・U6 も同じものを使う（レビュー R-04）。
  - AppSession → FetchCoordinator（最小版：1 ストリームを取得して渡すだけ）→ EventFetcher の形を最初から作る。U3 は中身を広げるだけにする（ADR-001、ADR-004、レビュー R-05）。
  - 英日の文言をキーで引く仕組みと、キーボード操作・Escape でダイアログを閉じる土台を作る（レビュー R-02）。
  - 画面側の型チェック（`tsc --noEmit`）・Prettier・ESLint の設定、`npm audit` による依存関係のチェック、`package-lock.json` のコミットを用意する。CI への組み込みは CI Pipeline ステージで行う（F2、レビュー R-03）。
- **含む部品**：CloudWatchLogsGateway（GetLogEvents と、その偽物）、EventFetcher（開始・終了時刻の指定とページ終端の判定）、FetchCoordinator（最小版）、TimeRangeModel（UTC の日時の解釈とミリ秒範囲の算出）、AppSession（最小限）、DesktopUi（最小限、文言とキーボード操作の土台を含む）。
- **含まない**：一覧からの選択（U2）、複数ストリーム・再試行・部分失敗・中断・逐次表示・仮想スクロールの一覧（U3）、タイムゾーン切替（U4）、絞り込み（U5）、キャッシュ（U6）、行の展開・ダークモード・終了確認（U7）。
- **確認方法**：`cargo test` がすべて通る／確認用プログラムを実際の AWS に対して手元で実行する／GUI を起動して取得結果を目で確かめる。その後、人間が骨組みのチェックポイントを承認する（team.md）。
- **実装上の注意**：
  - 呼ぶ API は GetLogEvents だけ。読み取り 3 API 以外は呼ばない（project.md Forbidden）。
  - 自動テストと CI は実際の AWS に接続しない。テストは CloudWatchLogsGateway の偽物で行う（project.md Forbidden、ADR-002）。
  - エラー表示と診断ログに秘密の認証情報とアクセスキー ID を出さない（project.md Forbidden、ADR-006）。
  - ページ終端の判定は純粋なロジックとしてテストを先に書く（team.md の Testing Posture）。

## U2：接続とロググループの選択（u2-connection-selection）

- **説明**：手入力をやめ、プロファイル・リージョン・ロググループを一覧から選べるようにする（F1）。
- **範囲**：AWS 共有設定ファイルからのプロファイル一覧、SDK の公開リージョン一覧と既定リージョン、DescribeLogGroups による全ロググループの列挙、名前での絞り込み、接続を変えたらロググループ一覧を取り直す（FR1.1、FR1.3、FR1.4、FR2）。
- **含む部品**：ConnectionCatalog、LogGroupBrowser、CloudWatchLogsGateway（DescribeLogGroups の追加）、AppSession と DesktopUi の該当部分。
- **実装上の注意**：認証情報そのものは読み出さず、プロファイル名と既定リージョンだけを扱う（NFR5）。

## U3：取得の作り込み（u3-fetch-robustness）

- **説明**：選んだロググループの中の関係するストリームをすべて取得し、ストリームを跨いだ時刻順で逐次表示する。取得の頑丈さを作り込む（F1、Q1）。
- **範囲**：DescribeLogStreams によるストリームの列挙と、最初・最後のイベント時刻による選定（余裕 1 時間、時刻なしは含める）、スロットリング時の待機と再試行、ストリーム単位の部分失敗のまとめ、取得の中断（終了時の内部処理）、取得したログの逐次追加と時刻順の保持（100 万件）、表示範囲の行だけを取り寄せる Tauri のコマンドと仮想スクロールの一覧（100 万件をここで確かめる）、取得中のロックと状態表示、件数と失敗数の表示（FR4.1、FR4.2、FR4.5、FR4.7、FR4.8、FR4.10、FR4.11、NFR2〜NFR4、レビュー R-01）。
- **含む部品**：StreamPlanner、EventFetcher（再試行・失敗記録・中断）、EventTimeline、FetchCoordinator（U1 の最小版を広げる。キャッシュの分岐は U6）、CloudWatchLogsGateway（DescribeLogStreams の追加）、AppSession と DesktopUi の該当部分。
- **実装上の注意**：時刻順の並び・ストリームの選定・再試行は純粋なロジックとしてテストを先に書く（team.md）。依存は AppSession → FetchCoordinator の一方向で、進み具合は開始時に渡した受け口へ届ける（ADR-007、ADR-008）。

## U4：時間範囲とタイムゾーン（u4-time-range）

- **説明**：日時の入力とタイムゾーンを要件どおりに仕上げる。
- **範囲**：ローカルと UTC の切替（既定はローカル）、切り替えても同じ瞬間を保つ、夏時間で存在しない日時は入力の誤り・2 回現れる日時は早い方、[Fetch] を押せる条件と理由の表示、ログ一覧の時刻表示の切替（FR3.2〜FR3.4、FR3.6）。終了日時の境界（FR3.5）は U1 で作ったものを使う（レビュー R-04）。
- **含む部品**：TimeRangeModel（U1 の分を広げる）、AppSession と DesktopUi の該当部分（上部バーの TZ 切替、日時入力）。
- **実装上の注意**：タイムゾーン変換・夏時間・終了境界は純粋なロジックとしてテストを先に書く（team.md）。

## U5：絞り込み（u5-filter）

- **説明**：取得済みのログを部分一致で絞り込む。
- **範囲**：部分一致の絞り込み、「絞り込み後 / 全件」の表示、再取得しても絞り込み文字列を引き継ぐ、10 万件で 10 秒以内・100 万件で 100 秒以内（U3 の仮想スクロールの一覧で確かめる）（FR6、NFR1、NFR2）。絞り込み中は、U3 の表示範囲の取り寄せコマンドと同じ形で、絞り込み結果（FilterResult）の行だけを返す。具体的な形は Functional Design で決める（レビュー R-07）。
- **含む部品**：FilterEngine（FilterCondition、FilterResult）、AppSession と DesktopUi の該当部分。
- **実装上の注意**：AWS の API は呼ばない（FR6.2）。大文字・小文字の区別は Functional Design で決める（FR6.5）。

## U6：ディスクキャッシュ（u6-disk-cache）

- **説明**：利用者が選んだときだけ取得結果をディスクに保存し、範囲が含まれれば再利用する。設定ダイアログを含む。期限が厳しいときは、この単位ごと外せる（FR7.8）。
- **範囲**：キャッシュの有効・無効（既定は無効）、保存場所（`~/Library/Caches/` 配下）、範囲が含まれるときの再利用、全ストリーム成功時だけの書き込み、中断・部分失敗時は書かない、有効期限なし、全削除、設定ダイアログ（英日の文言とキーボード操作もこの単位で仕上げる）（FR7、レビュー R-02）。
- **含む部品**：LogCache、FetchCoordinator のキャッシュの分岐、AppSession と DesktopUi の設定ダイアログ。
- **実装上の注意**：秘密の認証情報とアクセスキー ID はキャッシュに書かない（project.md Forbidden）。無効時はディスクに書かない（NFR8）。

## U7：画面の仕上げ（u7-ui-polish）

- **説明**：画面を要件どおりに仕上げる。
- **範囲**：英語・日本語の文言の見直しと OS の言語に合わせた切替の確認、主な流れのキーボード操作の通し確認、OS に合わせたダークモード、最小ウィンドウ 1024×640、行を押すとその行の直下に全文を展開（複数可・コピー可・JSON 整形なし）、エラーの種類から「何が起きたか」と「次の行動」の文を作り、それまでの暫定の表示を置き換える（色やアイコンだけにしない）、取得中にウィンドウを閉じるときの確認（[Keep fetching] / [Close]）、診断ログは標準エラー出力だけ（FR1.5、FR4.9、FR5.2〜FR5.4、FR8.1、FR8.2、NFR11〜NFR13、NFR16）。
- **含む部品**：DesktopUi（仕上げ）、AppSession の終了確認のルール。文言の仕組みとキーボード操作の土台は U1、各画面の文言とキー操作は各単位が作る（レビュー R-02）。U6 の設定ダイアログは U6 で仕上げるため、U7 は U6 に依存しない。
- **実装上の注意**：見た目は OS・部品の標準のままにし、独自のデザインは作らない（NFR10、project.md Corrections）。取得中であることの表示は U3 で入れる最低限のものを保つ（project.md Corrections）。

## 部品と作業単位の対応

| 部品 | 主に作る単位 | ほかに手を入れる単位 |
|------|--------------|----------------------|
| ConnectionCatalog | U2 | — |
| CloudWatchLogsGateway | U1（GetLogEvents） | U2（DescribeLogGroups）、U3（DescribeLogStreams） |
| LogGroupBrowser | U2 | — |
| TimeRangeModel | U4 | U1（UTC の解釈とミリ秒範囲の算出） |
| StreamPlanner | U3 | — |
| EventFetcher | U3 | U1（ページ終端の判定まで） |
| EventTimeline | U3 | — |
| FilterEngine | U5 | — |
| LogCache | U6 | — |
| FetchCoordinator | U3 | U1（最小版）、U6（キャッシュの分岐） |
| AppSession | U1（最小限） | U2〜U7（それぞれの機能のルール） |
| DesktopUi | U7 | U1（土台・文言とキーボードの仕組み）、U2〜U6（それぞれの機能の画面と文言）、U3（仮想スクロールの一覧） |
