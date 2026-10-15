## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u1-concurrent-skeleton/nfr-requirements/reliability-requirements.md > NFR7.1、および traceability.json の NFR7 と NFR6 の行 | 要件分析の NFR7 は「CI で行カバレッジ 80% 以上」だが、NFR7.1 は「既存のテストが通ったまま」という別の内容になっている。traceability.json は NFR7 を OK とし、80% のカバレッジの床はどの成果物にも出てこない。NFR6 のうち「純粋なロジック（速さの調整、あきらめの数え方、N の数え方、経過の表記）はテストを先に書く」も、NFR6.1（AWS に接続しない）だけが引き継がれて落ちている。対応づけが実際の中身と合っていない | NFR7 の子 ID として、CI で行カバレッジ 80% 以上（下回る PR はマージしない）を測定方法つきで足す。既存テストが通る要件は別の ID にするか NFR1 系へ移す。NFR6 には、CallPacer・あきらめの数え方などの純粋なロジックをテスト先行で書くことを追加する。traceability.json の対応先を直す | New |
| R-02 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u1-concurrent-skeleton/nfr-requirements/performance-requirements.md > NFR2.1、observability-requirements.md > NFR2.3 | 手元の main と U1 の比較は、手順が実行できる形になっていない。main の FetchCheck には要約の行も経過時間の表示もなく（NFR2.3 は U1 の版にだけ効く）、main の「かかった時間」の測り方が書かれていない。機能設計 5 節によれば、main の FetchCheck にはリージョンを指定する引数が無く、U1 で足される。そのため main 側はプロファイルの既定リージョンで動き、東京でなければ上限が違う（指定が無いと 1 秒 5 回）。さらに、1 回ずつの測定で「U1 のほうが短ければ合格」とすると、ネットワークのばらつきで合否が揺れる。同じ範囲・同じストリーム本数の条件も書かれていない | NFR2.1 に手順を足す。例：両方の版を同じリージョン（東京、明示）・同じ時間範囲・同じ条件で、時間の測り方を決めて（例：両版とも外側で同じ方法で計るか、main 側に同じ要約を一時的に当てる）、複数回（回数を決める）実行して比べる。テスト用ロググループの条件（ストリーム数の目安）と、件数が一致する確認方法も書く。数字の目標を置かない決まりは保ってよい | New |
| R-03 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u1-concurrent-skeleton/nfr-requirements/scalability-requirements.md > NFR3.4、security-requirements.md > NFR6.1、tech-stack-decisions.md | 「クライアントの設定を組み立てて確かめるだけ」では、GetLogEvents の呼び出しごとの再試行の上書きは確かめにくい（上書きは操作の組み立ての中にあり、設定の値を読み出す道が無い）。実際に確かめるには、偽の HTTP クライアントで 5xx を返し、試行が 1 回だけであることを数える形になる。tech-stack-decisions は「足すのは futures-util の 1 つだけ」としているが、テストでの偽の HTTP クライアントの作り方（開発用に足す依存の有無）は決まっていない | NFR3.4 の確かめ方を、偽の HTTP クライアントで試行回数を数える形に書き直し、既存の開発用依存（aws-smithy-runtime-api）で足りるか、開発用の依存が増えるかを tech-stack-decisions に明記する | New |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u1-concurrent-skeleton/nfr-requirements/scalability-requirements.md > NFR1.4 | 「かかる時間は本数 × 1 本あたりの呼び出し数 ÷ 速さの上限 程度になる」には、許容幅も条件も確かめ方も無く、合否を決められない。数十〜数百本という範囲だけが書かれている | 数値の許容幅（例：理論値に対する割合）と偽物の条件を決めてテストにするか、測らない参考の見込みとして明記し、合否の要件から外す | New |
| R-05 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u1-concurrent-skeleton/nfr-requirements/security-requirements.md > NFR8.4、NFR8.5、observability-requirements.md > NFR2.2、NFR2.3、NFR2.4、NFR8.6、traceability.json | 子 ID の親が内容と合っていない。NFR8 は「進み具合・中断・失敗の表示に秘密を出さない」だが、NFR8.5（新しい依存の cargo-deny）は依存の安全の話で、NFR8.4 はテレメトリを送らない話。NFR2 は「手元で時間を目で確かめる」だが、NFR2.2〜2.4 は診断ログの要件（質問票 Q1 から生まれた新しいもの）。また NFR3.1 は NFR3.2 と、NFR8.6 は NFR8.3 とほぼ同じ内容を別の ID で重ねている | 診断ログと依存の要件に合う親を選び直すか、新しい要件として由来（Q1、team.md Code Style）を明記して親を付け直す。重複する行は片方に寄せて ID を一本化する | New |
| R-06 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u1-concurrent-skeleton/nfr-requirements/observability-requirements.md > NFR2.2、security-requirements.md > NFR8.3 | 診断ログの項目に「終わりの状態（JobStatus）」が入っているが、質問票 Q1 の回答 A と最後の確認の要約には載っていない項目で、承認された範囲より広い。値は決まった語で秘密を含まないので危険は無いが、承認との食い違いは残る | Q1 の項目に合わせて外すか、追加の項目として要件に由来を明記する。いずれの場合も、NFR8.3 の単体テストの許可する語の一覧を、状態の語を含めて固定する | New |

### Summary

秘密の扱い（読み取り 3 API だけ、診断ログは数値と決まった語だけ、AWS に接続しない、外へ送らない）は、project.md の Forbidden と機能設計・コードの現状と矛盾せず、security-requirements は実装に進める水準にあります。速さの数字（12 回／秒、最小間隔 83,333,334 ナノ秒、100 本で約 8.5 秒、10 秒以下、同時 8 本）も互いに整合していることを確かめました。一方で、NFR7（カバレッジ 80%）と NFR6 のテスト先行が対応づけから落ちていること、手元の main 対 U1 の比較の手順が実行できる形になっていないことの 2 点が Major です。Major は 2 件のため規則上は READY としますが、承認の前に直すことを勧めます。
