# Project-Level Rules

> Project-specific specialisation and corrections. Loaded after `org.md` and
> `team.md` as strict-additive guidance; contradictions with broader policy
> are rejected. Populated by practices-discovery and the self-learning loop.
>
> Use sparingly: most teams don't need a project layer. Reach for it
> only when this specific project needs stable, durable guidance beyond the
> team practice (for example, package-specific release checks or an additional
> regression suite for a legacy component).

## Way of Working

<!-- Project-specific specialisation. Example: -->
<!-- This monorepo requires package-scoped branch names and a package owner -->
<!-- review in addition to the team's normal merge policy. -->

## Walking Skeleton

<!-- Project-specific specialisation. Example: -->
<!-- The walking skeleton must exercise the legacy service adapter as well -->
<!-- as the new service boundary. -->

## Testing Posture

<!-- Project-specific specialisation. -->

- クエリの速さの合否の数値は、利用者の答え（作る段階で決める）に従って要件では決めなかった（NFR1） (learned 2026-10-17) <!-- cid:261017-insights-query:requirements-analysis:179cee86658388289375b911f1f578f4a35afddd4c4e72233a29597869858202 -->

- 依頼 (4) の権限のテストの固定値は依頼文の 26 ではなく今は 32 だったため、数の固定そのものをやめて 3 か所の一覧の一致だけを確かめることにした（Q8 A）。 (learned 2026-10-20) <!-- cid:261020-carryover-fixes:requirements-analysis:9250f691f870af3ed67b8591162fc45d99ac9c4b955ce0ea7010e62c2d5b62bd -->

## Guard Policy

<!-- Project-specific. Mode: strict, relaxed, or off. Strict here holds for every intent and cannot be changed from chat. A section under the retired Change Control heading, written by an earlier release, is still read. -->

## Deployment

<!-- Project-specific specialisation. -->

## Code Style

<!-- Project-specific specialisation. -->

## Tech Stack

<!-- Technology choices locked for this project. -->

## Decided

<!-- Decisions made in earlier stages that should not be re-asked. -->
<!-- Format: DECIDED: [decision] (Stage [slug], [date]) -->

## Scope Overrides

<!-- Custom scope rules for this project. -->

## Forbidden

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: NEVER [behavior] (affirmed [date]) -->
<!-- Example: NEVER throw exceptions across service layer boundaries (affirmed 2026-05-17) -->

- NEVER 秘密の認証情報（シークレットアクセスキー・セッショントークン・SSO のトークン）とアクセスキー ID を、リポジトリ・アプリのログ・画面のエラー表示・キャッシュファイルに書き込む（プロファイル名・ロール名／ロール ARN・アカウント ID はログやエラー表示に出してよいが、リポジトリには置かない） (affirmed 2026-10-03)

- NEVER CloudWatch Logs の読み取り API（DescribeLogGroups / DescribeLogStreams / GetLogEvents）以外の API を呼ぶ (affirmed 2026-10-03)

- NEVER 自動テストや CI から実際の AWS に接続する、または CI に AWS の認証情報を置く (affirmed 2026-10-03)

- NEVER テレメトリやクラッシュレポートを外部に送る (affirmed 2026-10-03)

## Mandated

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: ALWAYS [behavior] (affirmed [date]) -->
<!-- Example: ALWAYS use Result<T,E> for fallible operations in service layer (affirmed 2026-05-17) -->

None. (affirmed 2026-10-03)

None. (affirmed 2026-10-11)

None. (affirmed 2026-10-12)

None. (affirmed 2026-10-15)

None. (affirmed 2026-10-20)

## Corrections

<!-- Project-specific corrections from human feedback. -->
<!-- Format: NEVER/ALWAYS [behavior] (learned [date]) -->
- 「Rust製」「GetLogEvents API」は利用者自身が指定した制約として扱い、ideation 成果物でも実装詳細ではなく前提条件として残す (learned 2026-10-02) <!-- cid:261002-cwlogs-viewer:intent-capture:74d4398b24a4f112f577207863f10250ccf36e3bb70cba67a834eb98c33dc7ee -->
- ALWAYS 質問は一問ずつ構造化質問で提示する（利用者が Chat モードを選んでも、論点をまとめて投げず一問ずつ聞く） (learned 2026-10-03) <!-- cid:261002-cwlogs-viewer:feasibility:05a9352b368ecb7c5eb634bb983900899de8d8f71c8fdc06ff5f6fe29b4b0e75 -->
- ALWAYS 進捗表示を対象外にした場合でも、処理中であることを示す最低限の状態表示（例：Fetching 表示とボタン無効化）は画面に含める (learned 2026-10-03) <!-- cid:261002-cwlogs-viewer:rough-mockups:a49c7eff6e1d6606f95c463f6bdc84b675b0e15c6d554db57e55516124d026b7 -->
- デスクトップ GUI の MVP は、見た目を標準部品のみ・作り込みなしとして期限を優先する (learned 2026-10-03) <!-- cid:261002-cwlogs-viewer:rough-mockups:b9a3c28dbc3b57f7f1b6aa4f5d8e57dbea355bb53f4f21dd2e76aa55c5430c24 -->
- ALWAYS 部品（コンポーネント）は書くコードだけにし、AWS SDK・外部 API・ファイルシステム・GUI フレームワークは外部依存として扱い、それらに触れる境界のコードを部品にする (learned 2026-10-03) <!-- cid:261002-cwlogs-viewer:domain-design:735533c17cfc1b86b464424e28676424e2c8c9a75125564547127189b4c88c20 -->
- ALWAYS 後のステージに先送りした技術選定でも、次の作業（例：薄い一本で GUI を動かす）に必要になった時点で質問して決める (learned 2026-10-03) <!-- cid:261002-cwlogs-viewer:units-generation:b5d44bcd5056cb51e60285d4db0301a99fb4119aeb6817cd775df432a6f60855 -->
- ALWAYS 薄い一本をさらに薄くして外れた機能は、既存の作業単位に詰め込まず、独立した作業単位として切り出す (learned 2026-10-03) <!-- cid:261002-cwlogs-viewer:units-generation:8a8e9098c04b2f6a206b6c6ff98433829c4295d90a08c0a80d76161c508e13bf -->
- ALWAYS レビュー回数の上限に達した後に出た指摘や、レビューを記録した後に気づいた修正は、その作業単位では直さず次の作業単位に回す（作業単位のチェックポイントはレビュー済みの内容との一致を求めるため） (learned 2026-10-04) <!-- cid:261002-cwlogs-viewer:functional-design:b9f0a9e7db912fc99ec48e2f1b217c684a4c71d2a260354f920166d54e4cfdfb -->
- [u1-walking-skeleton] 終了日時はその秒の 999 ミリ秒まで含むと解釈した（FR3.5） (learned 2026-10-08) <!-- cid:261002-cwlogs-viewer:functional-design:df7c08e7c951275ddae77ffc672074b5c24919fe84eef2d0d160cec5abecc352 -->
- [u1-walking-skeleton] 途中でエラーが起きても取得できたページは残して表示する（Q3） (learned 2026-10-08) <!-- cid:261002-cwlogs-viewer:functional-design:fba1c09f17dd4e7e2b3f87446342946d1360bf1a3fbe177fc0774e28e75f5835 -->
- [u1-walking-skeleton] U1 では件数の上限を設けず全件を取得・表示する（Q4） (learned 2026-10-08) <!-- cid:261002-cwlogs-viewer:functional-design:e5aa541d5199c8b0f3a3d1e35c6734f0dda827094e64dfa7565592c3dd9d078d -->
- [u2-connection-selection] 接続を変えるとき、表示中のログがあるときだけ確認ダイアログを出す（Q6 の利用者の追加要望） (learned 2026-10-08) <!-- cid:261002-cwlogs-viewer:functional-design:57ebd43fe68119647b5a71a8683754a87a0c2c60db85c4cbc69814e4ca83a126 -->
- [u2-connection-selection] 起動時は何も選ばず、前回の選択も覚えない（Q2） (learned 2026-10-08) <!-- cid:261002-cwlogs-viewer:functional-design:d5aff9e10d6a4f59eaceca08a07e1c016d4ee722af07db2db6449de11e2be4c9 -->
- [u3-fetch-robustness] ストリームは 1 つずつ順に取得し、再試行は要求ごとに最大 5 回、使い切りが 3 ストリーム続いたら残りを失敗とする（Q1、レビュー R-03） (learned 2026-10-08) <!-- cid:261002-cwlogs-viewer:functional-design:6d59d767a9363c5268cead56212e8b6684687044cf966255c1ab8c4d6c10c221 -->
- 依頼文の中の「〜したほうがいいかも？」のような相談は、根拠（例：マネジメントコンソールの振る舞い）を添えた推奨つきの追加質問で確認して決める (learned 2026-10-10) <!-- cid:261010-console-ui-polish:intent-capture:40f367fe7c8d1469208a6ccd7773a65f6136edd66a4f4e530d8b7778a4154bd5 -->
- アプリが利用者の操作で AWS CLI の aws sso login を起動することは、Forbidden の「CloudWatch Logs の読み取り API 以外を呼ばない」と「SSO のトークンを書き込まない」の対象外とする。アプリ自身は SSO の API を呼ばず、トークンを読み書きしない (learned 2026-10-11) <!-- cid:261011-sso-login:intent-capture:b2155b7843696ad2ed4315f53ce0950625508101b5fef4a57d001979208f5710 -->
- 選択肢の説明に添えた例の数値（例：「半分以下」）は確定した目標値とみなさず、測る条件と数値は後のステージで決める (learned 2026-10-12) <!-- cid:261012-parallel-fetch:intent-capture:0057eaf5e0e1b5719946a1ae0544ced3b84ee9b61be2d1e88549b9c2f666d36c -->
- 「含めないもの」の質問で選ばれなかった項目は、含めるとも含めないともみなさず、追加の質問で確かめる (learned 2026-10-12) <!-- cid:261012-parallel-fetch:intent-capture:7294424c81c605a135be8bd6e7b9479d249332912f3659665b0181f236684857 -->
- ALWAYS CI のジョブ名を変えるときは、ブランチの保護ルールの必須チェックの名前と合わせる（PR #20 で名前を変えてマージできなくなった） (learned 2026-10-13) <!-- cid:261012-parallel-fetch:code-generation:42c27d7572a097e518f636f0c205a6d03693f4cadb5891375e6cbc90393ba6bc -->
- [windows-support] 「macOS 版と同じ」は、今の macOS 版にある機能（取得・表示・絞り込み・キャッシュ・SSO ログイン・中断・経過時間）が Windows でも同じように使えることと解釈する。見た目の細部（メニューの位置など）を OS の慣習に合わせてよいかは要件分析で決める (learned 2026-10-15) <!-- cid:261015-windows-support:intent-capture:2d4ac3be590ff524518810e85ec1b2b02f049634b5aa0e820718d443281a7a20 -->
- [windows-support] マージ済みの取得の高速化（PR #22）と Windows 版は合わせて v0.2.0 で配る。v0.1.2 のタグは付けない（practices-discovery Q5） (learned 2026-10-15) <!-- cid:261015-windows-support:practices-discovery:31eb34738d8b5966169f1d9da26d9672a091fa2b3a2499f2c228d69aab4bd7c6 -->
- 古いコード知識ベース（parallel-fetch のとき）は Windows 対応などの後で古くなっていたため、利用者の選んだ全面の読み直しで 9 つの文書を作り直した (learned 2026-10-17) <!-- cid:261017-insights-query:reverse-engineering:95ef0198bfb0b69690975c0518b9c4289b6963d7ecbebbe6f7166cb3a92702d3 -->
- コンソールの振る舞いの細部（fields の既定の列、sort の既定、型の違う値の比較など）は要件では「コンソールと同じ」とだけ書き、中身は Functional Design で AWS の資料から確かめることにした（OQ2） (learned 2026-10-17) <!-- cid:261017-insights-query:requirements-analysis:419e40ce8b21876881d41a32d352e3205205674310736a5e953f31449c091f43 -->
- ラフな画面設計がないため、要件と今の画面（261010 の mockups と今のコード）から画面を起こした (learned 2026-10-17) <!-- cid:261017-insights-query:refined-mockups:9d52652f7d7c5c57f9588557755c208f2b2a55df45fad83eeb814bfd906c596b -->
- 期間とロググループの選択は 2 つの画面で別々に持つことにした（取得済みのログだけを共有） (learned 2026-10-17) <!-- cid:261017-insights-query:refined-mockups:5259c99276b5f4b17776439fde4549fa47560438df4505f2d8f8b93840c706de -->
- クエリ欄は利用者の選択どおり最初から CodeEditor（Ace）にした (learned 2026-10-17) <!-- cid:261017-insights-query:refined-mockups:97d255501f19db213a8ee12e5f5ddfbabbe215c969d2f43ce6e1e4e593037c8b -->
- 部品の一覧には新しく作る部品と変える部品だけを載せ、変えない部品（FilterEngine など）は codekb の一覧のままとした (learned 2026-10-17) <!-- cid:261017-insights-query:domain-design:766a7c4ba7c4b78c2ea5688d6dd037d37ab2cd84a7149ab997d2cd3a867c118d -->
- 範囲の定義の B5「クエリの中で期間とロググループを指定」を、クエリの画面に置く専用の欄で指定することと読み替えた（A1） (learned 2026-10-17) <!-- cid:261017-insights-query:requirements-analysis:5a1635440369bc5d17d1f42ac879d0cb20314de0ecca16013394185acfe406da -->
- [insights-query] 2 つの画面は取得済みのログを共有しないと見直した（RA Q3b、Q3a の見直し）。普段は片方の画面で済むため、同じログを 2 回取得しメモリも 2 つ分持つことを受け入れた (learned 2026-10-17) <!-- cid:261017-insights-query:requirements-analysis:6c9acb731de9834c403d122751199640fe0d2b5faf6e6b98caad88bca0b29c89 -->
- [insights-query] 要件の見直し（RA Q3b：取得済みのログを共有しない）に合わせ、画面設計は「2 つの画面の間の約束」・画面 4 の注記・補完の候補元だけを直し、画面の構成は変えなかった (learned 2026-10-17) <!-- cid:261017-insights-query:refined-mockups:6435c6759b97dded6f5b9d65bcf1dca0dc145fc258b6a7815bbb0fa649b2f067 -->
- [insights-query] 訂正：期間とロググループの選択も取得済みのログも、2 つの画面で別々に持つ（RA Q3b。前の「取得済みのログだけを共有」を改める） (learned 2026-10-17) <!-- cid:261017-insights-query:refined-mockups:2d6e1d0f9f42b74d02de1a8022e398f5c8da21c9807fe291b1323a4aae95a979 -->
- [insights-query] 要件の見直し（RA Q3b）で共有の保持（LogStore）をやめ、クエリの画面専用の QueryStore を作り、取得は今の FetchCoordinator に「捨てずに足す」入口を足して使うことにした（Q6） (learned 2026-10-18) <!-- cid:261017-insights-query:domain-design:e16daac4ec568dcfafc6baf66f92384ceb0eb1ef7ad7c7a672b6c5a8b4260165 -->
- [insights-query] U1 は QueryStore の最小版（1 つのロググループ、実行のたびにクエリの画面の保持だけを捨てて取り直す）で取得し、範囲の記録と取り足し・キャッシュの一部の利用・複数のロググループを U3 に残した（Q2a） (learned 2026-10-18) <!-- cid:261017-insights-query:units-generation:5ccb8545ad330b77dd3886f3a7d521e3165addc0e6a43250707d68e2f97a768b -->
- この計画は refactor を元にしており骨組みのチェックポイントはないが、team.md の Walking Skeleton にならって U1 を画面から AWS まで通る薄い一本にした (learned 2026-10-18) <!-- cid:261017-insights-query:units-generation:e7fb210fa0da9791f386e4181fbc90035528dea7408cccdf87c4b48d019cc748 -->
- 確認用プログラムは利用者の答え（言語がそろったあとに独立）に合わせて 4 つ目の作業単位 U4 にした (learned 2026-10-18) <!-- cid:261017-insights-query:units-generation:114fb65608c86aa19603267d50e9699c0460f6f4384332a4897db0d6b6ef03f3 -->
- [insights-query] @log の値はロググループ名だけとした。コンソールの「アカウント ID:ロググループ名」にするには読み取りの 3 つ以外の API が要り呼べないため、@log の値を比べるクエリは互換の確かめの組に入れない（U1 の機能設計） (learned 2026-10-18) <!-- cid:261017-insights-query:functional-design:f2b384a42bfdf5a2cda0f9e8ba1fda562ee5a27ab66f8d59f10c11860f9a71ab -->
- [insights-query] 言葉の大文字・小文字、優先順位（not > and > or）、文字列の項目と数値の比較、sort がないときの並び順は AWS の資料にないため U1 では仮の決まりにし、U2 の機能設計（OQ2）と U4 でコンソールに合わせる (learned 2026-10-18) <!-- cid:261017-insights-query:functional-design:d26950ece5247c9b8677a9781f73e90fbd64e5e3864dcbf1d022667179b4ea5c -->
