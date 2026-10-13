## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | construction/u3-give-up-rule/nfr-requirements/nfr-requirements-questions.md の前提（R-01・R-03・R-06）と functional-design/functional-spec.md の 2. 例・5. 持ち越しの扱い、functional-design/traceability.json | 機能設計レビューの R-01（例の数値）・R-03（あきらめた後の数え）・R-06（対応表）は、今回は質問票の前提に「コード生成の計画とテストで直す」と書かれただけで、正本の側は直っていない。functional-spec.md の例は今も未着手 s10〜s20・読み途中 s09 のままで、実際の枠の埋まり方（s14 以降が未着手、s09〜s13 が読み途中）と食い違う。5. には「数え方の結果は同じ」が残り、あきらめた後に数えを動かさない変更（現行の schedule.rs は後からも record を呼ぶ）と矛盾する。functional-design の traceability.json にも FR3.1〜FR3.4 がない。後続のステージが正本を読むと誤った期待値を台本にしてしまい、正しい内容が質問票にしか無い二重の状態になる。 | security-requirements.md か tech-stack-decisions.md に、正しい例の数値とあきらめた後に数えを固定する変更を、U1 からの差分として規範の文章で書く。あるいは機能設計の正本を直す手順をコード生成の入力として明記する。 | New |
| R-02 | Minor | construction/u3-give-up-rule/nfr-requirements/nfr-requirements-questions.md の R-05、tech-stack-decisions.md の数えの決まりの行 | R-05 は半分しか決まっていない。StreamEnding と ExhaustionStreak を retry.rs から、streak_ending を fetcher.rs から schedule.rs へ移すことは決まったが、現行の schedule.rs にある SlotEnd（Completed・FailedExhausted・FailedNotExhausted・Aborted・Skipped）を残すかどうかが書かれていない。残すと StreamEnding との分類が二重になる。また functional-spec.md の疑似コードは finish に StreamFetchEnd を渡すが、現行の finish は order・SlotEnd・失敗の内容を取り、StreamFetchEnd から SlotEnd への変換は coordinator.rs（493〜498 行）にある。分類を StreamSchedule だけが行うなら、schedule.rs が fetcher.rs の StreamFetchEnd を使う依存になるが、その向きは ADR-007 の StreamSchedule から EventFetcher への片方向と合っており、その旨も書かれていない。 | 終わり方の型の整理を 1 行で決める（SlotEnd を残すか StreamEnding に一本化するか、finish が受ける型、coordinator.rs の変換の行き先）。依存の向きが片方向であることも併記する。 | New |
| R-03 | Minor | construction/u3-give-up-rule/nfr-requirements/security-requirements.md の要件表（NFR7.1・NFR7.2）、traceability.json の coverage | テストで確かめる内容の一部が、規範の要件ではなく質問票の前提にしか書かれていない。BR3.5（閉じて止めたものを数えない）、BR3.6（RetryPolicy に max_consecutive_exhausted が無く EventFetcher が StreamEnding を使わない）、limit の下限 1 への切り上げ、あきらめの記録を ScheduleOutcome に載せる観測の口がそれにあたる。NFR8.9 の確かめ方（写した失敗が元と同じ値）も NFR3.7 の呼び出し 0 回の確認も、観測の口に依存している。traceability.json は FR3.1・FR3.3・FR3.4 をどれも NFR7.2（行カバレッジ 80% の閾値）にだけ対応づけており、振る舞いの検証への対応になっていない。80% の達成は、これらのテストが書かれたことを保証しない。 | 要件表に行を足す（例：NFR7.3）。BR3.3・BR3.5・BR3.6、limit の切り上げ、観測の口を、それぞれ確かめ方つきで書く。traceability.json の対応先をその行に直し、AC2.3.1〜AC2.3.4 と BR3.5・BR3.6 も載せる。 | New |
| R-04 | Minor | construction/u3-give-up-rule/nfr-requirements/nfr-requirements-questions.md の R-04、security-requirements.md の NFR3.7・NFR8.9 | NFR3.7 は「あきらめた後は未着手を呼ばない」と無条件に書くが、前提には、使い切りのストリームの失敗の内容が無い場合は Skipped にしない今の動き（schedule.rs の let (true, Some(failure)) の分岐）を残すとある。その場合は使い切りが limit に達しても未着手が呼ばれ続けるので、NFR3.7 の保証に穴が残る。「今は起きない」という根拠も、失敗の内容が必ず付くことを型で保証するものではない。そのとき数えを固定するか、あきらめた扱いにするかも決まっていない。 | 使い切り（Failed と exhausted: true）には失敗の内容が必ず付くことを前提として NFR3.7 に明記する。または分岐が起きたときの扱い（あきらめとして扱い、写す内容が無くても未着手を Skipped にする、など）を決めて書く。 | New |

### Summary

U3 は新しい画面・API・依存・保存先を持たないライブラリの整理であり、NFR3.7・NFR8.1・NFR8.9・NFR6.1・NFR7.1・NFR7.2 は上流の FR3・NFR3・NFR6〜NFR8、ADR-007、現行コードと矛盾しなかった。具体的には、CI の `cargo llvm-cov -p local-sights-core --fail-under-lines 80` が ci.yml と一致する。max_consecutive_exhausted を使う箇所が coordinator.rs と retry.rs のテストに限られ、確認用プログラムや Tauri アプリは RetryPolicy::default() を使うだけで変更が要らない。未着手を呼ばないことを確かめる既存の結合テストが tests/u3_fetch_flow.rs にあり、失敗の内容の許可リストのテストが failure.rs にある。循環依存や参照切れも見つからなかった。指摘はいずれも Minor で、機能設計レビューの軽微な指摘が正本に反映されず質問票の前提に留まっていること（R-01）、終わり方の型の整理が途中であること（R-02）、振る舞いの検証が規範の要件と対応表に載っていないこと（R-03）、失敗の内容が無い場合の穴（R-04）である。Critical と Major がないため READY とする。
