# Performance Test Instructions — リリースの .app をアドホック署名する

## 対象

この変更には性能の要件がない。`requirements.md` の NFR1〜NFR3 に性能の項目はなく、`nfr-requirements/`・`nfr-design/` もこのワークフロー（スコープ express）にはない。テスト戦略は Minimal で、性能テストの手順は作らない。そのため性能テストは行わない。

## 補足

署名検証ステップは `codesign` を 2 回呼ぶだけで、Release ワークフローの所要時間への影響は数秒にとどまる。ワークフローの `timeout-minutes: 40` は変えていない。
