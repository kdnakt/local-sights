# Domain Design 質問票

上流の成果物：`inception/requirements-analysis/requirements.md`（FR1〜FR8、NFR1〜NFR16）、`inception/practices-discovery/team-practices.md`（GUI に依存しないライブラリと GUI アプリを分け、AWS 呼び出しは trait の裏に置く）。

このステージでは、書くコードの「部品（コンポーネント）」の分け方を決める。どのフレームワークを使うか、1 つのアプリにまとめるか（配置）は後のステージで決める。

たたき台の部品案（質問の前提）：

| 部品案 | 役割 | 主な要件 |
|--------|------|----------|
| 接続設定（ConnectionCatalog） | プロファイル一覧・リージョン一覧・既定リージョン | FR1 |
| AWS 窓口（CloudWatchLogsGateway） | 読み取り 3 API を呼ぶ境界（trait）と、その AWS SDK 実装 | NFR6、NFR14 |
| ロググループ一覧（LogGroupBrowser） | ロググループの列挙と名前での絞り込み | FR2 |
| 時間範囲（TimeRangeModel） | 日時の解釈、タイムゾーン変換、夏時間、終了日時の境界 | FR3 |
| 取得（Fetch 系） | ストリームの列挙と時刻での絞り込み、ページング、再試行、部分失敗 | FR4 |
| ログの保持（EventTimeline） | 時刻順の並び、逐次追加、100 万件の保持 | FR4.7、NFR2 |
| 絞り込み（Filter） | 部分一致の絞り込み | FR6 |
| キャッシュ（LogCache） | ディスクキャッシュの読み書き・範囲判定・削除 | FR7 |
| 画面（GUI） | 画面・操作・表示言語・キーボード操作 | FR3.4、FR4.8、FR4.9、FR5、FR8、NFR11〜NFR13 |

## Q1. 取得の部品は、1 つにまとめますか、分けますか？

背景：取得は「ストリームを列挙して時刻で絞る（FR4.1〜FR4.2）」と「各ストリームのページをたどり、再試行し、部分失敗をまとめる（FR4.3〜FR4.5、FR4.10）」の 2 段階です。MVP 後の最優先は横断取得の高速化（並列化など、IB-07）です。

A. 2 つに分ける：ストリームの選定（StreamPlanner）と、ページ取得・再試行（EventFetcher）。高速化のときに取得側だけを差し替えやすい
B. 1 つにまとめる（FetchService）。部品が少なく、最初は作りやすい
X. Other (please specify)

[Answer]: A. 2 つに分ける：ストリームの選定（StreamPlanner）と、ページ取得・再試行（EventFetcher）。高速化のときに取得側だけを差し替えやすい

## Q2. 絞り込みは、ログの保持と同じ部品にしますか、分けますか？

背景：絞り込み（FR6）は保持しているログ（FR4.7）に対して行います。MVP 後に Logs Insights 互換のクエリ（IB-11）を足す予定で、それは絞り込みの発展形です。

A. 分ける：絞り込み（FilterEngine）は「ログの並びと条件を受け取り、合う行を返す」部品にし、保持（EventTimeline）とは別にする。後でクエリに置き換えやすい
B. 同じ部品にする：保持している部品が絞り込みも担う（データの近くで処理でき、速度を出しやすい）
X. Other (please specify)

[Answer]: A. 分ける：絞り込み（FilterEngine）は「ログの並びと条件を受け取り、合う行を返す」部品にし、保持（EventTimeline）とは別にする。後でクエリに置き換えやすい

## Q3. 画面の状態とルール（[Fetch] を押せる条件、取得中のロック、絞り込みの引き継ぎ、終了確認など）は、どこに置きますか？

背景：FR3.4、FR4.8、FR4.9、FR6.4 は画面の振る舞いですが、ルールとしてはテストできます。開発ルールでは「GUI に依存しない層の公開関数には必ずテストを書く」としています。

A. GUI に依存しない「画面の状態」部品（AppSession）をライブラリ側に置き、ルールはそこでテストする。GUI はその状態を表示し、操作を伝えるだけにする
B. GUI の中に置く（部品が少ない。ルールのテストは GUI の手動確認になる）
X. Other (please specify)

[Answer]: A. GUI に依存しない「画面の状態」部品（AppSession）をライブラリ側に置き、ルールはそこでテストする。GUI はその状態を表示し、操作を伝えるだけにする

## Q4. キャッシュを使うか AWS から取るかの判断は、どの部品が持ちますか？

背景：キャッシュは任意・既定無効で、範囲に含まれれば使い、失敗・途中終了した取得は記録しません（FR7.4、FR7.9）。

A. 取得の部品が判断する：取得の入口で先にキャッシュを見て、なければ AWS から取り、成功したときだけキャッシュに書く（呼ぶ側はキャッシュを意識しない）
B. 画面の状態（AppSession）が判断する：キャッシュを見てから取得を呼ぶ。取得の部品はキャッシュを知らない
X. Other (please specify)

[Answer]: A. 取得の部品が判断する：取得の入口で先にキャッシュを見て、なければ AWS から取り、成功したときだけキャッシュに書く（呼ぶ側はキャッシュを意識しない）

## Q5. エラーの表示文（英語・日本語）は、どこで作りますか？

背景：エラーは「何が起きたか」と「次の行動」を文字で示し（FR8.1）、表示言語は英語と日本語（NFR12）、秘密の認証情報とアクセスキー ID は出しません（FR8.3）。

A. ライブラリはエラーの種類（例：認証切れ、権限不足、通信断、スロットリング）と安全な詳細だけを返し、文の組み立てと翻訳は画面側で行う
B. ライブラリが英語・日本語の表示文まで作って返す
X. Other (please specify)

[Answer]: A. ライブラリはエラーの種類（例：認証切れ、権限不足、通信断、スロットリング）と安全な詳細だけを返し、文の組み立てと翻訳は画面側で行う

## Follow-up Questions

## F1. Q4「取得の入口でキャッシュを見る」の入口は、どの部品にしますか？

背景：Q1 で取得をストリームの選定（StreamPlanner）とページ取得・再試行（EventFetcher）に分けたため、「キャッシュを見る → なければ選定と取得 → 全ストリーム成功時だけキャッシュに書く」という流れを束ねる入口が必要になる。

A. 取得の流れを束ねる小さな部品（FetchCoordinator）を足す。キャッシュの判断、選定と取得の呼び出し、ログの保持への逐次追加、成功時のキャッシュ書き込みを担う。StreamPlanner と EventFetcher はキャッシュを知らない
B. StreamPlanner を入口にして、キャッシュの判断と EventFetcher の呼び出しも担わせる（部品は増えない）
X. Other (please specify)

[Answer]: A. 取得の流れを束ねる小さな部品（FetchCoordinator）を足す。キャッシュの判断、選定と取得の呼び出し、ログの保持への逐次追加、成功時のキャッシュ書き込みを担う。StreamPlanner と EventFetcher はキャッシュを知らない

## Consolidated Summary Confirmation

回答のまとめ：

- 取得は 2 つの部品に分ける：ストリームの選定（StreamPlanner）と、ページ取得・再試行・部分失敗のまとめ（EventFetcher）（Q1）
- 絞り込み（FilterEngine）は、ログの保持（EventTimeline）とは別の部品にする（Q2）
- 画面の状態とルール（[Fetch] の条件、取得中のロック、絞り込みの引き継ぎ、終了確認）は、GUI に依存しない画面の状態部品（AppSession）としてライブラリ側に置く。GUI は表示と操作の伝達だけ（Q3）
- キャッシュを使うかどうかは取得側が判断し、呼ぶ側はキャッシュを意識しない（Q4）。その入口として取得の流れを束ねる部品（FetchCoordinator）を足し、キャッシュの判断・選定と取得の呼び出し・保持への逐次追加・全ストリーム成功時だけのキャッシュ書き込みを担う。StreamPlanner と EventFetcher はキャッシュを知らない（F1）
- ライブラリはエラーの種類と安全な詳細だけを返し、英語・日本語の文は画面側で作る（Q5）
- 上記以外の部品はたたき台どおり：接続設定（ConnectionCatalog）、AWS 窓口（CloudWatchLogsGateway）、ロググループ一覧（LogGroupBrowser）、時間範囲（TimeRangeModel）、キャッシュ（LogCache）、画面（GUI）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
