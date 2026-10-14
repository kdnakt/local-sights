# Reliability Design — U5 中断（u5-cancel）

上流：`construction/u5-cancel/nfr-requirements/`（NFR4.2〜NFR10.1、追加の質問 Q1）、`construction/u5-cancel/functional-design/`（BR5.1〜BR5.14。BR5.3・BR5.4 の応答待ちと 1.2・1.4 は非機能要件で置き換え済み）、`inception/contract-design/contract-summary.md`（C3〜C7）、今のコード。質問票は `nfr-design-questions.md`。

設計の正本は `security-design.md` の 2 節。

| 要件 | 設計 |
|------|------|
| NFR6.15 | StopReason の強さの決まり（Close が勝つ）は U1 のまま。閉じるは今の abort() の道を通る。閉じるときの確認（U7）は変えない。読みの打ち切りは理由を問わないので、閉じるときも今より早く終わる |
| NFR6.16 | finish_fetch のすべての分岐の最初で cancelling = false。abort_fetch_with_failure でも同じ |
| NFR6.17 | begin_and_spawn_fetch は session のロックの中で持ち手を登録する。cancel_fetch は session → abort の順でロックを取り、request_cancel が返す番号と持ち手の番号が同じときだけ送る。cancel_fetch は session のロックを持ったまま view を送るので、取得の終わりの view より先に届く |
| NFR6.18 | 読む段の後、Cancel でも N = 0 なら decide_job_status が今の決め方を返し、今の終わりの流れ（キャッシュの判定を含む）に進む |
| NFR6.19 | serve_from_cache は Close のときだけ abort()。キャッシュに外れたら run_fetch の一覧の段で止まる |

## 失敗の扱い

中断で新しく失敗の道は増えない。`unwrap`・`expect` は使わない。cancel_fetch はエラーを返さない。

## Assumptions & Open Questions

None.
