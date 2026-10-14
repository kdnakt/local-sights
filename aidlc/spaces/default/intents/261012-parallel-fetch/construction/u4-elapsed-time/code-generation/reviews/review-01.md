## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | src/elapsed.ts > nextAnchor（sameFetch の判定） | 取得の番号は等しいかどうかだけで比べている。起動直後に getSession の古い応答が、後から届いた新しい取得の running より後に着くと、小さい番号の running が新しい取得として受け取られ、時計が一度古い値に戻る。次の session-changed で直るため害は小さい。 | 次に手を入れるときに、受け取った番号が表示中の番号より小さい値は無視するか、既知の制約として code-summary.md に残す。 | New |
| R-02 | Minor | src/components/ElapsedTime.test.tsx、src/hooks/useElapsed.ts | 描画中に state を合わせる形と useEffect の後始末は、StrictMode（描画と effect の二重実行）の下でテストされていない。コードを読む限り二重実行でも誤動作は見つからないが、保証するテストがない。 | StrictMode で包んだ描画で、タイマーが 1 本だけになることと時計が二重に進まないことを確かめるテストを 1 件足す。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| cargo fmt --all --check | PASS | 整形の問題なし |
| cargo clippy -p local-sights-core --all-targets | PASS | 警告なし |
| cargo test -p local-sights-core --lib session:: | PASS（115 件） | 経過の新規テストを含め全て通る |
| cargo check -p local-sights --tests | PASS | アプリ側は SessionView を直接組み立てておらず、変更の影響なし |
| npx tsc --noEmit | PASS | 型の問題なし |
| npx eslint . | PASS | 指摘なし |
| npx prettier --check . | PASS | 整形の問題なし |
| npx vitest run | PASS（35 ファイル、367 件） | 既存テストを変えずに通る |
| npm run test:coverage、cargo llvm-cov | 実行済み | 80% 未満を示す失敗は出ていない |

### 確認した点

**Rust（session.rs）**
- 書き込み点は 4 か所（begin_fetch、finish_fetch の Aborted と通常終了、abort_fetch_with_failure、apply_connection_change）で、設計と BR4.1〜BR4.4 に一致する。
- finish_fetch と abort_fetch_with_failure は、取得中・現在の取得・番号の一致を確かめた後でだけ確定する。そのため、古い終わりが新しい取得の記録を確定することはない。
- 戻らない時計と、時計が戻ったときの 0 の扱い（saturating_duration_since）を確かめた。テスト以外のコードに unwrap と expect はなく、unwrap_or は許される範囲である。
- SessionClock の Debug は名前だけを出し、中身は漏れない。AppSession の Debug を通してもテストで確かめている。
- serde の形は camelCase で、running、elapsedMs、fetchNumber の真偽値と数値だけ。null は None に対応する。

**画面**
- nextAnchor は計画どおり動く。同じ番号の running は大きいほうを保持し、番号が変われば置き換える。running false は最終の値で置き換え、終わりを見た番号の running は無視する。
- useElapsed は state を描画中に合わせる形（received と anchor）で、無限ループにならない。effect は anchor と tick に依存し、cleanup で clearTimeout する。ElapsedTime が外れた場合の後始末はテストで確かめている。
- aria-hidden は running の間だけ付く。StatusLine の毎秒の描き直しが ElapsedTime だけに限られることは、翻訳関数の呼び出し回数を数えるテストで確かめている。
- withProgress、withFilter、mergeSessionView が elapsed の同一性を保つことは App.test.tsx で確かめており、fetch-progress では時計が合わせ直されない。

**範囲**
- source-manifest の 13 件は、作業単位に関係する変更だけである。作業単位に無関係な主張は見つからなかった。

### Summary

計画・機能設計・非機能設計の設計書と実装は整合しており、検証ツールはすべて通る。Critical と Major の指摘はなく、小さな 2 点のみ。READY とする。
