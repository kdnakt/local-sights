## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-04T06:11:56Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u1-walking-skeleton/functional-design/functional-spec.md > 2. ワークフロー UC1 手順 6・8.2・10・12、UC2 手順 4 | FetchCoordinator が呼び出し元に渡す「受け口」の中身が定義されていない。手順 10・11 では status・件数・ページ数・ApiFailure だけを受け口に届けるとあるが、手順 12 の一覧（AppSession 経由）と UC2 手順 4（確認用プログラムが LogEvent を 1 件ずつ出力）には LogEvent 自体が呼び出し元に届く経路が必要。EventTimeline は U1 の部品に含まれず、FetchOutcome は完了時にしか存在しない（status は Completed／Failed のみ）ため、取得中・確認用プログラムでは誰がどのタイミングで LogEvent を保持し渡すのかを実装者が推測するしかない。 | 受け口の契約を決める：受け取るもの（LogEvent を逐次か、完了時にまとめてか）、呼ばれる順序、Failed のときに途中までの LogEvent をどう渡すか（BR4.1）、UC1 と UC2 で同じ受け口を使うこと。U1 で LogEvent を保持する部品を 1 つ明記する。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u1-walking-skeleton/functional-design/entities.md > FetchOutcome・PageCursor・ConnectionTarget・FetchRequest・MessageCatalog・LogEvent（owner）・SessionState。functional-spec.md > UC1 手順 7 | 共有の components.md との整合が取れていない。(a) components.md の FetchJob・StreamFetchOutcome に対応する U1 のエンティティが FetchOutcome・PageCursor と別名で作られ、対応関係も U3 での移行も書かれていない。(b) LogEvent の持ち主を EventTimeline としているが、unit-of-work.md の U1「含む部品」に EventTimeline はない。(c) SessionState から components.md の識別子 sessionId が落ちている。(d) ConnectionTarget の持ち主は CloudWatchLogsGateway だが、UC1 手順 7 は FetchCoordinator が接続先を決めてリージョンなしの RegionMissing を出すとしており、責任の置き場所が食い違う。U3 が「広げるだけ」（unit-of-work.md）で済まなくなる恐れがある。 | 各エンティティを components.md のエンティティ（FetchJob、StreamFetchOutcome、LogEvent、SessionState）に対応付けるか、別名にした理由と U3 での置き換え方を書く。接続先の決定と RegionMissing の発生を担う部品を 1 つに決めて、手順 7 と entities.md の owner をそろえる。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u1-walking-skeleton/functional-design/functional-spec.md > 5. ER 図 | ER 図が entities.md から導かれていない。図にある「ConnectionTarget と FetchRequest の 1 対 1 の関係」と「PageCursor から FetchOutcome への多対 1 の関係」は、entities.md では ConnectionTarget・PageCursor とも relationships が空で、元のデータにない。また PageCursor の多対 1 は、U1 が 1 ストリームだけであることと合わない。 | entities.md の relationships に両方の関係（多重度と向き）を足すか、図から外す。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u1-walking-skeleton/functional-design/functional-spec.md > 4. 画面（U1 の最小版）の最終段落、rules.md > BR3.4 | ステージ定義の「技術に依存しない（フレームワークへの言及なし）」に反する記述がある。functional-spec.md 4 章に「TypeScript＋React」「Tauri のコマンドとイベント」、BR3.4 に「trait」。選定自体は unit-of-work.md で決定済みだが、この設計成果物には書かない。 | 「画面と AppSession を、状態の読み取りと操作の伝達だけでつなぐ」といった技術中立の表現にし、技術名は unit-of-work.md と ADR を参照する。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u1-walking-skeleton/functional-design/rules.md > BR5.1。entities.md > LogEvent.sequence | requirements.md 7 章が Functional Design で決めるとした「同じ時刻のイベントの並び順（FR4.7）」の決め方が曖昧。sequence は「同じ時刻の並び順を決める」とあり、時刻が主、sequence が副の並びに読めるが、BR5.1 は「sequence の昇順」だけを並び替えの鍵にしている。API の返却順が時刻順でない場合、どちらに従うか決まらない。 | 並び替えの鍵（時刻、次に sequence）か、API の返却順をそのまま使うかを BR5.1 に 1 つに決めて書き、U3 の複数ストリームの並びにつながる形にする。 | New |
| R-06 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u1-walking-skeleton/functional-design/functional-spec.md > UC2 手順 2、1. 部品のつながり（図）。rules.md > BR1.1 | UC2 は BR1.1〜BR1.3 と「同じ検証」を求めるが、BR1.1 の適用先は AppSession が持つ FetchRequest で、確認用プログラムは AppSession を通らない（図では FetchCoordinator を直接使う）。検証と TimeRange の算出を誰が担い、画面と確認用プログラムでどう共有するかが書かれていない。 | 検証（必須・形式・前後関係）と TimeRange 算出を、画面と確認用プログラムの両方から使える部品（TimeRangeModel など）に置くと決めて書く。 | New |
| R-07 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u1-walking-skeleton/functional-design/rules.md > BR3.2。functional-spec.md > 3. 状態遷移の表 | 端の振る舞いが未定義。(a) 応答に次のトークンが含まれないときの扱い（終了か InvalidInput か）。(b) pageCount に、トークンが同じで終了と判定した最後の応答を含めるか。(c) 状態表では Done・Failed の [Fetch] が無条件に有効とあるが、手順 3 と BR1.1〜BR1.3 の検証は常に効くはずで、表と食い違う。 | (a)(b) を BR3.2／PageCursor に 1 行ずつ決めて書き、(c) の表を「検証を通ったときだけ押せる」に直す。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| sensor-traceability（functional-design / u1-walking-skeleton） | PASS: gaps 0、orphans 0、invalid_targets 0 | U1 の FR（FR1.2、FR3.1、FR3.5、FR4.3、FR4.4、FR4.6、FR5.1、FR8.3）はすべて BR に対応し、BR のうち FR のないものは reverse で説明されている。構造上の問題はない。上の指摘は構造ではなく中身（契約・持ち主・整合）に関するもの。 |

### Summary

トレーサビリティは機械的に通り、ルールの内容（ページ終端、終了時刻の +1 ミリ秒、エラーの安全な詳細）も要件に沿っている。ただし、FetchCoordinator の受け口の契約と LogEvent の保持場所が未定義（R-01）で、エンティティの持ち主も components.md と食い違っている（R-02）。Major が 2 件で READY の基準内だが、承認の前にこの 2 件を直すことを強く勧める。
