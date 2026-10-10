## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-05T03:34:06Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u4-time-range/functional-design/rules.md > BR3.4、BR3.1、BR1.2 | BR3.4 は「画面とライブラリは同じローカルのタイムゾーンを使う」と宣言するだけで、ローカルのタイムゾーンをどこから得て、どちらが変換するかを定めていない。既存実装では入力の解釈は Rust（chrono、`clock` のみ）、ログ一覧の時刻は webview の TypeScript（`src/format.ts` の `formatUtcMillis`、JS の Date）にあり、ローカルの変換が Rust と JS の 2 か所に分かれ得る。Rust は OS の設定（`TZ` 環境変数を含む）を、webview はその実行環境の設定を見るため、端末から `TZ=` 付きで起動した場合などに、入力の解釈と一覧の表示で同じ瞬間が違う時刻になり得る。また BR3.4 の「タイムゾーンを決めた値（夏時間のある地域）に差し替えて確かめる」ための手段も未定義で、chrono-tz などの IANA 時刻帯データはワークスペースの依存に無く、JS 側の差し替え方も書かれていない。 | 変換の持ち主を 1 つに決めて書く。例：ローカルの変換（解釈・作り直し・行の時刻の文字列化）はすべて Rust の TimeRangeModel が行い、webview は文字列を表示するだけにする。または、Rust が決めた IANA 名か UTC オフセット表を webview に渡して同じ値で変換する。あわせて、テストでタイムゾーンを差し替える境界（例：オフセット検索の trait）と、夏時間のある地域のデータをどこから得るかを BR3.4 または functional-spec §7 に書く。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u4-time-range/functional-design/rules.md > BR1.4、BR1.3；entities.md > DateTimeInput.error；functional-spec.md > §3 DateTimeInput 状態遷移 | BR1.4 は、瞬間を持たない入力（形式の誤り・存在しない日時）の文字列と error をそのまま残す。しかし NonexistentLocalTime は「ローカルで解釈したときだけ」成り立つ誤りで、UTC に切り替えると同じ文字列は正しい UTC の日時になる（例：`2024-03-10 02:30:00`）。それでも error = NonexistentLocalTime のまま残り、タイムゾーンが UTC なのに「夏時間の切り替えで存在しない日時です」が出て [Fetch] が押せず、BR1.3 の「instant は text を選んだタイムゾーンで解釈できたときに持つ」という不変条件にも反する（functional-spec の text fallback もこの状態を認めている）。Q1 の選択は「変換できない入力」の扱いを聞いたもので、切り替え後には変換できる入力まで含めてしまっている。 | 少なくとも NonexistentLocalTime は、切り替えのたびに新しいタイムゾーンで解釈し直す（UTC なら instant を持つ、ローカルでまた存在しなければ error のまま）と定め、状態遷移図（Nonexistent → Valid の切替遷移）と受け入れ条件を足す。Q1 の趣旨（形式の誤りは文字列と誤りをそのまま残す）を変えたくなければ、その旨を人間に確認し、「error = NonexistentLocalTime は timeZone = Local のときだけ取り得る」という不変条件を entities.md に明記する。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u4-time-range/functional-design/rules.md > BR1.2、BR2.2、BR3.2；functional-spec.md > §4 画面 | 既存の画面は入力欄のラベル（`form.start.label`「Start (UTC)」「開始（UTC）」）と形式の誤りの文言（`validation.startFormat` など「（UTC）」）に UTC を固定で書いている。U4 の規則は列の見出し（BR3.2）と存在しない日時の文言（BR2.2）しか定めず、入力欄のラベル・形式の誤りの文言・入力例の扱いを置き換える規則がない。実装すると、ローカルのときに「開始（UTC）」と出る不整合が残る。 | 入力欄のラベルと形式の誤りの文言から「（UTC）」を外すか、選んだタイムゾーンに合わせて「（ローカル）」「（UTC）」に切り替える規則を足し、既存文言キーの置き換えを列挙する。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u4-time-range/functional-design/rules.md > BR2.1 | BR2.1 の条件の並びに「接続の変更の確認待ちでない」（U2:BR2.6、`can_fetch` は確認待ちも見ている）がなく、「取得中でない」も理由として示す文言キーが ValidationError にも SelectionError にも定義されていない。FR3.4 は「理由を文字で示す」と定めているため、取得中・確認待ちのときに [Fetch] が無効で理由が空になり得る。 | 取得中と確認待ちを理由として出すか（文言キーを定義する）、Fetching 表示などの別の文字表示が理由を兼ねると明記する。条件の並びに確認待ちを加える。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u4-time-range/functional-design/rules.md > BR1.4、BR3.1 | 表現できない範囲の扱いが未定義。UTC の `0000-01-01 00:00:00` や `9999-12-31 23:59:59` は、ローカルのオフセットを足すと年が 0 未満や 9999 超になり、`yyyy-mm-dd hh:mm:ss` の文字列を作り直せない。ログ行の時刻にも、U1 は範囲外では生の数値を出す規則があるが、ローカルでの扱いは書かれていない。 | 作り直しができないときの扱い（例：文字列は UTC のままにして error を持たせない、または切替を拒む）と、行の時刻が範囲外のときは U1 と同じく生の数値を出すことを書く。 | New |
| R-06 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u4-time-range/functional-design/rules.md > BR1.4；functional-spec.md > UC1 受け入れ条件 | 2 回現れる時刻の遅い方の瞬間は、ローカルに切り替えると早い方と同じ文字列で表示され、利用者には区別できない（瞬間は保たれるが、表示だけでは違いが見えない）。さらにその欄を触らずに再解釈する経路（例：BR1.5 による入力イベント、フォーカスの出入りで値が再送される実装）があると、遅い方の瞬間が早い方に黙って変わる。 | 「作り直した文字列は、利用者が書き換えない限り解釈し直さない（値の変化があるときだけ解釈する）」ことを BR1.5 に明記し、同じ文字列への再入力で instant を変えないテストを加える。 | New |
| R-07 | Minor | aidlc/spaces/default/intents/261002-cwlogs-viewer/construction/u4-time-range/functional-design/traceability.json > coverage FR3.1、FR3.5、reverse | FR3.1 と FR3.5 は U4 で BR1.2・BR1.6 が意味を広げるのに status が `N/A` で、他の単位の「実装済み」と同じ扱いになっている。BR1.1〜BR1.3 も FR3.2 以外では目的の ID に現れず、整合の確認が取りにくい。 | FR3.1・FR3.5 を `OK`（U1 の振る舞いを U4 で置き換える）に変え、target に BR1.2・BR1.5・BR1.6 を挙げる。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| なし（この単位の functional-design にはスキーマ検証ツールの指定がなく、実行していない） | - | 参照整合は手作業で確認した。BR1.1〜BR3.5 は rules.md・spec・traceability の間で一致している。ENT の名前、ValidationError の種類、BR の参照先も解決する。 |

### Summary

夏時間の意味づけ（存在しない日時は誤り、2 回現れる日時は早い方、切替は瞬間を保つ）と、U1〜U3 のルールとの接続は概ね整合している。ただし、ローカルのタイムゾーンを Rust と webview がそれぞれ別に解決し得る点（R-01）と、UTC に切り替えても NonexistentLocalTime が残る不変条件の破れ（R-02）の 2 つの Major は、Code Generation に入る前に直すのが望ましい。Major は 2 件で閾値以内のため READY とするが、R-01・R-02 は承認前に判断してほしい。
