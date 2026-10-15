## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/functional-design/functional-spec.md > 1.1 取得（手順 3）、3.2 時計のフック、rules.md > BR4.6 | 画面が時計を合わせ直す契機が「SessionView を受け取るたび」としか書かれていない。実際のコードでは `withProgress` と `withFilter`（src/api.ts）が `{...view}` で新しい session オブジェクトを作る。実装者が session のオブジェクトの同一性をキーにすると、ページごとの fetch-progress のたびに古い `elapsed` で baseMs と receivedAt が置き換わり、表示が後ろへ戻る。手順 3 は「一覧の進み・取得の進み・キャッシュへの保存」で SessionView が届くと書くが、`on_listing_progress`・`on_batch`・`on_saving` は軽い `fetch-progress` で送られ（src-tauri/src/lib.rs の TauriSink）、SessionView は `on_started`・`on_planned`・`on_stream_finished`・`on_finished` で届く。 | 合わせ直しの契機を「`session-changed` で届いた新しい `elapsed` の値（オブジェクト）が変わったとき」と明記し、fetch-progress と filter-progress の合成では合わせ直さないと書く。手順 3 の説明を実際の送り方に直す。画面のテストに「fetch-progress の合成では時計が戻らない」を 1 件足す。 | New |
| R-02 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/functional-design/functional-spec.md > 3.3 読み上げ、rules.md > BR4.9 | 取得中に経過を読み上げの対象から外す方法が書かれていない。上流のレビュー（refined-mockups R-04）が方法を求めていたが、「対象から外す」のままである。状態行は `role="status" aria-live="polite"` の一つの領域なので、方法が決まらないと 4 章の「読み上げの対象外」のテストで何を確かめるかが決まらない。 | 方法を一つ決めて書く（例：running の間だけ経過の span に `aria-hidden="true"`、running でなくなったら外す）。テストはその属性の有無で確かめると書く。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/functional-design/functional-spec.md > 3.2 時計のフック | 1 秒ごとの描き直しをどの部品が持つかが書かれていない。フックを App のような上位で持つと、毎秒、ログの表を含む画面全体が描き直される。上流の interaction-spec は ElapsedTime を StatusLine の一項目としている。 | 時計の状態と 1 秒ごとの更新は StatusLine の中（または経過だけを描く小さな部品）に閉じ込め、上位の部品は毎秒描き直さないと書く。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/functional-design/functional-spec.md > 4 確かめ方（始まりと最終の値）、rules.md > BR4.10 | AppSession の時計を差し替える仕組みの形が書かれていない。今の AppSession は `#[derive(Debug, Clone)]` で、Arc<Mutex<AppSession>> として別タスクと共有される。時計を持たせる場合は Clone・Debug・Send・Sync を満たす形が要り、`view()` は `&self` で現在時刻を読む。表を見ただけでは、Rust のテストが共通の差し替え方で書けるか分からない。 | 時計は AppSession のコンストラクタか builder（例：`with_clock`）で渡す、Clone・Debug・Send・Sync を満たす小さな型にする、と書く。本番は Instant、テストは決まった時刻を返すもの。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/functional-design/rules.md > BR4.4、traceability.json > AC2.1.3 の coverage | AC2.1.3 と FR4.3 は「最終の経過を次の取得を始めるまで残す」と書くが、BR4.4（質問 Q2 の回答 A）は接続を変えると消す。これは利用者が選んだ逸脱だが、traceability の AC2.1.3 は逸脱の注記なしで OK になっている。後の段（build-and-test など）が AC をそのまま検証すると、BR4.4 のテストと食い違う。contract-summary の C5 も `elapsed` の null を「一度も取得していない」としか定義していない。 | AC2.1.3 の対象に BR4.4 を加え、接続の変更で消えるのは Q2 で承認した例外だと functional-spec の Assumptions & Open Questions か traceability に一行書く。C5 の null の意味に「接続の変更で結果が消えたとき」を加える扱いを U5 側と合わせる。 | New |
| R-06 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/functional-design/rules.md > BR4.6 | 新しい `elapsed` を受け取るたびに baseMs を置き換える決まりだけでは、送受信の遅れで取得中の表示が 1 秒後ろへ戻ることがある（Rust で 8.9 秒に作った値が画面に 9.1 秒で届くと、00:09 から 00:08 に戻る）。ADR-005 が認めるのは終わりのときの小さなずれで、取得中に数字が減ることまでは認めていない。 | running から running への合わせ直しでは、表示する値が減らないようにする（直前の表示値との max を取る）と書くか、戻りを許すなら ADR-005 のずれの範囲と明記する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| 検証ツールの指定なし | 実行していない | 段の定義に検証ツールの指定がなかったため。代わりに session.rs の phase を変える 4 か所（begin_fetch、finish_fetch の Aborted と通常、abort_fetch_with_failure、apply_connection_change）と、BR4.1〜BR4.4 の対応を手で突き合わせた。取りこぼしはない。 |

### Summary

ElapsedRecord の状態遷移は、今の session.rs で段階を変える 4 か所と、キャッシュ保存を終わりの前に置く coordinator の順序にすべて合っており、BR4.1〜BR4.10 と AC2.1.1〜AC2.1.7 の対応も足りている。Critical と Major はなく、READY とする。上の Minor は、画面の合わせ直しの契機、読み上げの方法、時計の差し替えの形、AC2.1.3 との逸脱の記録を実装前に一行ずつ書き足すためのもので、実装を止めるものではない。
