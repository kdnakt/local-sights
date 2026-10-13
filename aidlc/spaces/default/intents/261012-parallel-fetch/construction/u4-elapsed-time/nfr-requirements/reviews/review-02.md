## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 2

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/nfr-requirements/reliability-requirements.md > NFR6.6（取得中の表示は減らない） | 「合わせ直しの値が今の表示より小さいときは今の表示を保つ」の適用範囲が書かれていない。機能設計の状態表（Final から次の取得を始めて Running に移る）では、直前の最終の値（例：00:31）が出たまま、次の取得の最初の SessionView が elapsedMs 0 付近の running で届く。「今の表示」と比べる決まりを文字どおり実装すると、2 回目の取得で表示が 00:31 に張り付き、31 秒たつまで進まない。FR4.2（押した時点から数える）に反する。NFR6.6 の確かめ方（running から running の 8,900 と、running false の 8,950）はこの境目を含まない。 | 「減らさない」を同じ取得の running から running の合わせ直しだけに限ると書く。新しい取得の始まり（前が Final または空で、新しい running を受け取ったとき）、elapsed が空になったとき、running false を受け取ったときは保持を解いて受け取った値に置き換える。画面のテストに「00:31 の最終の値が出ている状態で、新しい取得の running の 0 を受け取ると 00:00 から進む」を足す。 | Resolved (reviewer) |
| R-02 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/nfr-requirements/reliability-requirements.md > NFR6.9（合わせ直しは session-changed のときだけ） | 実際の App.tsx は `session-changed` のほかに、起動時の `getSession()` と `updateInput` の応答でも `setSession(view)` で完全な SessionView を受け取る。NFR6.9 の文言「session-changed のときだけ」では、画面の再読み込みなどで取得中に `getSession()` の結果が elapsed を running で運んできても、時計が始まらない（次の session-changed まで経過が出ない）。 | 合わせ直しの契機を「session-changed または getSession など、バックエンド由来の完全な SessionView が届いたとき」と書き、fetch-progress と filter-progress の合成だけを除外すると直す。getSession の running を受けて時計が始まるテストを足す。 | Resolved (reviewer) |
| R-03 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/nfr-requirements/tech-stack-decisions.md > 画面の時計 / reliability-requirements.md > NFR6.7 | ライブラリの Instant と画面の performance.now が、PC のスリープ中に同じように止まるか（または進むか）を確かめていない。食い違うと、スリープをまたいだ取得で画面の表示がライブラリの値より大きくなり、NFR6.6 の「減らさない」で高い値に張り付いたまま、終わりの最終の値で大きく戻る。NFR6.7 の「ずれは 1 秒以内」は無条件に書かれている。 | 配布対象の macOS（WKWebView）で両方の時計の挙動を確かめて書くか、NFR6.7 に「スリープをまたいだ場合は対象外」と明記する。 | Resolved (reviewer) |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/nfr-requirements/tech-stack-decisions.md > NFR6.12 | チーム規約は純粋なロジックをテスト先行にするが、NFR6.12 は画面の時計のフックを実装してからテストを書くとしている。合わせ直しと保持（max を取る決め、NFR6.6、NFR6.9）は副作用のない判断で、フックの中に置くと純粋な関数として切り出されずにテスト後書きになる。 | 合わせ直しと保持の判断を純粋な関数として切り出し、その関数はテスト先行と書く。タイマーに触れる部分だけを実装後のテストにする。 | Resolved (reviewer) |
| R-05 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/nfr-requirements/traceability.json > NFR8 の coverage / observability-requirements.md > NFR9.2 | NFR8（秘密を出さない）の対応先に、秘密と関係のない NFR8.12（取得 1 回の要約ログを変えない）が入っている。また NFR9.2 は、aria-hidden を外すと最終の経過が読み上げられると言うが、確かめ方は属性の有無だけで、読み上げられる保証はない。外した経過の部分が同じ更新で状態行の他の変化と一緒に出る前提も書かれていない。 | NFR8.12 を NFR8 の対応から外し、要約ログの維持として別に書く（または NFR8 の範囲を説明する）。NFR9.2 には、最終の経過が読み上げられるのは終わりの状態行の変化と同じ更新で出るためだと前提を書き、GUI での確認項目に加える。 | Resolved (reviewer) |
| R-06 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u4-elapsed-time/nfr-requirements/reliability-requirements.md > NFR6.6 | 新しい取得の始まりを「表示が最終の値か空のときに running を受け取った」で見分けているが、running の表示中に前の取得が終わらないまま別の取得の running が届く経路（例：AppSession の取り直し）があるかは書かれていない。通常は running false を挟むため実害は小さい。 | コード生成の計画で、running false を挟まずに新しい取得が始まる経路がないことを一度確かめる。あればその場合の扱いを NFR6.6 に足す。 | New |

> R-01 Reviewer note: NFR6.6 が保持の適用範囲を同じ取得の running から running だけに限った。新しい取得の始まり（表示が最終の値か空のとき）・running false・空では保持を解くと明記し、00:31 の状態から新しい取得の 0 を受けて 00:00 から進めるテストも書かれている。

> R-02 Reviewer note: NFR6.9 が、session-changed に加えて起動時の getSession と操作の応答の SessionView でも合わせ直すと書き換え、fetch-progress と filter-progress の合成だけを除外した。getSession で running を受け取ると時計が始まるテストも足されている。

> R-03 Reviewer note: NFR6.7 がスリープをまたいだ取得を「ずれ 1 秒以内」の対象外と明記し、終わったときはライブラリの最終の値を出すこと、GUI で一度見ておくことを書いた。

> R-04 Reviewer note: NFR6.12 が、合わせ直しと保持の判断を純粋な関数として切り出し、テスト先行と書いた。tech-stack-decisions.md の表も同じ内容になっている。

> R-05 Reviewer note: traceability.json の NFR8 の対応は NFR8.10 と NFR8.11 だけになった。NFR9.2 には読み上げの前提（属性を外すのが終わりの表示と同じ描き直しで起き、role="status" の領域の変化として伝わること）と、VoiceOver での GUI 確認が書かれている。StatusLine.tsx に role="status" と aria-live="polite" があり、前提は現行コードと矛盾しない。

### Summary

前回の Major 1 件（R-01）と Minor 4 件はすべて直っており、Critical も Major も残っていない。新しい指摘は Minor 1 件だけで、実装を妨げない。
