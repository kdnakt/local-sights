# Security Requirements — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/functional-design/`（functional-spec.md・rules.md）、`inception/requirements-analysis/requirements.md`（NFR1〜NFR10）、`inception/contract-design/contract-summary.md`、`codekb/local-sights/`。質問票の回答は `nfr-requirements-questions.md` の Q1・Q2。

U1 は外から呼ばれる口を持たないローカルのデスクトップアプリの中の変更で、新しい画面・コマンド・権限（Tauri の capabilities）を足さない。守るものは、利用者の AWS の認証情報と、同じ AWS アカウントの他の利用者の呼び出しの枠である。

## 要件

| ID | 要件 | 確かめ方 | 出典 |
|----|------|----------|------|
| NFR8.1 | 呼ぶ API は CloudWatch Logs の読み取り 3 つ（DescribeLogGroups・DescribeLogStreams・GetLogEvents）だけ。U1 で新しい API を呼ばない | CloudWatchLogsGateway の trait は 3 操作だけ（形を変えない）。コードの差分の確認 | project.md Forbidden |
| NFR8.2 | 秘密の認証情報（シークレットアクセスキー・セッショントークン・SSO のトークン）とアクセスキー ID を、ログ・画面・キャッシュ・リポジトリに書かない | 失敗の詳細は今の許可リスト（ApiFailure の安全な詳細）を通す。新しい診断ログの 1 行（NFR8.3）は数と時間だけ | project.md Forbidden、NFR8 |
| NFR8.3 | 取得 1 回の終わりの診断ログ（標準エラー出力の 1 行）に出すのは、読んだストリームの本数、GetLogEvents の呼び出しの数（再試行を含む）、制限を受けた回数、かかった時間、同時に読んだ本数の最大、終わりの状態だけ。ストリーム名・ロググループ名・プロファイル名・ロール・アカウント ID・リージョンなどの接続の情報・失敗の詳細は出さない | 診断ログの文字列を組み立てる関数の単体テストで、入るのが数値と決まった語だけであることを確かめる | Q1、NFR8 |
| NFR8.4 | 診断ログはファイルに書かず、外にも送らない（テレメトリ・クラッシュレポートを送らない） | 出力先は標準エラー出力だけ（今と同じ） | project.md Forbidden、前回の NFR16 |
| NFR6.1 | 自動テストと CI は実際の AWS に接続せず、CI に AWS の認証情報を置かない | 新しいテストはすべて FakeGateway を使う。SDK の再試行の設定の単体テストも、クライアントの設定を組み立てて確かめるだけで接続しない | project.md Forbidden、NFR6 |
| NFR3.1 | 同じアカウントの他の利用者の枠を奪わない：GetLogEvents は再試行を含めて、どの 1 秒の区間でもリージョンの上限の半分以下 | 偽物で呼んだ時刻を記録するテスト（performance・scalability 側） | FR2.1、NFR3 |
| NFR8.5 | 新しい依存（futures-util）は、cargo-deny の検査（脆弱性・ライセンス・出どころ）を通るものだけ。既定の機能は切り、要るものだけ有効にする | CI の cargo-deny が通る | team.md Code Style |

## 脅威と対策

| 脅威 | 対策 |
|------|------|
| 同時に読むことで呼び出しが増え、同じアカウントの他の利用者が制限を受ける | 速さの上限（リージョンの上限の半分、NFR3.1）、SDK の再試行を止めて数を合わせる（BR1.6） |
| 診断ログに秘密や利用者の情報が混ざる | 出す項目を数値と終わりの状態に限る（NFR8.3）、失敗の詳細は許可リストを通す（NFR8.2） |
| 渡し待ちのページがたまり続けてメモリを使い切る | 渡し待ちは最大 8 ページ（NFR5.2） |
| 新しい依存に脆弱性がある | cargo-deny と Dependabot（今の仕組み）、NFR8.5 |

## Assumptions & Open Questions

None.
