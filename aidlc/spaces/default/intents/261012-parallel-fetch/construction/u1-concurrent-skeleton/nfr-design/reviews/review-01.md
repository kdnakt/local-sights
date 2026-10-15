## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | nfr-design/scalability-design.md の NFR3.4 行、nfr-design/security-design.md の NFR6.1 行 | SDK の再試行の上書きを「再試行の回数を 1 にする」と書いている。AWS SDK の設定は最大試行回数（max_attempts）で、初回の試行を含む。「再試行 1 回」と読むと合計 2 回の試行になり、呼び出しの数が倍になって NFR3.2 の上限を破る。単体テストが「値 1」を確かめるとき、どちらの意味の 1 かが決まっていない（機能設計 BR1.6 は「再試行なし」と書いている） | 「最大試行回数 1（再試行なし）」と言い換え、単体テストが確かめる値を max_attempts = 1 と明記する。上書きを作る関数だけでなく、GetLogEvents の呼び出しにその上書きが実際に付いていることも、可能な範囲で（組み立てた設定の確認で）テストに含める | New |
| R-02 | Minor | nfr-design/observability-design.md の NFR2.2・NFR2.4 行、nfr-design/logical-components.md の FetchCoordinator 行 | FetchStats は FetchCoordinator が作る入れ物だが、calls と throttled は EventFetcher・Retrier の試行の中で数える。FuturesUnordered で最大 8 つの読み取りが借用のまま並ぶ形で、数を入れ物へ渡す経路（Cell・Atomic・PageStep に数を載せる、など）が書かれていない。また、streams が Skipped を含むか、一覧の最初のページの失敗（Failed）・閉じる（Aborted）・0 本のときに要約を出すか、elapsed_ms をどう扱うかも決まっていない | 数の受け渡し方（例：試行ごとに共有の Atomic か Cell を増やす、または PageStep に試行の数を載せて Coordinator が足す）を 1 つ決めて書く。streams の定義と、Failed・Aborted・0 本のときの要約の出し方（出す場合の値）を 1 行ずつ足す | New |
| R-03 | Minor | nfr-design/performance-design.md の NFR2.1 行、nfr-design/nfr-design-questions.md の Q1 | 比べる手順は両方に AWS_REGION だけを付けるとしているが、機能設計（functional-spec 5 節）は U1 の FetchCheck にリージョンを必ず指定する引数を足すとしている。CallPacer の上限は取得の条件のリージョンで決まり、今の FetchCheck は条件のリージョンを未設定にしている。環境変数だけでは SDK のリージョンは合っても、CallPacer が「指定なし＝1 秒 5 回」になり、U1 の版が不当に遅くなる。または引数が必須なら手順どおりに動かない | 手順書に、main の版は AWS_REGION、U1 の版は AWS_REGION と同じ値のリージョン引数の両方を指定することを書く。FetchCheck の引数と環境変数が食い違ったときの扱い（または引数を SDK の設定にも使うこと）を 1 行決める | New |
| R-04 | Minor | nfr-design/scalability-design.md の NFR3.3 行、nfr-design/security-design.md の脅威の表 | CallPacer が保証するのは許可の時刻の間隔で、実際の呼び出しの時刻ではない。取得 1 回のタスクが長く止まると（保持先のロックの待ちなど）、遅れて起きた複数の読み取りがほぼ同時に呼び出し、その 1 秒の区間では上限を超えうる。止めた時計のテストでは見えない。実際の AWS 側の上限は 25 回（東京）で上限の半分に余裕があるため重大ではない | 余裕が実際の上限の半分であることに頼っている旨と、許可の時刻と呼び出しの時刻のずれは許容することを 1 行書く | New |
| R-05 | Minor | nfr-design/scalability-design.md の NFR3.3 行 | 許可の予約に std::sync::Mutex を使う。チーム規約ではテスト以外で unwrap・expect を使わないが、lock の戻り値（毒化）の扱いが書かれていない | 毒化したときは中身をそのまま使う（into_inner）か、取得を失敗として終えるかを決めて書く | New |

### Summary

NFR1.1〜NFR8.6 の 29 件はすべて traceability.json に載り、各設計文書に実装できる仕組みが書かれている。許可の予約（ロックの中で枠を決めてから待つ）、biased な select による止める信号の優先、リージョンの表、PageStep の 1 ページずつの進め方、止める理由の強さの表は機能設計と矛盾しない。診断ログの 1 行は、数値と JobStatus の固定の語しか受け取らない型で組み立てるため秘密の混入を防げ、新しい API・テレメトリ・ファイル書き込みはなく、futures-util の機能指定（default-features = false、std）も妥当である。テストは FakeGateway と止めた時計で AWS に接続せず、tokio の time と test-util、macros は既存の依存で足りる。mermaid の sequenceDiagram は構文として有効で、文字による代替もある。残る指摘はいずれも軽微な明確化で、コード生成の前に直すのが望ましいが、設計の実装を妨げるものはない。
