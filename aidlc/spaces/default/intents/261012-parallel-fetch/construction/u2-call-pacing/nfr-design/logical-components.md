# Logical Components — U2 呼び出しの速さ（u2-call-pacing）

上流：`construction/u2-call-pacing/nfr-requirements/`、`construction/u2-call-pacing/functional-design/`、`inception/domain-design/components.md`、U1 の `nfr-design/logical-components.md`。まとめの確認は `nfr-design-questions.md`。

U2 は新しい部品を足さない。U1 の部品に次の仕組みを足す。失敗の範囲は U1 と同じく「取得 1 回」。

| 部品 | U2 で入れる仕組み | 失敗の範囲 | 共有するもの |
|------|-------------------|------------|--------------|
| CallPacer | 今の速さ・下げてよい時刻・制限なしの数え始め・最も低い速さを U1 のロックの中に持つ。`report_throttled()`、`current_rate_per_second()`、`lowest_rate_per_second()`。`min_interval()` は今の速さの間隔を返す。冒頭の文書を直す（U1 のコード生成のレビュー R-01、ロックの説明） | 取得 1 回（取得ごとに作る） | 取得 1 回の全ストリーム |
| EventFetcher | `counted_call` で Throttled を数えるところで `report_throttled()` を呼ぶ | 1 本のストリーム | CallPacer |
| FetchCoordinator | 取得の終わりに最も低い速さを読んで FetchStats に渡す。CallPacer の前に終わった取得はリージョンの上限を渡す | 取得 1 回 | FetchStats |
| FetchStats | 要約の 1 行に `min_rate` の欄（max_in_flight の後・status の前） | 取得 1 回 | — |
| FakeGateway（テスト） | U1 のまま（台本で Throttled を返し、呼んだ時刻を記録する） | テストだけ | — |

## Assumptions & Open Questions

None.
