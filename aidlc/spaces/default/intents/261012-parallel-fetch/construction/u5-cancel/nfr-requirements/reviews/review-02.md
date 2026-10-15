## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 2

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-requirements/performance-requirements.md > NFR4.4（あわせて reliability-requirements.md に該当する要件がないこと） | 応答待ちの最中に中断したときは、出ている応答が届くまで止まらない、と決めている。しかし今のコードには呼び出しごとの時間切れの設定が見当たらない（crates と src-tauri/src に TimeoutConfig などが無い。時間切れの扱いは gateway/aws.rs の分類だけ）。応答が返らない通信では、［中断中…］が上限なく続きうる。NFR4.4 の確かめ方も、偽物の 500 ミリ秒の応答だけで、止まるまでの上限の数値がない。中断はこの作業単位の中心の品質目標なのに、最悪の場合の止まるまでの時間が決まっていない。 | 応答待ちから止まるまでの最悪の時間を要件に書く。たとえば、呼び出しごとの時間切れを設けてそれを上限にする。あるいは、上限を設けず、応答が返らないときの抜け道は閉じるだけだと明記して受け入れる。どちらにしても、確かめ方（時間切れの偽物を使ったテストか、手元の確認）も書く。 | Resolved (reviewer) |
| R-02 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-requirements/reliability-requirements.md > NFR6.17 | ロックの順の要件は、確かめ方が「コードの確認」と、持ち手が無いときに何もしないテストだけである。今の begin_and_spawn_fetch（src-tauri/src/lib.rs）は、session のロックを手放した後で AppState の abort に持ち手を登録している。そのため、この要件が防ぐはずの「新しい取得の持ち手が登録される前に中断が届き、中断中だけが立って止まらない」状態が、今のコードの形では起こりうる。要件を満たす変更が必要なことが、要件の側に書かれていない。 | 現状の登録がロックの外にあることを前提として書く。登録を session のロックの内側に移すこと、および「始める操作の途中に中断が届いても、持ち手が古いまま中断中だけが立たない」ことを確かめるテスト（または、登録と状態の更新を 1 つの関数にまとめ、その単体テスト）を要件に足す。 | Resolved (reviewer) |
| R-03 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-requirements/performance-requirements.md > NFR5.5 | 確かめ方に「既存の速度測定テストが今と同じ基準で通る」とあるが、それらのテストは #[ignore] が付いていて（log_view.rs と timeline.rs）、CI（.github/workflows/ci.yml）では動かない。この作業単位は一覧の部品に手を入れないので、実質、確かめる対象がない。誰がいつ流すのかが書かれていない。 | 手元で --release --ignored を流すタイミング（Build and Test など）を書く。または、速度への影響がないことを理由に、確かめ方を画面のテスト（中断の後の絞り込み）に絞る。 | Resolved (reviewer) |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-requirements/security-requirements.md > NFR8.15 | 権限の確かめ方が「権限の設定ファイルの差分」だけになっている。この app では、コマンドごとの権限が src-tauri/build.rs の AppManifest に並べたコマンド名から作られ、capabilities/default.json の allow-* で許可される。build.rs への追加と generate_handler への登録が要件に出てこない。 | 確かめ方に build.rs のコマンド一覧、capabilities/default.json、ハンドラ登録の 3 か所がそろうことを足す。足すのは cancel_fetch に限り、ほかの権限が増えていないことも確かめる。 | Resolved (reviewer) |
| R-05 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u5-cancel/nfr-requirements/nfr-requirements-questions.md > 前提として決めたこと、および Consolidated Summary Confirmation | 質問票の前半に、旧い決めが残っている。応答待ちのときはその応答で止まる、止めた後に届いたページが最後かどうかは同じ読み手でもう一度次を求めて決める、という内容である。追加の質問 Q1 の回答（応答は待たずに打ち切る）と、performance-requirements.md の先頭の置き換えの説明と食い違い、読む人が旧い方を現行の決めと取り違えうる。成果物の側は正しく置き換わっている。 | 質問票の前半の該当の 2 項目に、追加の質問 Q1 で置き換わった旨の一文を足す。成果物は変えなくてよい。 | New |

> R-01 Reviewer note: 追加の質問 Q1 の回答 A で、中断したら出ている呼び出しを待たずに打ち切ると決まった。performance-requirements.md の NFR4.3 は、応答を返さない偽物でも、止めた時計を進めずに取得が終わることを確かめ方にしている。応答が返らない場合の上限なしの問題は解消した。coordinator.rs の読み取りループ（FuturesUnordered で読み、停止後に届いた結果を捨てる作り）は、止める信号と並べて待つ形にして future を落とすだけで足り、実現できる。tech-stack-decisions.md にも打ち切りの方法が書かれている。

> R-02 Reviewer note: reliability-requirements.md の NFR6.17 が、今の begin_and_spawn_fetch は session のロックを手放してから持ち手を登録している（src-tauri/src/lib.rs の 681〜682 行目と一致）と明記した。登録をロックの内側に移す変更、ロックの順、登録と状態の更新を 1 つの関数にまとめた単体テストが要件に入った。

> R-03 Reviewer note: NFR5.5 が、一覧の部品に手を入れないため速度測定テスト（#[ignore]）は流さないと明記し、確かめ方を画面のテスト（中断の後の絞り込み）に絞った。

> R-04 Reviewer note: security-requirements.md の NFR8.15 が、build.rs のコマンド一覧、capabilities/default.json、generate_handler の 3 か所に cancel_fetch がそろうこと、ほかの権限が増えていないことを確かめ方にした。

### 補足

- 検討したが指摘にしなかった点：requirements.md の FR5.2 と NFR4 は、出ている呼び出しの応答を待つ前提の文面だが、打ち切りは「待つ時間までに収まる」という上限を満たす。［中断中…］が一瞬で終わるだけで矛盾しない。FR5.9 の範囲が狭まる点は NFR4.5 と reliability-requirements.md の NFR6.18 に反映されている。
- 検討したが指摘にしなかった点：キャッシュ読み取りの spawn_blocking 中の中断は NFR6.19 が扱っている。応答を返さない偽物は今の FakeGateway にはまだないが、コード生成で足す性質のものである。
