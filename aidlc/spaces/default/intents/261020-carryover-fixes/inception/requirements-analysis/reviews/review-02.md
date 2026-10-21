## Review

**Verdict:** READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-10-20T23:03:44Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261020-carryover-fixes/inception/requirements-analysis/requirements.md > FR4.1 (FR4.1.1、受け入れ基準) | 単数・複数の対象が「数が 1 のとき stream」としか書かれていない。`src/i18n/messages.ts` の実際の文では、(a) 中断の知らせ「{n} streams were not finished」は名詞だけでなく動詞（was / were）も数に合わせる必要がある。(b) 進み具合「{finished}/{planned} streams」「{done} / {total} streams」はどちらの数で単複を決めるかが未定。(c) 603 行の「listing streams, {count} selected」の streams は数に結びついていない文なのに、「7 か所」に数えられている。このままでは実装者が推測するしかなく、QA もテストを書けない。 | 7 か所それぞれについて、どの数で単複を決めるか（分数なら分母か）、動詞の一致も直すか、数に結びつかない文（603 行）を対象から外すか、を FR4.1 に表で書く。各文に単数の例（例：「1 stream was not finished」）を受け入れ基準に足す。 | Resolved (reviewer) |
| R-02 | Major | aidlc/spaces/default/intents/261020-carryover-fixes/inception/requirements-analysis/requirements.md > FR5.2〜FR5.6、A6、FR5.7 | 今のリージョンの選択欄は `TopBar.tsx` の `ButtonDropdown`（文字を打てない）で、FR5.3 の絞り込みは部品の差し替えが前提なのに、A6 は「替えてよい」と任意の書き方になっている。FR5.6 は「接続に使う振る舞いと確認ダイアログ」しか挙げておらず、今ある、サインイン中・取得中は選択欄を無効にする決まり（TopBar.tsx 76 行付近、261011 BR3.4）と、未選択のときの表示（`connection.region.placeholder`）、選んだ値を閉じたときにコードだけか名前つきかを書いていない。また FR5.7 の「表と `STANDARD_REGIONS` の一致」を確かめるテストは、表をどの層（Rust のアプリ側か TypeScript か）に置くかで書き方が変わるが、置き場所が決まっていない。 | A6 を「替える（FR5.3 に必要）」と言い切る。FR5.6 に、無効になる条件と未選択の表示、閉じているときの表示の決まりを足す。名前の表の置き場所（Rust／TypeScript）と、`STANDARD_REGIONS` との一致をどの層のテストで確かめるかを書く。 | Resolved (reviewer) |
| R-03 | Minor | aidlc/spaces/default/intents/261020-carryover-fixes/inception/requirements-analysis/requirements.md > FR2.2.1（begin_run の捨てる時点） | 捨てる時点が「実行が始まるとき」だけで、実行がクエリの構文エラー、ロググループの未選択、取得の失敗や中断で終わった場合に、選ばれていなかったロググループの保持がすでに失われていてよいかが書かれていない（`begin_run` は保持を捨てたうえで世代を進め、表示用の保持も空にする）。 | 失敗・中断した実行のあとも捨てた保持は戻らないこと（または構文エラーなど取得の前に落ちる実行では捨てないこと）を、FR2.2 に一文で決め、受け入れ基準に 1 つ足す。 | Unresolved |
| R-04 | Minor | aidlc/spaces/default/intents/261020-carryover-fixes/inception/requirements-analysis/requirements.md > FR1.3、受け入れ基準（200 回続けて流す） | 安定したことの確かめが、手元での 200 回の手動の繰り返しだけで、CI では守られない。順番の固定が壊れて再び不安定になっても、気づく仕組みがない。 | 順番を固定する道具そのもの（照合が答える前に中断が届くこと）を、時間に依らない形のテストで確かめる基準を足す。または 200 回の確認を Build and Test の記録に残すことを決める。 | Unresolved |
| R-05 | Minor | aidlc/spaces/default/intents/261020-carryover-fixes/inception/requirements-analysis/requirements.md > FR3.1、FR3.2、FR3.3（受け入れ基準） | 実際の古い説明は `plan.rs` の `DEFAULT_RESULT_LIMIT`（19〜20 行）の「`limit` から 1 から `MAX_LIMIT` がないとき」で、`limit 0` が 0 行になる今の決まりと食い違う。受け入れ基準の「矛盾しない」は目で読むだけで、検証の形が弱い。FR3.2 は終わった作業（261017）の記録を書き換えるが、対象の文書のパスが書かれていない。FR3.3 の受け入れ基準は書かれていない。 | FR3.1 の期待する文言（`limit` が 0 から `MAX_LIMIT` ならその値、なければ既定の上限）を決める。FR3.2 に対象文書のパスを書き、FR3.3 にも「doc コメントに該当の記述がある」という確かめを足す。 | Unresolved |
| R-06 | Minor | aidlc/spaces/default/intents/261020-carryover-fixes/inception/requirements-analysis/requirements.md > A1、A2、NFR1 | team.md Walking Skeleton は、両 OS に効く変更では Windows の手元の確認を必須としている。A1 は人の選択（Practices Discovery Q5 A）として例外にしているが、FR5（選択欄の部品の差し替え）と FR4.2 は画面に出る変更で、CI の `windows-latest` は GUI を描かないため、Windows での日本語名の表示や選択欄の操作は誰も確かめない。リスクが明示されていない。 | A1 に、Windows で見ない画面の変更（FR4.1、FR4.2、FR5）を挙げ、次の版を配る前の確認（`docs/releasing.md` の両 OS での手元の確認）で拾うことを書く。 | Unresolved |
| R-07 | Minor | aidlc/spaces/default/intents/261020-carryover-fixes/inception/requirements-analysis/requirements.md > NFR4（FR2.1 の速さ） | 「読む行が減る」は測れる基準になっていない。FR2.1 の受け入れ基準は読んだ行数を確かめるのでテスト側は足りるが、NFR4 の文は目標のように読める。 | NFR4 を「既存の速さのテストが通る」と「FR2.1 の読んだ行数のテスト」に分けて書くか、行が減る点を削る。 | Unresolved |
| R-08 | Minor | aidlc/spaces/default/intents/261020-carryover-fixes/inception/requirements-analysis/requirements.md > FR5.10 | 一致のテストは Vitest が Rust のソース (regions.rs) を読む前提だが、読み方 (ファイルの取り方、配列の取り出し方、リポジトリの相対パス) が書かれていない。Rust 側の並びの書式が変わるとテストが壊れやすい。受け入れ基準の「消す・足すと落ちる」は確かめられるので、実装を止めるほどではない。 | Code Generation の計画で、取り出し方 (例：STANDARD_REGIONS の文字列リテラルを正規表現で抜く) と、読めなかったときは落ちる (黙って通らない) ことを決める。 | New |

> R-01 Reviewer note: FR4.1 の表で、決め手の数、動詞の一致 (was/were)、単数・複数の例、対象外の文 (603 行と 209 行の先頭) が決まった。行番号は messages.ts と一致する。

> R-02 Reviewer note: FR5 で部品の差し替え、無効・未選択・閉じているときの表示、名前の表の置き場所 (画面側)、STANDARD_REGIONS (34 個) との一致を確かめるテストが決まった。番号の参照にも壊れはない。

> R-03 Reviewer note: 人の判断で未対応。失敗・中断した実行のあとに捨てた保持が戻るかどうかが未記載のまま。

> R-04 Reviewer note: 人の判断で未対応。安定の確かめが手元の 200 回の繰り返しだけで、CI では守られないまま。

> R-05 Reviewer note: 人の判断で未対応。FR3.1 の期待する文言、FR3.2 の対象文書のパス、FR3.3 の受け入れ基準が未記載のまま。

> R-06 Reviewer note: 人の判断で未対応。Windows で見ない画面の変更 (FR4.1、FR4.2、FR5) のリスクが未記載のまま。

> R-07 Reviewer note: 人の判断で未対応。NFR4 の「読む行が減る」が測れる基準になっていないまま。

### Summary

人が直すと決めた Major の R-01 と R-02 は解消し、番号の参照の壊れや矛盾もない。未対応の Minor (R-03 から R-07) は重さを変えずに残り、新しい Minor を 1 件足したが、Critical と Major は残っていないので READY とする。
