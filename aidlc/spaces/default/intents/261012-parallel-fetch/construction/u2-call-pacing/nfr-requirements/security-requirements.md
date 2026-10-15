# Security Requirements — U2 呼び出しの速さ（u2-call-pacing）

上流：`construction/u2-call-pacing/functional-design/`（functional-spec.md・rules.md BR2.1〜BR2.8）、`inception/requirements-analysis/requirements.md`（FR2.4〜FR2.6、NFR3・NFR6・NFR7・NFR8）、`inception/contract-design/contract-summary.md`（C1）、`codekb/local-sights/`、U1 の `nfr-requirements/security-requirements.md`。まとめの確認は `nfr-requirements-questions.md`。

U2 はライブラリの中の CallPacer に下げ方と戻し方を足すだけで、新しい画面・コマンド・権限（Tauri の capabilities）・API・保存先・依存を持たない。守るものは U1 と同じく、利用者の AWS の認証情報と、同じ AWS アカウントの他の利用者の呼び出しの枠である。

## 要件

| ID | 要件 | 確かめ方 | 出典 |
|----|------|----------|------|
| NFR3.5 | 制限を受けたら呼び出しを減らし、同じアカウントの他の利用者の枠を空ける：制限の知らせで今の速さを半分にし（下限 1 秒 1 回）、下げている間の許可の間隔は今の速さの間隔以上 | 止めた時計で CallPacer の速さと許可の時刻を確かめる単体テスト、偽物のゲートウェイで制限が続くと間隔が広がる結合テスト | FR2.4、BR2.2、BR2.3、BR2.5 |
| NFR3.6 | 戻しても上限を超えない：今の速さはリージョンの上限（U1 の BR1.1・BR1.2）を超えず、どの 1 秒の区間でも GetLogEvents の許可は上限以下のまま（U1 の NFR3.1 を保つ） | 戻しの単体テストで上限に止まること、結合テストで制限が止んだ後も 1 秒の上限を超えないこと | FR2.1、BR2.4、BR2.5 |
| NFR8.1 | 呼ぶ API は CloudWatch Logs の読み取り 3 つだけ。U2 で新しい API を呼ばない | CloudWatchLogsGateway の trait を変えない。コードの差分の確認 | project.md Forbidden |
| NFR8.7 | 要約の 1 行に足す `min_rate=` は整数だけ。秘密の認証情報・アクセスキー ID・接続の情報（プロファイル名・リージョンなど）を含まない | 要約を組み立てる関数の単体テストで、入るのが数値と決まった語だけであることを確かめる（U1 の NFR8.3・NFR8.6 のテストを U2 の書式に直す） | BR2.7、project.md Forbidden |
| NFR8.8 | 引き下げは画面にも外にも知らせない。出力先は今と同じ標準エラー出力の要約の 1 行だけ（テレメトリを送らない） | 画面の型（JobStatus・FetchSink の知らせ）を変えないことの確認 | FR2.6、project.md Forbidden |
| NFR6.1 | 自動テストと CI は実際の AWS に接続せず、CI に AWS の認証情報を置かない | 新しいテストはすべて止めた時計と FakeGateway を使う | project.md Forbidden |
| NFR7.1 | 既存のテストは通ったまま。ただし U1 の要約の書式のテスト（欄の順）は、`min_rate=` が増えるので U2 の書式に直す | `cargo test -p local-sights-core` | NFR7、BR2.7 |
| NFR7.2 | ライブラリの行カバレッジ 80% 以上を CI で確かめる。CallPacer の下げ方・戻し方は純粋な部品としてテストを先に書く | CI の `cargo llvm-cov -p local-sights-core --fail-under-lines 80` | team.md Testing Posture、NFR7 |

## 脅威と対策

| 脅威 | 対策 |
|------|------|
| 制限を受けても同じ速さで呼び続け、同じアカウントの他の利用者を待たせる | 制限の知らせで半分に下げる（NFR3.5） |
| 戻しすぎて上限を超え、また制限を受ける | 上限で止める・少しずつ戻す（NFR3.6、BR2.4） |
| 8 本の制限がまとめて届いて一気に下限まで落ち、取得が不要に遅くなる | 下げてよい時刻までは重ねて下げない（BR2.3） |
| 要約の 1 行に秘密や接続の情報が混ざる | 足す欄は整数だけ（NFR8.7） |

## Assumptions & Open Questions

None.
