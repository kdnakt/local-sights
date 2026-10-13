# Security Design — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/nfr-requirements/security-requirements.md`（NFR8.1〜NFR8.5、NFR6.1、NFR3.1）、`construction/u1-concurrent-skeleton/functional-design/`、`project.md` Forbidden。質問票の回答は `nfr-design-questions.md` の Q1。

U1 は新しい画面・Tauri のコマンド・権限（capabilities）を足さない。認証は今と同じく AWS SDK の仕組みに任せ、アプリは秘密の認証情報を読まない・持たない。

## 設計

| 要件 | 仕組み |
|------|--------|
| NFR8.1（読み取り 3 つだけ） | CloudWatchLogsGateway の trait の形を変えない。新しい呼び出しは足さない。SDK の再試行の上書きは GetLogEvents の 1 つの呼び出しの設定だけで、呼ぶ API は増えない |
| NFR8.2（秘密を出さない） | 失敗の詳細は今の ApiFailure の許可リストを通す（変えない）。あきらめで写す失敗も同じ値を写すだけ |
| NFR8.3・NFR8.6（診断ログの中身） | 要約の 1 行は、数値の項目（本数、呼び出しの数、制限の回数、時間ミリ秒、同時の最大）と終わりの状態（JobStatus の決まった語）だけで組み立てる。組み立ての関数は数値と列挙の値しか受け取らない形にして、文字列（名前・失敗の詳細）を入れられないようにする。単体テストで、出力が決まった書式（`local-sights: fetch summary streams=… calls=… throttled=… elapsed_ms=… max_in_flight=… status=…`）に合うことを確かめる |
| NFR8.4（外に送らない） | 出力先は標準エラー出力だけ。ファイル・ネットワークには書かない |
| NFR6.1（AWS に接続しない） | 新しいテストはすべて FakeGateway と tokio の止めた時計を使う。SDK の設定の確かめは、GetLogEvents の呼び出しに付ける設定の上書きを作る関数を分けて、その値（再試行の回数 1）を単体テストで確かめる。HTTP は使わない |
| NFR3.1（他の利用者の枠を奪わない） | performance-design・scalability-design の速さの上限 |
| NFR8.5（新しい依存） | `futures-util` は `default-features = false`、`features = ["std"]`。CI の cargo-deny で確かめる |

## 脅威への対策の割り当て

| 脅威 | 対策の場所 |
|------|------------|
| 呼び出しが増えて同じアカウントの他の利用者が制限を受ける | CallPacer（速さの上限）、AwsCloudWatchLogsGateway（SDK の再試行の上書き） |
| 診断ログに秘密や名前が混ざる | 要約の組み立て関数の型（数値と列挙だけ） |
| 渡し待ちのページがたまってメモリを使い切る | FetchCoordinator の並べ方（最大 8 ページ） |
| 新しい依存の脆弱性 | cargo-deny、Dependabot |

## Assumptions & Open Questions

None.
