# Security Requirements — U3 あきらめの決まり（u3-give-up-rule）

上流：`construction/u3-give-up-rule/functional-design/`（rules.md BR3.1〜BR3.7、functional-spec.md、レビュー 1 回目の R-01〜R-06）、`inception/requirements-analysis/requirements.md`（FR3.1〜FR3.4、NFR3・NFR6・NFR7・NFR8）、`inception/contract-design/contract-summary.md`（C2）、`codekb/local-sights/`、U1・U2 の `nfr-requirements/`。まとめの確認は `nfr-requirements-questions.md`。

U3 はライブラリの中の StreamSchedule の数えの決まりと持ち主を整えるだけで、新しい画面・コマンド・権限・API・保存先・依存を持たない。

## 要件

| ID | 要件 | 確かめ方 | 出典 |
|----|------|----------|------|
| NFR3.7 | あきらめた後は、未着手のストリームで GetLogEvents を呼ばない。制限が続くときに同じアカウントの他の利用者の枠を使い続けない | 偽物のゲートウェイで、あきらめの後に未着手のストリームの呼び出しが 0 回であることを確かめる結合テスト | FR3.2、NFR3 |
| NFR8.1 | 呼ぶ API は CloudWatch Logs の読み取り 3 つだけ。U3 で新しい API を呼ばない | CloudWatchLogsGateway の trait を変えない | project.md Forbidden |
| NFR8.9 | Skipped に写す失敗の内容は、3 本目の使い切りの失敗の安全な詳細（ApiFailure の許可リストを通したもの）だけ。秘密の認証情報・アクセスキー ID を含まない | 写した失敗が元の ApiFailure と同じ値であることを確かめる単体テスト（許可リストは今のテストが確かめる） | FR3.2、project.md Forbidden、NFR8 |
| NFR6.1 | 自動テストと CI は実際の AWS に接続せず、CI に AWS の認証情報を置かない | 新しいテストは StreamSchedule の単体テストと FakeGateway | project.md Forbidden |
| NFR7.1 | 既存のテストは通ったまま。ただし RetryPolicy の数えの項目（max_consecutive_exhausted）・ExhaustionStreak・StreamEnding を使うテストは、StreamSchedule に移した形に同じ変更で直す（確かめる中身は弱めない） | `cargo test -p local-sights-core` | NFR7、ADR-007 |
| NFR7.2 | ライブラリの行カバレッジ 80% 以上を CI で確かめる。StreamSchedule の数えの決まりは純粋な部品としてテストを先に書き、終わる順を入れ替えた場合（AC2.3.1〜AC2.3.4）と、あきらめた後に数えを動かさないことを確かめる | CI の `cargo llvm-cov -p local-sights-core --fail-under-lines 80` | team.md Testing Posture、NFR7、FR3.1〜FR3.4 |

## 脅威と対策

| 脅威 | 対策 |
|------|------|
| 制限が続いているのに残りのストリームを呼び続け、同じアカウントの他の利用者を待たせる | 3 本の使い切りで未着手を呼ばずに失敗にする（NFR3.7） |
| Skipped の失敗の詳細に秘密が混ざる | 今の安全な詳細をそのまま写す（NFR8.9） |
| 数えの持ち主を移すときに、終わり方の分け方が 2 か所に残って食い違う | 分けるのは StreamSchedule だけにし、retry.rs と fetcher.rs の分け方をなくす（ADR-007） |

## Assumptions & Open Questions

None.
