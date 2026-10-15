# Tech Stack Decisions — U5 中断（u5-cancel）

上流：`construction/u5-cancel/functional-design/`（functional-spec.md・rules.md BR5.1〜BR5.14、レビュー 2 回目の新しい指摘 2 件）、`inception/requirements-analysis/requirements.md`（FR5.1〜FR5.11、NFR1〜NFR10）、`inception/contract-design/contract-summary.md`（C3〜C7）、`codekb/local-sights/technology-stack.md`。質問票は `nfr-requirements-questions.md`（まとめの確認）。

新しい言語・フレームワーク・依存は足さない（NFR8.14）。

| 対象 | 選んだもの | 理由 | 採らなかったもの |
|------|-----------|------|------------------|
| 止める信号 | U1 で用意した理由つきの StopHandle／StopSignal（retry.rs）に Cancel を送る | 止め方を 1 か所に保つ（ADR-004） | 中断用の別の信号 |
| 出ている呼び出しの打ち切り | 中断を受けたら、読み途中の呼び出しの待ち（FuturesUnordered に積んだもの）と一覧の呼び出しの待ちを、止める信号と並べて待ち、信号が来たら待ちを捨てる（Rust の future を落とすと HTTP の要求も打ち切られる） | 応答が返らなくても止まる（NFR4.3、追加の質問 Q1） | 呼び出しごとの時間切れ：中断以外の動きも変わる |
| 中断のコマンド | Tauri のコマンド `cancel_fetch` と権限の追加 | 今のコマンドと同じ作り（C6） | イベントで送る |
| ボタンと注意 | Cloudscape の Button（loading）と Alert（warning） | 標準部品だけを使う（project.md Corrections） | 自作の部品 |
| テスト | Rust の標準のテスト・tokio の止めた時計・FakeGateway、Vitest と Testing Library | 今の道具で足りる | 新しいテストの道具 |

| ID | 要件 | 確かめ方 | 出典 |
|----|------|----------|------|
| NFR7.4 | 行カバレッジはライブラリ・画面ともに 80% 以上を保つ | CI | NFR7 |
| NFR6.20 | 純粋な判断（N の数え方、終わりの状態の決め方、押せるかの判定）はテストを先に書く。取得の流れのつなぎ込み・Tauri のコマンド・画面は実装してからテストを書く | コード生成の計画とテストの順 | NFR6、team.md Testing Posture |

## Assumptions & Open Questions

None.
