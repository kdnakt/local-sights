# Reliability Requirements — U5 中断（u5-cancel）

上流：`construction/u5-cancel/functional-design/`（functional-spec.md・rules.md BR5.1〜BR5.14、レビュー 2 回目の新しい指摘 2 件）、`inception/requirements-analysis/requirements.md`（FR5.1〜FR5.11、NFR1〜NFR10）、`inception/contract-design/contract-summary.md`（C3〜C7）、`codekb/local-sights/technology-stack.md`。質問票は `nfr-requirements-questions.md`（まとめの確認）。

| ID | 要件 | 目標・測り方 | 出典 |
|----|------|--------------|------|
| NFR6.15 | 閉じるは常に中断より強い。中断の後に閉じても、閉じるときの今の動き（確認、取れたログを捨てて終わる）は変わらない | 今の閉じるときの回帰テストが変えずに通る。中断の後に閉じるで Aborted になる結合テスト | FR5.10、ADR-004 |
| NFR6.16 | どの終わり方（Completed・CompletedWithFailures・Failed・Aborted・Cancelled・異常な停止）でも［中断中…］が戻る | session.rs のテストで終わり方ごとに cancelling が false | BR5.8 |
| NFR6.17 | 取得を始める操作は、止める信号の持ち手を session のロックを持ったまま AppState に登録する（今の begin_and_spawn_fetch はロックを手放してから登録しているので、順を入れ替える）。取得を始める操作も `cancel_fetch` も、session のロック → 持ち手のロックの順で取り、逆の順では取らない。持ち手が無いうちに中断中だけが立つことはない | 登録と状態の更新を 1 つの関数にまとめ、その単体テストで、始める操作の直後に中断が届いても新しい持ち手に Cancel が届き中断中が立つこと、持ち手が無いときは何もしないことを確かめる | 機能設計レビュー 2 回目の指摘 2、BR5.1、レビュー R-02 |
| NFR6.18 | すべてのストリームを読み終えた後・取得の終わりの前に届いた中断は終わった扱い（FR5.9、NFR4.5）。ログは消えず、キャッシュの決まりも今のまま | 結合テストで、Completed で終わりキャッシュに書かれること | FR5.9、BR5.5 |
| NFR6.19 | キャッシュを引いている間の中断は、当たれば Completed、当たらなければ一覧の前の中断になる。どちらでも取得は必ず終わる | 結合テスト（キャッシュあり、当たり・外れ） | BR5.5 |

## Assumptions & Open Questions

None.
