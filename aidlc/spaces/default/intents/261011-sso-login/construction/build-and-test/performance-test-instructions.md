# Performance Test Instructions — アプリから SSO ログインする

## 対象

この変更には性能の要件がない。`requirements.md` の NFR1〜NFR7 に性能の項目はなく、`nfr-requirements/`・`nfr-design/` もこのワークフロー（スコープ refactor）にはない。テスト戦略は Standard で、Standard では性能テストの手順を作らない。そのため性能テストは行わない。

## 時間に関わる振る舞い

時間に関わる振る舞いは、性能ではなく機能として確かめている。

| 振る舞い | 確かめ方 |
|---|---|
| 5 分の時間切れ（FR2.5） | `sso_cli.rs` のテストで、時間切れを短くした引数を使って TimedOut になり、プロセスが残らないことを確かめる |
| SIGTERM のあと 2 秒待って SIGKILL（BR3.3） | `sso_cli.rs` のキャンセルと子プロセスのテスト |

ログイン後の一覧の読み直しは、既存の一覧の読み込みをそのまま使うため、新しい性能の基準はない。
