# Evidence

> 下書き（リード、Step 2）。協力エージェントなしの実行（collaborators: off）のため、調べたのはリード（aidlc-pipeline-deploy-agent）1 人で、寄稿ファイルはない。再実行なので、確定済みの `team.md` を基準にし、今の証拠と食い違う点だけを洗い出した。チームの意図は決めていない。証拠で決まらない点は「面談で聞くこと」にまとめた。

## 調べたもの・推論したこと（リード：aidlc-pipeline-deploy-agent）

| 対象 | 内容 |
|------|------|
| `aidlc/spaces/default/memory/team.md` | 5 セクションとも確定済み（261011-local-release で Deployment・Testing Posture・Code Style を更新）。再実行の基準として使った。 |
| `aidlc/spaces/default/memory/project.md` | `## Forbidden` の 4 件と `## Mandated` の `None.`（2 回）。`## Corrections` に、`aws sso login` の起動は Forbidden の対象外とする補足がある。今回の下書きはこれらに触れない。 |
| `aidlc/spaces/default/memory/org.md` | トランクベース・スクワッシュマージ。Testing Posture のスコープ別の下限：`mvp` は 80% の行カバレッジ下限と、マージ前の CI での実行を加える。 |
| `aidlc-state.md`（261012-parallel-fetch） | Brownfield、スコープ `mvp`（`skeleton: on`）、Test Strategy `Standard`、Operation はすべて SKIP、collaborators off。これまでの intent のスコープは `local-tool`・`bugfix`・`bugfix`・`refactor` で、`mvp` は今回が初めて。そのため org.md の 80% 下限が当てはまるのも今回が初めて。 |
| 261002-cwlogs-viewer と 261011-local-release の `inception/practices-discovery/` | 前回までの決定（Testing Posture の `custom`、Deployment の Q1〜Q5、GitHub 側の設定は有効という Q7 の回答）。 |
| 261011-local-release の `inception/requirements-analysis/requirements-analysis-questions.md` ほか | Q5 の回答 B（CI が最初から公開し下書きにしない）と Q5-1 の回答 A（「下書きとして置く」の記述は記録の手順で直す）。`requirements.md:124` が記述の直しを予告し、`construction/code-generation/reviews/review-01.md` の R-01 が直し忘れの防止を求め、`construction/build-and-test/build-and-test-summary.md:46` が「team.md は下書きのまま」と記録している。記述は今も直っていない。 |
| 逆解析の成果物：`code-structure.md`、`technology-stack.md`、`dependencies.md`、`code-quality-assessment.md`、`architecture.md`、`business-overview.md`（`aidlc/spaces/default/codekb/local-sights/`、コミット `9d08cbf`） | 中核とアプリの分離、trait の裏の AWS、テスト以外に `unwrap()`／`expect()` がないこと、秘密を出さないことはコード上で守られている（`code-quality-assessment.md` 総評、`code-structure.md:96`）。テストは中核 409・アプリ 12・画面とスクリプト 281。カバレッジの計測設定はない。`code-quality-assessment.md` の D4（CI のコメントと `deny.toml` の食い違い）と D5（リリースの下書き）を、下の「ずれ」で確かめ直した。`technology-stack.md` と `dependencies.md` から CI・Dependabot・`cargo-deny` の構成、`business-overview.md` と `architecture.md` から配布の流れと境界の置き方を確かめた。 |
| `git log`（`main` は 17 件の PR 由来のスクワッシュコミット、`9d08cbf` (#17) まで） | `main` のコミットはすべて `(#n)` 付き。直接 push の跡はない。今の作業ブランチは `claude/parallel-fetch`。下書きの時点の HEAD は `78bef39`（AI-DLC の記録だけの変更）。 |
| リモートのタグ（`git ls-remote --tags origin`、`git fetch --tags` で取得） | `v0.1.0` → `9d08cbf`（`main` の #17）。`v0.1.1` → `f25a193`（`origin/sso-login` 上の「AI-DLC: SSO login intent records up to approved code-generation plan」、`main` にはない。その時点の版は 3 か所とも `0.1.0`）。どちらも軽量タグ。なお `git fetch` で手元のタグとリモート追跡ブランチ（`origin/sso-login`）が増えたが、ファイルは変えていない。 |
| GitHub の設定（REST API で読み取りのみ） | `main` のルールセット（有効）：削除禁止、強制 push 禁止、PR 必須（承認 0 人、マージ方法はスクワッシュのみ）、必須の状態確認は CI の 5 ジョブ（`Rust library (fmt, clippy, test)`、`Front end (tsc, eslint, prettier, vitest, build)`、`cargo-deny`、`Tauri app (macos-latest)`、`Tauri app (ubuntu-latest)`）。secret scanning と push protection は有効、Dependabot のセキュリティ更新も有効。タグを守るルールセットはない。`delete_branch_on_merge` は false で、マージ済みの `sso-login` ブランチがリモートに残っている。Releases は `v0.1.0`（`draft: false`、`Local-Sights_0.1.0_aarch64.dmg` 1 つ）だけ。Release ワークフローの実行は `v0.1.0` が成功、`v0.1.1` が 14 秒で失敗（`verify` で止まったと考えられる）。 |
| `.github/workflows/ci.yml` | PR と `main` への push で実行。`RUSTFLAGS: ""`（21 行目、警告はエラーにしない）。`rust-core`（fmt・ライブラリの clippy・`cargo test -p local-sights-core`）、`frontend`（`npm run check:version`、`tsc --noEmit`、`eslint`、`prettier --check`、`vitest run`、`vite build`、`npm audit --audit-level=high`）、`cargo-deny`、`tauri-app`（Linux と macOS でワークスペースの clippy、ビルド、`cargo test -p local-sights`（118 行目、#17 で追加））。76 行目のコメントは「`deny.toml` の `[graph] all-features = true`」。 |
| `.github/workflows/release.yml` | `v*` タグで実行。`verify`（`npm run check:version -- --tag`、タグのコミットが `main` にあるか：49・54 行目）→ `test`（ライブラリと画面のテスト）→ `build-and-publish`（Apple Silicon の確認、未署名の `.dmg`、1 つだけあるかの確認、`gh release create ... --verify-tag`（138 行目、`--draft` なし））。4 行目のコメントは「publishes it on GitHub Releases (not a draft)」。秘密は `GITHUB_TOKEN` だけ。 |
| `.github/workflows/release-dry-run.yml` | 手動実行。タグなし・公開なしで `.dmg` を作り、7 日間だけ成果物として残す。 |
| `deny.toml` | 7 行目 `all-features = false`（コメント「実際にビルドする機能を検査する」）。脆弱性・unmaintained・取り下げ版は失敗、例外は RUSTSEC-2024-0370 だけ。ライセンスの許可リスト、取得元は crates.io のみ。ci.yml のコメントと `deny.toml` は同じコミット `b5e95d6`（#2）で入っており、最初から食い違っている。 |
| `.github/dependabot.yml` | cargo・npm・github-actions を毎週。 |
| `scripts/release/check-version.mjs`、`docs/releasing.md` | 版をそろえる対象は `Cargo.toml`・`Cargo.lock`・`package.json`（と `package-lock.json`）・`src-tauri/tauri.conf.json`。`docs/releasing.md:7`・`:65` は「下書きでなく公開」。 |
| `docs/install.md` | 会社の決まり（MDM、社内承認）の確認、`.dmg` の入れ方、Gatekeeper の警告の開き方、手での更新と戻し方。team.md の Deployment と合っている。 |
| `vitest.config.ts`、`package.json`、`package-lock.json` | カバレッジの設定はない。`@vitest/coverage-v8` などは vitest の任意の依存として lock に名前があるだけで、入れてはいない。Rust 側にも `cargo-llvm-cov` などの設定はない。 |
| 261012-parallel-fetch の ideation（`feasibility/constraint-register.md` の C-O5 ほか） | 速さの比較は CI ではなく本人の手元で行う、という制約。Walking Skeleton の「2 と 3 は本人の手元」と合っている。 |

推論：

- Way of Working は、git の履歴と GitHub のルールセット（PR 必須・スクワッシュのみ・CI 5 ジョブ必須・強制 push と削除の禁止）の両方で裏付けられた。文面は変えなくてよい。
- Code Style は、`unwrap()`／`expect()`・trait の境界・ツールの設定がすべて実態と合っている。secret scanning は有効（push protection も有効）。ずれは `cargo-deny` の機能の範囲のコメントだけ。
- Testing Posture の進め方（`custom`）は変える理由がない。ただし今回はスコープ `mvp` のため、org.md の 80% 行カバレッジ下限が加わる。team.md の「数値のカバレッジ下限は設けない。スコープごとの下限は org.md に従う」は、チームとしては下限を足さないという意味で、org.md の下限を外すことはできない（下の層は上の層と食い違えない）。測る仕組みがないので、どう満たすかを決める必要がある。
- Deployment は、配布の形・対象・署名なし・手での更新・戻し方が実装と手順書に合っている。ずれは「下書き」の一点と、版をそろえる対象の書き方。加えて、`main` にないコミットに付いた `v0.1.1` タグが残っている。ワークフローの `verify` が止めたため実害はないが、タグを付け直すか次の版をどうするかは人間の判断が要る。

## 実態とのずれ（証拠つき）

| ID | 基準（team.md）の記述 | 実態 | 証拠 | 面談 |
|----|------------------------|------|------|------|
| P1 | Deployment「GitHub Releases に下書きとして置く」 | 下書きにせずすぐ公開している（`v0.1.0` は `draft: false`） | `.github/workflows/release.yml:4`、`:138`（`--draft` なし）、`docs/releasing.md:7`・`:65`、GitHub API の Releases。261011-local-release の要件定義 Q5＝B・Q5-1＝A で本人が「すぐ公開し、記述を直す」と決めている | Q1 |
| P2 | Code Style「CI で `cargo-deny` による依存関係のチェック」（機能の範囲は書いていない） | CI のコメントは `all-features = true`、`deny.toml` は `false` | `.github/workflows/ci.yml:76`、`deny.toml:7`、どちらも `b5e95d6`（#2）から | Q3 |
| P3 | Testing Posture「数値のカバレッジ下限は設けない。スコープごとの下限は org.md に従う」 | 今回のスコープ `mvp` では org.md の 80% 行カバレッジ下限が当てはまるが、測る仕組みがリポジトリにない | `aidlc-state.md` の Scope、`org.md` の Testing Posture、`vitest.config.ts`、CI にカバレッジの手順なし | Q2 |
| P4 | Walking Skeleton「薄い一本は GUI を起動し GetLogEvents を呼んで結果を表示する極小版」 | その一本はアプリを最初に作ったとき（261002）のもので、すでにある。今回のスコープ `mvp` は `skeleton: on` のため、今回の最初の作業単位で何を一本とするかが文面から決まらない | `.claude/scopes/aidlc-mvp.md`（`skeleton: on`）、261002-cwlogs-viewer の記録、`crates/local-sights-core/examples/fetch_check.rs` | Q4 |
| P5 | Deployment「`Cargo.toml`・`package.json`・`src-tauri/tauri.conf.json` の版をそろえ」 | 検査は `Cargo.lock` と `package-lock.json` も含める。リリース前の「Release dry run」、タグで動くワークフローのテスト、CI のアプリ側の `cargo test -p local-sights` も記述にない | `scripts/release/check-version.mjs:2`、`docs/releasing.md` の Versions、`.github/workflows/release-dry-run.yml`、`release.yml` の `test`、`ci.yml:118` | Q5 |
| P6 | Deployment「リリースを区切るときは `main` 上のタグを使う」 | `v0.1.1` が `main` にないコミット（`f25a193`、`origin/sso-login`）に付いて残っている。版は `0.1.0` のまま。Release ワークフローは失敗し、Release は作られていない。タグを守る設定はない | `git ls-remote --tags origin`、`gh api .../rulesets`、Release ワークフローの実行履歴 | Q6 |

質問にしないもの（記録だけ）：

- マージ済みの `sso-login` ブランチがリモートに残っている（`delete_branch_on_merge: false`）。「長命のブランチを作らない」の文面に反するものではなく後片付けの話なので、面談では聞かない。人間が望めば別の作業で消す。
- `code-quality-assessment.md` の D1〜D3・D6〜D8 はコードの負債で、チームの慣行のずれではない。今回の intent の要件・設計で扱う。

## 面談で聞くこと（Step 4 の候補）

証拠で決まらない点だけを挙げる。各質問の候補の並びは下書きの案で、面談のときに見直す。どの質問も、決めるのは人間で、ここでは推奨を書かない（P1 だけは過去の本人の決定があることを示す）。

### Q1. リリースを下書きにするか、すぐ公開するか（P1）

- 証拠：ワークフローと手順書はすぐ公開、`v0.1.0` は公開済み。261011 の要件定義で本人が「すぐ公開し、team.md の記述を直す」と決めたが、記述は直っていない。
- 候補：A. すぐ公開する（今の実装どおり）。team.md の「下書きとして置く」を「下書きにせず公開して置く」に直す／B. 下書きに戻す（team.md はそのまま。`release.yml` と `docs/releasing.md` を直す作業を別に起こす）／X. Other (please specify)

### Q2. スコープ `mvp` の 80% 行カバレッジ下限をどう満たすか（P3）

- 証拠：org.md の決まりで `mvp` には 80% の行カバレッジ下限が加わり、team.md で外すことはできない。今はカバレッジを測る設定がない（Rust・画面とも）。
- 候補：A. CI で測り、80% を下回る PR はマージしない（例：ライブラリは `cargo-llvm-cov`、画面は Vitest のカバレッジ）／B. CI のゲートにはせず、Build and Test の段階で手元で測って結果を記録し、80% を確かめる／C. 測る範囲を GUI に依存しない層（ライブラリ側）と画面のロジックに限り、Tauri のアプリ層は外す（A か B と組み合わせる）／X. Other (please specify)
- 補足：どの答えでも、今回の作業の Build and Test で下限を確かめる。下限を下げる選択肢は置けない。

### Q3. `cargo-deny` が検査する機能の範囲（P2）

- 証拠：`deny.toml` は `all-features = false`（実際にビルドする機能だけを検査）、CI のコメントは `true` と書いている。どちらも最初のコミットから。
- 候補：A. `false` が意図どおり。CI のコメントを直す（team.md の文面は変えない）／B. `true` にして任意の機能も含めて検査する（`deny.toml` を変える。新しい違反が出るかもしれない）／X. Other (please specify)
- 補足：どちらに決めても、コードの直しは今回の作業か別の作業で行い、この段階ではファイルを変えない。

### Q4. すでに動くアプリに機能を足すときの「薄い一本」（P4）

- 「薄い一本」（walking skeleton）は、端から端まで動く最小の版を最初に作り、部品のつながりを確かめてから機能を足していくやり方。
- 証拠：今の文面の一本（GUI を起動して GetLogEvents の結果を表示）はアプリを最初に作ったときのもの。今回のスコープは薄い一本を作る設定（`skeleton: on`）で、最初の作業単位で何を一本とするかを決める必要がある。
- 候補：A. 書き方を一般化する。「各作業の最初の作業単位で、新しい振る舞いが画面から AWS まで通る最小の一本を作る（今回なら、例えば複数のストリームを同時に読んで表示するところまで）」とし、確認の 3 つ（`cargo test`、手元の確認用プログラム、GUI を目で見る）はそのまま／B. 今の文面のまま残し、今回の一本は作業単位を決める段階（units-generation）で決める／X. Other (please specify)

### Q5. Deployment の記述を今の仕組みに合わせるか（P5）

- 証拠：版をそろえる検査は `Cargo.lock` と `package-lock.json` も含む。タグの前に「Release dry run」で `.dmg` の作成だけを試せる。タグで動くワークフローはビルドの前にテストを流す。CI はアプリ側の `cargo test -p local-sights` も実行する。
- 候補：A. 書き足す（版をそろえる対象に `Cargo.lock` と `package-lock.json` を加え、dry run とリリース前のテストを 1 行で記す）／B. 今のまま変えない／X. Other (please specify)

### Q6. `main` にないコミットに付いたタグ `v0.1.1` の扱い（P6）

- 証拠：`v0.1.1` は `origin/sso-login` 上のコミットに付いていて、版は `0.1.0` のまま。Release ワークフローは止まり、Release はない。タグを守る設定はない。
- 候補：A. 慣行の文面はそのまま。`v0.1.1` は消し、次のリリースで `main` のコミットに付け直す（後片付けは別の作業）／B. A に加えて、GitHub のルールセットで `v*` タグの作成・移動を本人だけに絞るなど、タグを守る設定を入れることを慣行に書く／C. `v0.1.1` は残し、次のリリースは `v0.1.2` から付ける／X. Other (please specify)

## Sources

- Initial description: [desc] 「ロググループ全体の横断取得を高速化したい（IB-07、MVP 後の最優先）。今はストリームを 1 つずつ順に取得しているので、ストリームの多いロググループほど待ち時間が延びる。読み取り API 3 つ（DescribeLogGroups / DescribeLogStreams / GetLogEvents）だけで速くしたい。」
- Workflow-selected scope: [scope] `mvp`
- `aidlc/spaces/default/memory/team.md`、`project.md`、`org.md`、`phases/inception.md`
- 逆解析：`code-structure.md`、`technology-stack.md`、`dependencies.md`、`code-quality-assessment.md`、`architecture.md`、`business-overview.md`（`aidlc/spaces/default/codekb/local-sights/`）
- `aidlc/spaces/default/intents/261002-cwlogs-viewer/inception/practices-discovery/`、`aidlc/spaces/default/intents/261011-local-release/inception/practices-discovery/`、`aidlc/spaces/default/intents/261011-local-release/inception/requirements-analysis/`、`construction/code-generation/reviews/review-01.md`、`construction/build-and-test/build-and-test-summary.md`
- `aidlc/spaces/default/intents/261012-parallel-fetch/aidlc-state.md`、`ideation/feasibility/constraint-register.md`、`.claude/scopes/aidlc-mvp.md`（`skeleton:` の値だけ）
- `git log`、`git ls-remote --tags origin`、`git fetch --tags origin`、`git for-each-ref refs/tags`、`git rev-parse HEAD`
- GitHub REST API（読み取りのみ）：`repos/kdnakt/local-sights`、`/rulesets`、`/rules/branches/main`、`/releases`、Release ワークフローの実行一覧
- `.github/workflows/ci.yml`、`release.yml`、`release-dry-run.yml`、`.github/dependabot.yml`、`deny.toml`、`Cargo.toml`、`package.json`、`vitest.config.ts`、`scripts/release/check-version.mjs`、`docs/releasing.md`、`docs/install.md`

## Assumptions & Open Questions

- [assumption] org.md の `mvp` の「マージ前の CI での実行」は、テストを CI でマージ前に流すことを指すと読んだ。これは今の CI で満たしている。80% の下限そのものをどこで確かめるかは Q2 で決める。
- [assumption] `v0.1.1` の Release ワークフローの失敗は、`verify` のタグのコミットが `main` にない確認、または版の不一致で止まったものと考えた。実行のログは開いていない（実行時間 14 秒と、Release が存在しないことからの推論）。
- [assumption] GitHub のルールセットと secret scanning の状態は、下書きの時点（2026-10-12）に API で読んだもの。
- 未解決：Q1〜Q6（上の「面談で聞くこと」）。
