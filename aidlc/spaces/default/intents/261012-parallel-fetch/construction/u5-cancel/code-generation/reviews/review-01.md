## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | src/components/CancelNotice.tsx > role="status" の囲み、README.md > Cancel の手元の確認の最後から 2 番目の項目 | role="status" の囲みは、注意の文を中に入れたまま画面に現れる。読み上げの領域は中身が変わる前から画面にあるときのほうが確実に読まれるので、「出たときに 1 回読まれる」は支援技術によっては読まれない。README の手元の確認はこの動きを前提にしている | 空の role="status" の囲みを常に置いて中身だけを入れ替えるか、手元の確認の文を「読まれない場合がある」前提に改める。VoiceOver での確認結果を記録する | New |
| R-02 | Minor | src/i18n/messages.ts > cancel.notice.afterListing（英語） | 英語の文が "{n} streams were not finished" と複数形に固定されていて、N が 1 のとき "1 streams" と出る。N が 1 になるのは、1 本だけ読んでいる最中に中断した場合などで、普通に起きる | n が 1 のときの文を別に持つか、単複どちらでも通る言い方（例："Streams not finished: {n}"）に変える。英語のテストに n が 1 の例を足す | New |
| R-03 | Minor | src-tauri/src/lib.rs > begin_and_spawn_fetch の始まりの view の送信、tests > the_manifest_the_capability_and_the_handler_list_the_same_commands | (1) 始まりの view は session のロックを放してから送るため、その直後に cancel_fetch が通ると、［中断中…］の view の後に古い「押せる」view が届く。非機能設計 R-08 の途中の更新と同じ種類で、取得の終わりの view（取得のタスクは始まりの view を送った後に作られる）で直るが、code-summary.md には途中の更新しか書いていない。(2) 権限の 3 か所のテストは `manifest.len() == 26` と数を固定しているため、次にコマンドを足す作業が理由なく赤くなる | (1) code-summary.md の R-08 の記述に、始まりの view も同じ扱いであることを一言足す。(2) 数の固定を外し、3 つの一覧が互いに一致し cancel_fetch を含むことだけを確かめる | New |

### 確かめたこと（差分で反証を試みた結果）

**検証ツール**

| ツール | 結果 | 解釈 |
|---|---|---|
| cargo fmt --all --check | 合格 | 差なし |
| cargo clippy --workspace --all-targets | 合格 | 警告 0 |
| cargo test --workspace | 合格 | lib 451 件・u5_cancel を含む結合テスト・src-tauri 19 件が合格、失敗 0 |
| npx tsc --noEmit、npx eslint .、npx prettier --check . | 合格 | 指摘なし |
| npx vitest run | 合格 | 36 ファイル・385 件 |

**観点ごとの結果**

- biased select：ReadStage::run は止めを先に待ち、止めたら `reads` を落として抜ける。受け取った直後にも `is_stopped()` を見て、止めと同時に届いた応答は捨てる。捨てた読みは `schedule.finish` を通らないので、StreamSchedule は読み途中のままで矛盾せず、N は `outcome().unfinished_streams`（失敗は finish 済みなので入らない）になる。全ストリームを読み終えた後に止めが来ても N は 0 で、`decide_job_status` が Cancel でも通常の決め方に戻す（NFR4.5）。一覧の段も同じ形で、`PlanOutcome::Aborted` から Close は `abort()`、Cancel は一覧の前の Cancelled に分かれる。落とした futures が持つ許可や待ちは drop で解放され、job とタイムラインは ReadStage の外で一貫している
- decide_job_status：Close は何があっても Aborted、Cancel は plan なしまたは N が 0 より大きいとき Cancelled、それ以外は従来の表。表のテストが 5 つの場合を押さえている。キャッシュ書き込みは `decide_write` が Completed かつ失敗 0 のときだけなので、Cancelled は書かれない
- 閉じる側：Close は従来どおり破棄して Aborted。変更された u1 の 1 件は、応答を待つことを確かめていた部分を NFR4.3（中断でも閉じるでも打ち切る）に合わせて直したもので、Aborted・0 件・バッチなしの確認は残っている。code-summary.md の計画からのずれに理由が書かれており、正当と判断する
- ロック順：`begin_registered_fetch` も `cancel_running_fetch` も session → abort。abort だけを取る箇所（abort_fetch、取得タスクの後始末）は session を持ったまま abort 以外を取らないので、逆順の取り方は無く、デッドロックしない。持ち手の登録は session のロックの中なので、始めた直後の中断は新しい持ち手に届く。emit は session のロックの中で行うが待たない操作で、取得の終わりの view は session のロックを取った後に作られるので、終わりの view が必ず後になる
- cancelling の戻り：finish_fetch（Aborted を含むどの状態でも）、abort_fetch_with_failure、begin_fetch で false に戻り、テストが全部の終わり方を通している
- 画面：ConditionArea の 4 つのボタンの状態は R-09 の表のとおり。R-06 の effect は「ボタンが要素として無効か」で動き、中断中はフォーカスが残る。日付欄の Enter は取得中は欄が無効で何も起こさない。withProgress の cacheSaving で押せなくなる変更は core の BR5.2 と同じ判定
- i18n：キーは英日の両方があり、messages.test が揃いを確かめている。権限：build.rs、capabilities/default.json、generate_handler、自動生成の cancel_fetch.toml が一致し、テストで比較されている
- unwrap と expect：新しい非テストのコードに無い。新しい文言と README に秘密の情報は無い

### Summary

打ち切りの流れ（biased select、N の数え方、終わりの状態の表、Close 優先、ロック順、cancelling の戻り）は設計の正本のとおりに実装され、落とした futures が StreamSchedule や job を矛盾させる経路は見つからなかった。Critical と Major は無く、読み上げ領域の作り方・英語の複数形・古い view の記録と数の固定テストという Minor 3 件のみのため READY とする。
