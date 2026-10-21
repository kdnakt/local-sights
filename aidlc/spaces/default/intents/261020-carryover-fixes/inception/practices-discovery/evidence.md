# 証拠と推定（261020-carryover-fixes の Practices Discovery、再実行）

> リード（pipeline-deploy）が面談の答えを反映した最終版。協力者（quality・developer・devsecops）は切られているため、リードの下書きがそのまま面談の案になり、面談の 6 問の答えをここに統合した。基準線は `aidlc/spaces/default/memory/team.md` の affirmed 済みの 5 節で、節ごとに「何を読み、基準線と合っているか、何を変えたか」を書く。面談の決定（Q1〜Q6 の答えと反映のしかた）は末尾にまとめた。

## 読んだもの

| 区分 | ファイル・コマンド | 使いどころ |
|---|---|---|
| 基準線 | `aidlc/spaces/default/memory/team.md`（5 節）、`project.md`（Forbidden 4 件、Mandated `None.` 4 行、Corrections）、`org.md` | 今の決まり。食い違いの有無を比べる元 |
| 作業の記録 | `aidlc/spaces/default/intents/261020-carryover-fixes/aidlc-state.md`（Scope `bugfix`、Test Strategy `Standard`、Project Type Brownfield） | この作業に効くスコープ別の決まりの確認 |
| コードの知識ベース（2026-10-20、コミット `3208713`） | `aidlc/spaces/default/codekb/local-sights/code-structure.md`、`technology-stack.md`、`dependencies.md`、`code-quality-assessment.md`、`architecture.md`、`business-overview.md` | 構成・検査・配布の仕組みの現状と、依頼の 5 項目の場所 |
| CI とリリース | `.github/workflows/ci.yml`、`release.yml`、`release-dry-run.yml`、`.github/dependabot.yml` | ジョブ名、カバレッジ、cargo-deny、Release の流れ、インストーラの形 |
| 文書 | `docs/releasing.md`、`docs/install.md`、`README.md`（「Tests and checks」「Releasing」「Secrets」の節）、`.gitattributes`、`Cargo.toml`（`version = "0.2.0"`） | 手順書と版の現状 |
| git | `git log --oneline -30`、`git rev-parse HEAD`（下書きの時点で `83cf8c3a2dcff36cf8daf433156f0cbc8115f007`。面談後の統合の時点では `cd21c7c9364f00261eabf0367f568cb7fecfb571` で、`practices-discovery-timestamp.md` にはこちらを記した）、`git tag --list`（`v0.1.0`、`v0.1.1`、`v0.2.0`）、`git status -sb`、`git branch -a --contains 8bb3759` | 公開済みのタグ、PR の流れ、今いるブランチ |

テスト・ビルドは実行していない。AWS にはつないでいない。

## 節ごとの確認

### Way of Working — 変更なし（Q4 の答え A で確定）

- 直近 30 コミットのうち `main` 上のものはすべて PR 番号（`(#N)`）を持ち、スクワッシュマージの 1 コミットになっている（`git log`）。番号のない 2 コミット（`8bb3759`、`83cf8c3`、どちらも AI-DLC の記録）は短命のブランチ `claude/serene-tesla-lns2q0` 上にあり、`main` には入っていない（`git branch -a --contains`）。「`main` への直接 push はしない。ドキュメントだけの変更も同じ流れに乗せる」と合う。
- `ci.yml` は `pull_request` と `push`（`main`）で動き、先頭のコメントに「PRs are squash-merged into main once every job here is green」とある。
- ジョブ名は `Rust library (fmt, clippy, test)`、`Front end (tsc, eslint, prettier, vitest, build)`、`cargo-deny`、`Tauri app (${{ matrix.os }})`（`ubuntu-latest`・`macos-latest`・`windows-latest`）で、`ci.yml` 96〜97 行に「The job names are required status checks of the main ruleset: keep them as they are」と書かれている。節の例（`Tauri app (windows-latest)`）と一致する。ルールセット自体は GitHub の設定で、リポジトリの中からは確かめられない。
- 長命のブランチはない（`git branch -a` に見えるのは今の作業ブランチだけ。リリースは `main` 上のタグ）。
- 確定：文面は基準線のまま（Q4 の答え A）。

### Walking Skeleton — 変更なし（Q4 の答え A、Q5 の答え A で確定）

- 節の 3 つの確認（`cargo test`、確認用プログラム、GUI の目視）に対応するものがコードにある：`cargo test` は CI の 3 OS で流れる（`ci.yml`）、確認用プログラムは `crates/local-sights-core/examples/fetch_check.rs` と `compat_check.rs`（`code-structure.md`）、`README.md` に手元の確認手順がある（`code-quality-assessment.md` の「ドキュメント」）。
- この作業は `bugfix` スコープで、記録（`aidlc-state.md`）に Unit DAG を作るステージ（units-generation）はなく、骨組みのチェックポイントの儀式は起きない（org.md「no Unit DAG means no skeleton checkpoint ceremony」）。節は「各作業の最初の作業単位で」の決まりとして残す。
- 依頼文の前提「Windows 11 の手元の確認は今回も飛ばす」は、節の「両方の OS に効く変更では、2 と 3 を macOS と Windows の両方で行う」に対するこの作業だけの例外である。今回の 5 項目のうち OS ごとに分かれる境界のコードに触るものはなく（`code-quality-assessment.md`：(1) は `tests/u5_cancel.rs` と `coordinator.rs`、(2) は `cache/mod.rs` と `query/`、(4)(5) は画面と `lib.rs` のテスト）、Windows だけで違う振る舞いは想定しにくい。面談（Q5 の答え A）で、チームの決まりとしての文面は変えず、この作業だけの例外として作業の記録（要件分析と Build and Test）に残すことに決めた。節に条件（「利用者に見える変更が小さい作業では省いてよい」など）は足さない。
- 確定：文面は基準線のまま（Q4 の答え A、Q5 の答え A）。

### Testing Posture — 変更なし（Q4 の答え A で確定）

- `Methodology: custom` / `Ordering` の基準線に対し、今のコードは判断を副作用のない関数に寄せ（時刻順マージ、ページの終端、再試行、ペース、諦め、絞り込み、クエリの読みと評価、日付、SSO の出力の抜き出し、画面側の `elapsed.ts` ほか）、OS に触れる処理を `cfg` の境界に閉じ込めている（`code-quality-assessment.md`「純粋なロジックの比率」、`architecture.md`「判断は OS に依存しない関数へ」）。進め方を変える跡はない。
- カバレッジ 80%：`ci.yml` 44 行 `cargo llvm-cov -p local-sights-core --fail-under-lines 80`（Linux）、73 行 `npm run test:coverage`（Vitest、行 80%。`technology-stack.md` も同じ）。
- Rust のテストは 3 OS：`rust-core`（Linux、カバレッジつき）、`tauri-app` の `tests (library)`（Linux 以外）と `tests (app)`（全 OS）。画面のテストは Linux の `frontend` だけ。節の記述と一致する。
- AWS につながない：`ci.yml` 3〜5 行のコメント、結合テストは `FakeGateway`、CLI は偽のスクリプト／`aws.cmd`（`code-quality-assessment.md`、`dependencies.md`）。
- この作業に効くスコープ別の決まり：記録のスコープは `bugfix` で、org.md の Testing Posture は `bugfix` に「その不具合に対する狙ったリグレッションテストを足し、既存のスイートを緑に保つ」を求める。節の「そのほかのスコープ別の決まりも org.md に従う」でこれが効くため、(1) の `u5_cancel` の安定化には落ちていた条件を固定するテストが要る。節の文面に足す必要はない（org.md から引ける）。面談では 4 節とも変更なし（Q4 の答え A）で確かめた。
- `Methodology evidence` の文はこれまでの面談の経緯をまとめたもの。面談（Q4 の答え A）は「今の team.md の文面を一字も変えない」なので、「261020-carryover-fixes でも変えていない」の一文も足さず、`- **Methodology**: custom` / `- **Methodology evidence**:` / `- **Ordering**:` は基準線のままにした。
- 確定：文面は基準線のまま（Q4 の答え A）。

### Deployment — 「次の版」の箇条書きを書き換えた（Q1 の答え C、Q2 の答え B、Q3 の答え C で確定）

基準線と証拠が合っている点：

- 配布の形：`tauri.conf.json` の NSIS は `installMode: currentUser`（`technology-stack.md`）、`release.yml` の `build-windows` は `*-setup.exe` の x64 を 1 つだけ確かめる。macOS は `.dmg`（`aarch64`）を 1 つだけ確かめる。
- 作る場所と置き場所：`release.yml` は `v*` のタグで `verify` → `test` → `build-macos` と `build-windows` → `publish`。`publish` は `needs: [verify, test, build-macos, build-windows]` で、両方のファイルがそろったときだけ `gh release create --verify-tag`（下書きではない）。節のとおり。
- リリース前の確認：`release-dry-run.yml`（`workflow_dispatch`、タグも公開もなし、成果物は 7 日）。
- 署名と公証：`APPLE_SIGNING_IDENTITY: "-"` でアドホック署名し、`codesign --verify` と `Signature=adhoc` を確かめる。Windows は署名なし。`docs/install.md` に Gatekeeper（「Done」→「Open Anyway」）と SmartScreen（「詳細情報」→「実行」）の開き方がある。
- バージョン：`npm run check:version`（CI）と `check:version -- --tag`（Release の `verify`）で 5 ファイルの版とタグの一致を確かめる（`docs/releasing.md` 28〜45 行）。`Cargo.toml` の版は `0.2.0`。
- 更新と戻し方、手順書の置き場所：`docs/install.md` に macOS と Windows の節が分かれ、AWS CLI v2 の入れ方と探す場所（`C:\Program Files\Amazon\AWSCLIV2\aws.exe` → `PATH`）がある。
- 細かな観察（変更にはしない）：節の「GitHub Actions がテストを流してから」に対し、`release.yml` の `test` ジョブはライブラリのテストと Vitest を Linux で流すだけで、アプリのテストと 3 OS の実行は CI（PR 時）に任せている。タグは CI が緑の `main` に打つ前提なので節の読みと矛盾しない。面談（Q3）では細かく書き直す選択肢 B は選ばれなかったため、文面は今のままにした。

基準線と合わなくなった点（書き換えの案）：

- 「次の版のタグは `v0.2.0` とする。マージ済みの取得の高速化（PR #22）と Windows 版を合わせてこの版で配り、`v0.1.2` は付けない。…」と、その下の learned 行「`v0.2.0` には … IB-11 も含める。IB-11 が終わるまで `v0.2.0` のタグは付けない」は、2026-10-20 の `v0.2.0` の公開で過去のものになった。`git tag --list` は `v0.1.0`、`v0.1.1`、`v0.2.0` を返し、`v0.2.0` は `main` の `cd453f0`（Dependabot の PR #32 のコミット）に打たれている（依頼文、https://github.com/kdnakt/local-sights/releases/tag/v0.2.0 ）。`v0.2.0` に入ったのは `git log` で、取得の高速化（#22）、Windows 版（#23・#24・#25）、IB-11（#34）、Dependabot の更新 8 本（#26〜#33）。
- `main` には `v0.2.0` のあとに 1 コミット（`3208713`、PR #35、互換の確かめの記録）が入っている。次の版にはこれも含まれるが、面談（Q3）で「PR #35 を含むと書く」選択肢 A は選ばれなかったため、節には書かない。
- 書き換えた内容（`team-practices.md` の `## Deployment`）：
  1. 「次の版のタグは `v0.2.0` とする。…」の箇条書きと、その下の learned 行（`<!-- cid:261017-insights-query:approval-handoff:… -->` つき）は過去の決定で、置き換え後の節には残さない（promote が節ごと置き換えるため、履歴は監査ログと旧版の `team.md` に残る）。
  2. 代わりに「次の版の番号は、タグを打つ直前に決める。`v0.2.0` のあとの作業（持ち越し Minor の一括修正など）はそれだけでは配らず、次の機能が終わるのを待って合わせて配る。配るときは手順書（`docs/releasing.md`）どおり、版を上げる PR → Release dry run → 両 OS での手元の確認 → タグの順で進める」の箇条書きを入れた（Q1 の答え C と Q2 の答え B）。下書きの案だった `v0.2.1` は採らず、番号は書かない。
  3. 公開済みの版の一覧を独立した箇条書きにし、「公開済みの版は `v0.1.0`・`v0.1.1`・`v0.2.0`（どれも `main` 上のコミット）。一度公開したタグは付け替えも使い回しもしない」とした（Q3 の答え C）。`v0.2.0` の中身や公開日は節には書かず、この文書に残す。
  4. ほかの Deployment の箇条書き（配布の対象・形・作る場所と置き場所・dry run・署名と公証・バージョンのそろえ方・更新・戻し方・手順書・AWS へのデプロイなし・タグで区切る）は今の文面のまま。`v0.2.1` と PR #35 への言及は節に残していない。
- 下書きで `v0.2.1` を案にした理由（利用者に見える変更が (5) のリージョン名の表示だけで、API や保存形式を変えないためパッチの版に当たると読んだ。ただし新しい機能とも読めるため `v0.3.0` の余地もあった）は、面談で「番号はタグを打つ直前に決める」（Q1 の答え C）となったため、ここに経緯として残すだけにする。

### Code Style — 変更なし（Q4 の答え A で確定）

- `cargo fmt --all --check`（`ci.yml` 38 行）、`npx prettier --check .`（71 行）、`cargo clippy`（`RUSTFLAGS: ""`、21 行のコメント「clippy warnings stay warnings (team.md: no -D warnings)」）、ワークスペースの clippy とアプリのビルドが 3 OS（`tauri-app`）、`npx eslint .`（69 行）、`npx tsc --noEmit`（67 行）、cargo-deny（`EmbarkStudios/cargo-deny-action`、`deny.toml` で実際にビルドする機能だけ）、`npm audit --audit-level=high`（77 行）、`.gitattributes` の `* text=auto eol=lf`。節の各項目と一致する。
- 命名と構成、`unwrap()` / `expect()` の制限は `code-structure.md`「コードの決まりごと」のとおり守られている（`unwrap` / `expect` はテストと `tests/support/` に限る。`fake_gateway.rs:293-297` の `expect` はテストの道具の中）。
- 「GitHub の秘密情報スキャン（secret scanning）を有効にする」は GitHub の設定で、リポジトリの中からは確かめられない。前回の面談で affirmed 済みのため、そのまま残す。
- 確定：文面は基準線のまま（Q4 の答え A）。

### Mandated / Forbidden — どちらも `None.`（Q6 の答え A で確定）

- 依頼文に新しい強い制約はない。「既存のテストはすべて緑のまま」は org.md の `bugfix` スコープと team.md の Testing Posture がすでに求めている。「Windows 11 の手元の確認は飛ばす」はこの作業だけの例外で、恒久の決まりではない。
- `project.md` の Forbidden 4 件は affirmed 済み（2026-10-03）で、今回の 5 項目はいずれもそれに触れない（(5) のリージョン名は表をコードに持つだけで新しい API は呼ばない。`technology-stack.md` の末尾）。
- 面談（Q6 の答え A）で、Mandated・Forbidden はどちらも `None.` と確かめた。`discovered-rules.md` もそのとおり。

## コードの文書に残る古い記述（この作業の修正候補、team.md ではない）

- `docs/releasing.md` 47〜49 行：「The next release is `v0.2.0`: the first one with the Windows installer, together with the faster fetching merged before it. There is no `v0.1.2`.」。`v0.2.0` は公開済みなので古い。同じファイルの手順の例（53〜57 行の `0.2.0`、80〜82 行の `v0.2.0`、87 行、126〜130 行、137 行の「`v0.2.1` after `v0.2.0`」）は例として読めるが、次の版の記述と合わせて直すのが自然。これは team.md ではなくコード側の文書の古さなので、依頼の (3)「記述の古さ」に加える候補として要件分析で扱う。面談の決定（次の版の番号はタグを打つ直前に決める）に合わせ、直すときも特定の番号を書かない形がよい。
- `docs/install.md` の `Local-Sights_0.2.0_aarch64.dmg` / `Local-Sights_0.2.0_x64-setup.exe` は「Do not rely on the exact file name」つきの例なので、そのままでも誤りではない。

## 面談の決定

面談は `practices-discovery-questions.md` の 6 問で行った（下書きで挙げていた 8 問のうち、4 節の「変更なし」の確認は Q4 に、Testing Posture の `bugfix` の認識合わせは Q4 の説明に、Mandated / Forbidden は Q6 にまとめた）。答えと反映先は次のとおり。

| 質問 | 答え | 反映 |
|---|---|---|
| Q1 次の版の番号 | C（この作業では番号を決めず、タグを打つ直前に決める） | `team-practices.md` の Deployment に「次の版の番号は、タグを打つ直前に決める」と書いた。下書きの `v0.2.1` の案は採らず、`v0.3.0` も書かない。 |
| Q2 タグを打つ時期 | B（この作業だけでは配らない。次の機能が終わるのを待って合わせて配る） | 同じ箇条書きに「`v0.2.0` のあとの作業はそれだけでは配らず、次の機能が終わるのを待って合わせて配る。配るときは手順書どおり、版を上げる PR → Release dry run → 両 OS での手元の確認 → タグの順」と書いた。 |
| Q3 Deployment 節の細部 | C のみ（公開済みの版の一覧を独立した箇条書きにし、「一度公開したタグは付け替えも使い回しもしない」を添える） | 「公開済みの版は `v0.1.0`・`v0.1.1`・`v0.2.0`（どれも `main` 上のコミット）。一度公開したタグは付け替えも使い回しもしない」を独立した箇条書きにした。A（PR #35 を含むと書く）と B（`release.yml` の実際に合わせて細かく書く）は入れていない。 |
| Q4 ほかの 4 節 | A（4 節とも変更なし） | Way of Working・Walking Skeleton・Testing Posture・Code Style は今の `team.md` の文面を一字も変えていない。Testing Posture の `- **Methodology**: custom` / `- **Methodology evidence**:` / `- **Ordering**:` も基準線のまま。 |
| Q5 Windows の手元の確認を飛ばす扱い | A（この作業だけの例外として作業の記録に残す。節の文面は変えない） | Walking Skeleton の文面は変えない。例外は要件分析と Build and Test の記録に残す（この文書の Walking Skeleton の項にも書いた）。 |
| Q6 新しい強い制約 | A（Mandated・Forbidden はどちらも `None.`） | `discovered-rules.md` の `## Mandated` と `## Forbidden` はどちらも `None.`。`project.md` の既存の Forbidden 4 件はそのまま効く。 |

## 未解決の不確かさ

- `main` のルールセットの必須の状態確認の名前と、secret scanning の有効化は GitHub の設定で、リポジトリの中からは確かめられない。`ci.yml` のコメントと前回の affirmed を信じている。
- `v0.2.0` の Release の中身（2 つのファイルが載っていること）は依頼文の記述と `release.yml` の仕組みから推定しており、Release のページ自体は開いていない。
- 次の版の番号は面談でタグを打つ直前に決めることになった（Q1 の答え C）ため、この作業では未決のまま残る。決めるときは、(5) のリージョン名の表示を「新しい機能」と数えるか「見た目の改善」と数えるかが分かれ目になる。
- この作業の Construction で (1) の直し方（外れの側にも台本を持たせる／止める合図の順序を保証する／テストを 2 つに分ける）はまだ決まっていない。どれを選んでも Testing Posture の文面は変わらないが、`bugfix` の「狙ったリグレッション」の形はそこで決まる。
