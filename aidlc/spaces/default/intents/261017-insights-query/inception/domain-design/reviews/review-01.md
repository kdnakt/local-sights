## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-17T13:12:09Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261017-insights-query/inception/domain-design/components.md > LogStore（behaviour）、FetchCoordinator（behaviour）、ADR-001 | 今のコードは、ログ管理の画面の [取得] のたびに保持中を捨てて AWS から取り直す（`coordinator.rs` の `serve_from_cache` / `run_fetch` は先頭で timeline を discard する）。新しい設計では、取得済みの範囲に入る期間は AWS を呼ばない（FR2.3）ため、終了が「いま」に近い期間で [取得] を押し直しても、あとから届いたログは二度と入らない。ディスクキャッシュは「取得開始の 5 分前まで」しか記録しない決まりでこの問題を避けているが、LogStore の取得済みの範囲にはその余白がない。「見た目の振る舞いは今と同じ」（FR1.8、ADR-001）とも食い違う。 | LogStore の取得済みの範囲に「新しさ」の決まりを設計で定める。例：取得開始の 5 分前より後は取得済みに加えない、または [取得] は明示的な取り直し（その範囲を捨てて取り直す）として扱う。どちらにするかを ADR に書き、OQ4（取り直したときの範囲の扱い）の答えとして閉じる。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261017-insights-query/inception/domain-design/components.md > LogCache（behaviour）、FetchCoordinator（responsibilities） | 今のキャッシュの書き込みは、1 回の取得が終わったあとに、取得した期間全体を timeline から写して 1 つの記録範囲として書く（`run_fetch_with_cache` の `decide_write` と `copy_held_events`）。新しい設計は取得を「足りない範囲だけ」にするが、キャッシュに何を書くか（足した範囲だけか、LogStore に溜まった全体か）と、誰が書くか（FetchCoordinator の責任に書き込みがない）が決まっていない。「書き込みの決まりは変えない」とだけあり、足りない範囲が複数に分かれたときの記録範囲の作り方も不明。 | FetchCoordinator が LogCache に書く責任を持つことと、書く単位（足りない範囲ごとに、全ストリームが成功したものだけ）を LogCache / FetchCoordinator の記述に足す。複数の足りない範囲があるときに、記録範囲が取得済みの範囲と矛盾しないことも書く。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261017-insights-query/inception/domain-design/components.md > QueryRunner（behaviour）、LogStore（behaviour）、AppSession（behaviour） | 取得は同時に 1 つに保つ（Q4）が、評価（QueryEvaluator が決まった件数ずつ読む）は取得ではないため、評価の途中にログ管理の画面の [取得] が LogStore にページを足せる。LogStore は版を持つが、評価がどの版を読むか（途中で版が上がったらやり直すか、その版の範囲だけで通すか）は書かれていない。 | 評価の実行中は LogStore を読む版を固定する、または版が変わったら結果を無効にする、のどちらかを QueryRunner か LogStore の記述に一行で足す。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261017-insights-query/inception/domain-design/components.md > QueryRunner（depends_on: EventFieldReader） | QueryRunner が EventFieldReader に依存する理由は「補完の候補の項目名」だが、補完の候補は実行の流れ（読み取り → 取得 → 評価）とは別の入口（FR9.1）で、実行の流れの部品に載せると責任が混ざる。TauriApp の補完のコマンドがどの部品を呼ぶかも書かれていない。 | 補完の候補の入口を持つ部品を決める（例：LogStore と EventFieldReader を直接つなぐ入口を TauriApp から呼ぶ、または小さな部品に分ける）。または、QueryRunner の責任に補完を明示して現状を維持する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| components.md の yaml の整合（python3 + yaml） | PASS：23 部品の名前は一意。宣言されていない参照なし。depends_on と dependents は対称。循環なし。すべてのエンティティに識別子があり、references の owned_by は宣言済み | 構造上の問題なし |
| traceability.json の整合 | PASS：FR1.1〜FR9.3 の 49 件のうち FR8.1・FR8.2 は理由つきの N/A、残りは OK で、target はすべて components.md の部品名として存在する | 上流の取りこぼしなし |

### Summary

構造（名前・依存・循環・エンティティの持ち主・トレーサビリティ）は整っており、部品の分け方も実装できる。承認前に確かめたいのは、LogStore の取得済みの範囲に「新しさ」の決まりがなく、[取得] の押し直しで最新のログが入らなくなりうること、と、取り足しになったあとのキャッシュの書き込みの持ち主と単位が決まっていないことの 2 点（どちらも機能設計で閉じられるが、ADR か記述に書いておくと手戻りが減る）。
