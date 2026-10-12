# Unit of Work Story Map — ロググループ全体の取得の高速化

上流：`inception/user-stories/stories.md`（US1.1〜US4.2）、`unit-of-work.md`。

## ストーリーと作業単位

| Story | 内容 | Unit ID | Directory | 関わる要件 |
|-------|------|---------|-----------|------------|
| US1.1 | ストリームの多いロググループを速く取得する | U1 | u1-concurrent-skeleton | FR1.1、FR1.4、FR2.1、FR2.2、NFR1 |
| US1.2 | 届いた分から時刻順に見え始める | U1 | u1-concurrent-skeleton | FR1.2、FR1.3、NFR5 |
| US1.3 | 一部のストリームが失敗しても、取れた分を見る | U1 | u1-concurrent-skeleton | FR1.5 |
| US2.1 | 取得にかかった時間を見る | U4 | u4-elapsed-time | FR4.1〜FR4.6 |
| US2.2 | 同じアカウントの他の人を待たせない | U2 | u2-call-pacing | FR2.3〜FR2.6、NFR3 |
| US2.3 | 制限を受け続けたら、無駄に待たずにあきらめる | U3 | u3-give-up-rule | FR3.1〜FR3.4 |
| US3.1 | 取得を途中で止める | U5 | u5-cancel | FR5.1、FR5.2、FR5.9、FR5.10、NFR4、NFR10 |
| US4.1 | 取れた分を見て、足りない分を知る | U5 | u5-cancel | FR5.3〜FR5.7、FR5.11 |
| US4.2 | 中断した取得はキャッシュに残らない | U5 | u5-cancel | FR5.8 |

## 複数の作業単位にまたがるもの

| 事項 | 作業単位 | 中身 |
|------|----------|------|
| 呼び出しの速さの上限（US2.2） | U1、U2 | U1 で固定の上限（リージョンの表、半分程度、SDK の再試行を止める）、U2 で引き下げと戻し・再試行の前の許可（Q1） |
| 文言（NFR9） | U4、U5 | 新しい文言を英語と日本語で足す |
| 秘密を出さない（NFR8） | U1、U5 | 失敗・中断の表示は今の許可リストを通す |
| テストの決まり（NFR6、NFR7） | すべて | 純粋なロジックはテスト先行、実際の AWS に接続しない、カバレッジ 80% |
| 下準備（FR6） | 作業単位の外 | U1 の前に小さな PR で入れる |

## 作業単位の中での順番

- U1：FetchStatus の切り出し → StreamSchedule（テスト先行）→ CallPacer の固定の上限（テスト先行）→ FetchCoordinator の同時の読み取りと集約 → SDK の再試行を止める → FakeGateway・RecordingSink・FetchCheck の追従 → 画面で確かめる（US1.1 → US1.2 → US1.3）。
- U2：引き下げと戻しの純粋なロジック（テスト先行）→ Retrier の試行の手順への組み込み（US2.2）。
- U3：StreamSchedule のあきらめの決まり（テスト先行）→ RetryPolicy からの移動（US2.3）。
- U4：表記の関数（テスト先行）→ AppSession の記録 → 画面の時計と状態行（US2.1）。
- U5：止める信号の理由と Cancelled → N の数え方（テスト先行）→ 中断のコマンド → ボタン・注意 → キャッシュ（US3.1 → US4.1 → US4.2）。

## 抜けの確認

- ストーリー 9 本すべてに作業単位がある。
- 作業単位 5 つすべてに 1 本以上のストーリーがある（U1：3 本、U2：1 本、U3：1 本、U4：1 本、U5：3 本）。

## Assumptions & Open Questions

None.
