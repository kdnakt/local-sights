## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-18T07:04:17Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261017-insights-query/construction/u3-shared-store/functional-design/rules.md > BR2.2、BR1.2、BR1.4（および functional-spec.md > 1 章の FetchCoordinator の行、UC1 の 7） | 「今の取得の流れの行き先を staging にして使う」とあるが、既存の取得の流れが前提にしている約束と合わない。(1) `run_fetch` は開始時に `TimelineStore::discard()` を呼び（coordinator.rs 372 行）、キャッシュ命中の `serve_from_cache`（797 行）と中止の `abort`（629 行）も同じ。staging や committed をそのまま `TimelineStore` として渡すと、同じロググループの 2 つ目の足りない範囲の取得を始めた時点で、1 つ目の範囲の staging（失敗した分）が消え、FR2.5 の「取得できた分で評価する」を満たさなくなる。(2) `EventTimeline` には `append` と `clear` しかなく、BR1.2 の「staging から committed へ移す」「新しい 5 分だけ staging に残す」を行う操作がない。移さずに両方に置くと BR1.5 の評価の並びで同じイベントが 2 回出る。(3) キャッシュ書き込みは `HeldEvents::copy_events_in_range` で保持中から範囲のイベントを読むが、U3 ではその読み先が staging か committed か決まっていない。ドメイン設計から回された R-03（入口の共通化）が実質未解決のまま残っている | 取得の流れに渡す口を 1 つ決めて書く。例：範囲 1 回ごとに使い捨ての受け皿（`TimelineStore` と `HeldEvents` を満たす）を作り、`discard` はその受け皿だけを空にする。取得が終わったら BR1.2 に従って受け皿から committed と staging へ振り分ける、と手順で示す。キャッシュ書き込みはその受け皿から読むと明記する。必要な `EventTimeline` 側の操作（範囲で分けて取り出す等）も部品の変更として entities.md か rules.md に書く | New |
| R-02 | Major | aidlc/spaces/default/intents/261017-insights-query/construction/u3-shared-store/functional-design/rules.md > BR5.1 | 入れ子の上限が「括弧と not」だけで、U2 のコードの見直し R-01（深い条件でアプリが落ちる）の直しとして足りない。パーサーは `a or b or c ...` と `a and b and ...` を左に偏った木にする（parser.rs 905〜921 行、`while` で `ConditionKind::Or/And(Box::new(left), ...)` を重ねる）。項が N 個なら木の深さは N で、評価（evaluator.rs 106 行付近の再帰）と木の破棄が N 段の再帰になる。括弧も not もない長い or の列は BR5.1 の数え方では 0 段なので、そのまま通ってスタックを使い切りうる。「スタックに十分な余裕がある」という根拠（質問票の表）も、この形には当てはまらない | 上限の対象を「木の深さ」に改める（or と and の連なりも数える）か、条件の項の数に別の上限を置き、超えたら Syntax にする。受け入れの確かめに、括弧なしで項が非常に多い or の列と、100 段ちょうどの入れ子を、Windows のスレッドの大きさでも通るテストとして加える | New |
| R-03 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u3-shared-store/functional-design/rules.md > BR5.3 | メモリの確かめが、ADR-008 と unit-of-work.md の U3「実装の注意」（ログ管理の画面とクエリの画面の両方に持った状態も含めて確かめる）より狭い。「100 万件を 2 つのロググループ」だけで、ログ管理の画面の保持、50 個のロググループ、結果の写し（最大 100,000 件、BR3.2）、評価の並び ViewEntry（1 件あたり 3 つの整数）の分が入っていない | 確かめの条件に、ログ管理の画面にも大きなログを持った状態と、ロググループを多く選んだ状態、結果の写しと評価の並びを含めると書く | New |
| R-04 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u3-shared-store/functional-design/functional-spec.md > UC1 の 7 | あるロググループの取得が Failed になっても残りのロググループを取得し続けるが、BR2.5 では 1 つでも Failed なら評価せず前の結果も消す。認証切れやスロットリングの使い切りなど全体に及ぶ失敗では、最大 50 個のロググループが再試行とあきらめの手順を毎回通ってから、結果なしで終わる。取得済みの範囲は残るため無駄ではないが、待ち時間が長くなりうる | 続ける理由（失敗したロググループをすべて示す）と、全体に及ぶ種類の失敗（認証切れなど）では残りを始めない例外の要否を決めて書く。続けるなら、画面の進み具合（BR2.7）が失敗済みの数を示すことを書く | New |
| R-05 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u3-shared-store/functional-design/rules.md > BR1.4、質問票の表（ストリーム内の順番） | 「今の EventTimeline は 0 から隙間のない番号を前提にしている」と書くが、実際の `EventTimeline::index` は隙間を許し、隙間は `MISSING` で埋める（timeline.rs 150 行付近）。一方 BR1.4 は「staging を捨てても番号は戻さない」ため、committed の番号に隙間が生じ、ストリームごとの索引の長さが、発行した番号の最大まで伸びる。取り直しが続くと索引が増える | 理由の文を実際のコードに合わせて直し、番号を戻さない選択の利点（committed の番号との衝突を避ける）と、索引が伸びる影響を一文で書く。必要なら staging を捨てるときに番号を committed の次から振り直す案も比べる | New |
| R-06 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u3-shared-store/functional-design/rules.md > BR2.1 | 新しい照会「足りない範囲と重なる部分だけを読む」で、今の `lookup` が持つ読み取り中の整合の確認（範囲ごとのイベント数の照合、壊れたファイルの扱い）をどう保つかが書かれていない。記録範囲の途中から切り出すと、範囲全体の件数の照合は、ファイルの先頭から読まないとできない | 新しい照会でも今と同じ確認を行うこと（読めない・壊れているときは AWS から取得する、BR2.1 の記述どおり）と、件数の照合をどの単位で行うかを書く | New |
| R-07 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u3-shared-store/functional-design/functional-spec.md > UC3、rules.md > BR1.2 | UC3 の文が崩れていて（「11:00:00.000〜12:00:59.999 のうち 11:00 の範囲の終わりの次のミリ秒から」）、取得済みの終わりが 11:00:00.000 なのか 11:00:59.999 なのか読み取れない。また BR1.2 の「取得を始めた時刻」が、実行の開始か、範囲 1 回ごとの取得の開始か決まっていない（50 個を順に取ると差が出る。実行の開始の方が保守的で、キャッシュの書き込みは範囲ごと） | UC3 を具体的な数値で書き直す（終了は秒の 999 ミリ秒まで、FR3.5）。BR1.2 の基準時刻をどちらか 1 つに決め、キャッシュの書き込みとの違いがあれば書く | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| sensor-traceability | PASS（gaps・orphans・invalid なし） | 追跡表は整っている。意味の抜け（上記の取得の流れの口）は表では検出されない |
| sensor-upstream-coverage | PASS（upstream なしの扱い、findings 0） | 上流の網羅はこのツールでは確かめられていない。FR2.1〜FR2.8 とドメイン設計の R-01〜R-05 は手で突き合わせた |

### Summary

要件 FR2 の 6 つの受け入れ基準と、ADR-004 から ADR-008 に沿った筋は通っており、範囲の計算・staging の考え方・結果の写しは実装できる形です。ただし、既存の取得の流れとの接続（discard の約束と staging から committed への移し方）と、R-01 の入れ子の上限の数え方（左に偏った or と and の列）の 2 点は、実装前に決めるか直すことを勧めます。Major は 2 件で READY の範囲内ですが、承認の前に重みを見てください。
