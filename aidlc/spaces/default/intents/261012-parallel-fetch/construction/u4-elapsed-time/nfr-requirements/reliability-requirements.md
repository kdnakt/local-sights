# Reliability Requirements — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/functional-design/`（functional-spec.md・rules.md BR4.1〜BR4.10、レビュー 1 回目の R-01〜R-06）、`inception/requirements-analysis/requirements.md`（FR4.1〜FR4.6、NFR1〜NFR10）、`inception/contract-design/contract-summary.md`（C4、C5）、`codekb/local-sights/technology-stack.md`。質問票の回答は `nfr-requirements-questions.md` の Q1 とまとめの確認。

| ID | 要件 | 目標・測り方 | 出典 |
|----|------|--------------|------|
| NFR6.5 | 取得中の表示は 1 秒ごとに進み、再試行の待ちなどライブラリから何も届かない間も止まらない | 画面のテスト（時計を差し替える）で、running の elapsed を 1 回だけ受け取り 16 秒進めると 16 秒分進む | FR4.2、AC2.1.2、BR4.6 |
| NFR6.6 | 取得中の表示は減らない。この保持は同じ取得の中の合わせ直し（running の表示中に running を受け取ったとき）だけに当てはめ、合わせ直しの値が今の表示より小さいときは今の表示を保つ。次の場合は保持を解いて受け取った値に置き換える：新しい取得の始まり（表示が最終の値か空のときに running を受け取った）、running false を受け取った（終わり。ここだけは 1 秒戻ることがある）、elapsed が空になった | 画面のテスト：表示 00:09 のときに elapsedMs 8,900 の running を受け取っても 00:09 のまま、次の 1 秒で 00:10。running false の 8,950 を受け取ると 00:08。最終の値 00:31 が出ている状態で新しい取得の running の 0 を受け取ると 00:00 から進む。空を受け取ると何も出ない | Q1、機能設計 R-06 |
| NFR6.7 | 取得中の表示とライブラリの値とのずれは 1 秒以内。ただし PC のスリープをまたいだ取得は対象外とする（ライブラリと画面の戻らない時計がスリープ中に同じように進むとは限らないため）。その場合も終わったときはライブラリの最終の値を出す | 画面のテストで、合わせ直しの直後の表示が受け取った値より 1 秒を超えて大きくならない（NFR6.6 で保った値が、次の合わせ直しまでに追いつく）。スリープをまたいだ場合の見え方は GUI の確認で一度見ておく | ADR-005、機能設計 R-06 |
| NFR6.8 | 時計は戻らない時計を使い、PC の時刻を変えても経過は負にも大きく飛びもしない。差が負になる計算は 0 にする | ライブラリと画面のテストで、時計を差し替えて負の差を与えると 00:00 | BR4.10 |
| NFR6.9 | 画面の時計の合わせ直しは、バックエンドから完全な SessionView が届いたとき（`session-changed`、起動時の `getSession`、操作の応答として返る SessionView）に行う。`fetch-progress`・`filter-progress` を画面の中で今の session に合成したものでは合わせ直さない | 画面のテストで、fetch-progress を何度合成しても表示が戻らないこと、取得中に `getSession` の結果として running の elapsed を受け取ると時計が始まること | 機能設計 R-01 |
| NFR6.10 | 経過が空になるのは「一度も取得していない」「閉じて止めた（Aborted）」「接続の変更で前の結果が消えた」の 3 つだけ。最後のものは機能設計の Q2 で利用者が選んだ扱いで、FR4.3・AC2.1.3 の「次の取得まで残す」の例外とする | ライブラリのテストで 3 つの場合と、ロググループを選び直しても残る場合を確かめる | BR4.3、BR4.4、R-05 |
| NFR6.11 | 取得の作業が異常に止まったときも、失敗として終える時点で最終の値を決め、経過が進み続けたままにならない | ライブラリのテスト（abort_fetch_with_failure の後は running false） | BR4.2 |

## Assumptions & Open Questions

None.
