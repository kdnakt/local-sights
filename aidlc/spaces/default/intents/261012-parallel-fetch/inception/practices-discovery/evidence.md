# Evidence

協力エージェントなしの実行（collaborators: off）のため、調べたのはリード（aidlc-pipeline-deploy-agent）1 人で、寄稿ファイルはない。再実行なので、確定済みの `team.md` を基準にし、今の証拠と食い違う点（P1〜P6）だけを洗い出して面談（Q1〜Q6）で人間に決めてもらい、その答えで `team-practices.md` を仕上げた。

## 調べたもの・推論したこと（リード：aidlc-pipeline-deploy-agent）

| 対象 | 内容 |
|------|------|
| `aidlc/spaces/default/memory/team.md` | 5 セクションとも確定済み（261011-local-release で Deployment・Testing Posture・Code Style を更新）。再実行の基準として使った。 |
| `aidlc/spaces/default/memory/project.md` | `## Forbidden` の 4 件と `## Mandated` の `None.`（2 回）。`## Corrections` に、`aws sso login` の起動は Forbidden の対象外とする補足がある。今回の成果物はこれらに触れない。 |
| `aidlc/spaces/default/memory/org.md` | トランクベース・スクワッシュマージ。Testing Posture のスコープ別の下限：`mvp` は 80% の行カバレッジ下限と、マージ前の CI での実行を加える。 |
| `aidlc-state.md`（261012-parallel-fetch） | Brownfield、スコープ `mvp`（`skeleton: on`）、Test Strategy `Standard`、Operation はすべて SKIP、collaborators off。これまでの intent のスコープは `local-tool`・`bugfix`・`bugfix`・`refactor` で、`mvp` は今回が初めて。そのため org.md の 80% 下限が当てはまるのも今回が初めて。 |
| 261002-cwlogs-viewer と 261011-local-release の `inception/practices-discovery/` | 前回までの決定（Testing Posture の `custom`、Deployment の Q1〜Q5、GitHub 側の設定は有効という Q7 の回答）。 |
| 261011-local-release の `inception/requirements-analysis/requirements-analysis-questions.md` ほか | Q5 の回答 B（CI が最初から公開し下書きにしない）と Q5-1 の回答 A（「下書きとして置く」の記述は記録の手順で直す）。`requirements.md:124` が記述の直しを予告し、`construction/code-generation/reviews/review-01.md` の R-01 が直し忘れの防止を求め、`construction/build-and-test/build-and-test-summary.md:46` が「team.md は下書きのまま」と記録している。記述は今回まで直っていなかった。 |
| 逆解析の成果物：`code-structure.md`、`technology-stack.md`、`dependencies.md`、`code-quality-assessment.md`、`architecture.md`、`business-overview.md`（`aidlc/spaces/default/codekb/local-sights/`、コミット `9d08cbf`） | 中核とアプリの分離、trait の裏の AWS、テスト以外に `unwrap()`／`expect()` がないこと、秘密を出さないことはコード上で守られている（`code-quality-assessment.md` 総評、`code-structure.md:96`）。テストは中核 409・アプリ 12・画面とスクリプト 281。カバレッジの計測設定はない。`code-quality-assessment.md` の D4（CI のコメントと `deny.toml` の食い違い）と D5（リリースの下書き）を、下の「ずれ」で確かめ直した。`technology-stack.md` と `dependencies.md` から CI・Dependabot・`cargo-deny` の構成、`business-overview.md` と `architecture.md` から配布の流れと境界の置き方を確かめた。 |
| `git log`（`main` は 17 件の PR 由来のスクワッシュコミット、`9d08cbf` (#17) まで） | `main` のコミットはすべて `(#n)` 付き。直接 push の跡はない。今の作業ブランチは `claude/parallel-fetch`。下書きの時点の HEAD は `78bef39`、統合の時点は `7aa3146`（どちらも AI-DLC の記録だけの変更）。 |
| リモートのタグ（`git ls-remote --tags origin`、`git fetch --tags` で取得） | `v0.1.0` → `9d08cbf`（`main` の #17）。`v0.1.1` → `f25a193`（`origin/sso-login` 上の「AI-DLC: SSO login intent records up to approved code-generation plan」、`main` にはない。その時点の版は 3 か所とも `0.1.0`）。どちらも軽量タグ。なお `git fetch` で手元のタグとリモート追跡ブランチ（`origin/sso-login`）が増えたが、ファイルは変えていない。 |
| GitHub の設定（REST API で読み取りのみ） | `main` のルールセット（有効）：削除禁止、強制 push 禁止、PR 必須（承認 0 人、マージ方法はスクワッシュのみ）、必須の状態確認は CI の 5 ジョブ（`Rust library (fmt, clippy, test)`、`Front end (tsc, eslint, prettier, vitest, build)`、`cargo-deny`、`Tauri app (macos-latest)`、`Tauri app (ubuntu-latest)`）。secret scanning と push protection は有効、Dependabot のセキュリティ更新も有効。タグを守るルールセットはない。`delete_branch_on_merge` は false で、マージ済みの `sso-login` ブランチがリモートに残っている。Releases は `v0.1.0`（`draft: false`、`Local-Sights_0.1.0_aarch64.dmg` 1 つ）だけ。Release ワークフローの実行は `v0.1.0` が成功、`v0.1.1` が 14 秒で失敗（`verify` で止まったと考えられる）。 |
| `.github/workflows/ci.yml` | PR と `main` への push で実行。`RUSTFLAGS: ""`（21 行目、警告はエラーにしない）。`rust-core`（fmt・ライブラリの clippy・`cargo test -p local-sights-core`）、`frontend`（`npm run check:version`、`tsc --noEmit`、`eslint`、`prettier --check`、`vitest run`、`vite build`、`npm audit --audit-level=high`）、`cargo-deny`、`tauri-app`（Linux と macOS でワークスペースの clippy、ビルド、`cargo test -p local-sights`（118 行目、#17 で追加））。76 行目のコメントは「`deny.toml` の `[graph] all-features = true`」。カバレッジを測る手順はない。 |
| `.github/workflows/release.yml` | `v*` タグで実行。`verify`（`npm run check:version -- --tag`、タグのコミットが `main` にあるか：49・54 行目）→ `test`（ライブラリと画面のテスト）→ `build-and-publish`（Apple Silicon の確認、未署名の `.dmg`、1 つだけあるかの確認、`gh release create ... --verify-tag`（138 行目、`--draft` なし））。4 行目のコメントは「publishes it on GitHub Releases (not a draft)」。秘密は `GITHUB_TOKEN` だけ。 |
| `.github/workflows/release-dry-run.yml` | 手動実行。タグなし・公開なしで `.dmg` を作り、7 日間だけ成果物として残す。 |
| `deny.toml` | 7 行目 `all-features = false`（コメント「実際にビルドする機能を検査する」）。脆弱性・unmaintained・取り下げ版は失敗、例外は RUSTSEC-2024-0370 だけ。ライセンスの許可リスト、取得元は crates.io のみ。ci.yml のコメントと `deny.toml` は同じコミット `b5e95d6`（#2）で入っており、最初から食い違っている。 |
| `.github/dependabot.yml` | cargo・npm・github-actions を毎週。 |
| `scripts/release/check-version.mjs`、`docs/releasing.md` | 版をそろえる対象は `Cargo.toml`・`Cargo.lock`・`package.json`・`package-lock.json`・`src-tauri/tauri.conf.json`。`docs/releasing.md:7`・`:65` は「下書きでなく公開」。 |
| `docs/install.md` | 会社の決まり（MDM、社内承認）の確認、`.dmg` の入れ方、Gatekeeper の警告の開き方、手での更新と戻し方。team.md の Deployment と合っている。 |
| `vitest.config.ts`、`package.json`、`package-lock.json` | カバレッジの設定はない。`@vitest/coverage-v8` などは vitest の任意の依存として lock に名前があるだけで、入れてはいない。Rust 側にも `cargo-llvm-cov` などの設定はない。 |
| 261012-parallel-fetch の ideation（`feasibility/constraint-register.md` の C-O5 ほか） | 速さの比較は CI ではなく本人の手元で行う、という制約。Walking Skeleton の「2 と 3 は本人の手元」と合っている。 |

推論：

- Way of Working は、git の履歴と GitHub のルールセット（PR 必須・スクワッシュのみ・CI 5 ジョブ必須・強制 push と削除の禁止）の両方で裏付けられた。文面は変えていない。
- Code Style は、`unwrap()`／`expect()`・trait の境界・ツールの設定がすべて実態と合っている。secret scanning は有効（push protection も有効）。ずれは `cargo-deny` の機能の範囲についての CI のコメントだけで、設定（`all-features = false`）のほうが意図どおりと確かめた（Q3）。
- Testing Posture の進め方（`custom`）は変える理由がなく、面談でも変わらなかった。今回はスコープ `mvp` のため org.md の 80% 行カバレッジ下限が加わり、面談でそれを CI の合否で確かめることに決めた（Q2）。下限は下げていない。
- Deployment は、配布の形・対象・署名なし・手での更新・戻し方が実装と手順書に合っている。「下書き」の一点、版をそろえる対象、dry run とリリース前のテスト、`v0.1.1` のタグの扱いを面談で決めた（Q1・Q5・Q6）。

## 面談で決めたこと（Q1〜Q6）

面談ファイル：`practices-discovery-questions.md`。まとめの確認には人間が「Looks correct」と答えた。

| 質問 | 回答 | 決めたこと | `team-practices.md` への反映 |
|------|------|------------|------------------------------|
| Q1. リリースを下書きにするか、すぐ公開するか | A | すぐ公開する（今の仕組みどおり） | Deployment の「下書きとして置く」を「下書きにせず公開して置く」に直した |
| Q2. 80% の行カバレッジ下限をどう満たすか | A | CI で測り、80% を下回る PR はマージしない（例：ライブラリは `cargo-llvm-cov`、画面は Vitest のカバレッジ） | Testing Posture の「数値のカバレッジ下限は設けない」を、CI で測って 80% 未満はマージしない決まりに置き換えた。Code Style にも CI の合否に入ることを 1 行加えた |
| Q3. `cargo-deny` が検査する機能の範囲 | A | 「実際にビルドする機能だけ」（`all-features = false`）が意図どおり。直すのは CI のコメント | 決まりの中身は変えていない。Code Style の 1 行に「実際にビルドする機能を対象に」と範囲を書き添えた |
| Q4. 既存のアプリに機能を足すときの薄い一本 | A | 書き方を一般化する。各作業の最初の作業単位で、新しい振る舞いが画面から AWS まで通る最小の一本を作る（今回なら、例えば複数のストリームを同時に読んで表示するところまで）。確認の 3 つはそのまま | Walking Skeleton の冒頭と最初の箇条を一般化した。確認の 3 つ、2 と 3 は本人の手元、CI は実際の AWS に接続しない、チェックポイントの人間の承認は残した |
| Q5. 配布の記述を今の仕組みに合わせるか | A | 書き足す | 版をそろえる対象に `Cargo.lock` と `package-lock.json` を加え、タグで動くリリースがビルド前にテストを流すこと、手動の「Release dry run」で `.dmg` の作成を試せることを書いた |
| Q6. `main` にないコミットに付いたタグ `v0.1.1` | C | `v0.1.1` は残し、次のリリースは `v0.1.2` から付ける | Deployment に「次の版のタグは `v0.1.2`。`v0.1.1` は使い回さない」の 1 行を加えた。「`main` 上のタグを使う」の文面はそのまま |

## 実態とのずれと、その決着

| ID | 基準（team.md）の記述 | 実態 | 証拠 | 決着 |
|----|------------------------|------|------|------|
| P1 | Deployment「GitHub Releases に下書きとして置く」 | 下書きにせずすぐ公開している（`v0.1.0` は `draft: false`） | `.github/workflows/release.yml:4`、`:138`（`--draft` なし）、`docs/releasing.md:7`・`:65`、GitHub API の Releases。261011-local-release の要件定義 Q5＝B・Q5-1＝A | Q1＝A。記述を実態に合わせた。仕組みは変えない |
| P2 | Code Style「CI で `cargo-deny` による依存関係のチェック」（機能の範囲は書いていない） | CI のコメントは `all-features = true`、`deny.toml` は `false` | `.github/workflows/ci.yml:76`、`deny.toml:7`、どちらも `b5e95d6`（#2）から | Q3＝A。設定が正しく、CI のコメントの直しは後の作業（下の「後の作業」） |
| P3 | Testing Posture「数値のカバレッジ下限は設けない。スコープごとの下限は org.md に従う」 | 今回のスコープ `mvp` では org.md の 80% 行カバレッジ下限が当てはまるが、測る仕組みがリポジトリにない | `aidlc-state.md` の Scope、`org.md` の Testing Posture、`vitest.config.ts`、CI にカバレッジの手順なし | Q2＝A。CI で測って 80% 未満はマージしない決まりにした。CI への組み込みは後の作業 |
| P4 | Walking Skeleton「薄い一本は GUI を起動し GetLogEvents を呼んで結果を表示する極小版」 | その一本はアプリを最初に作ったとき（261002）のもので、すでにある。今回のスコープ `mvp` は `skeleton: on` | `.claude/scopes/aidlc-mvp.md`（`skeleton: on`）、261002-cwlogs-viewer の記録、`crates/local-sights-core/examples/fetch_check.rs` | Q4＝A。書き方を一般化した |
| P5 | Deployment「`Cargo.toml`・`package.json`・`src-tauri/tauri.conf.json` の版をそろえ」 | 検査は `Cargo.lock` と `package-lock.json` も含める。「Release dry run」とリリース前のテストも記述にない | `scripts/release/check-version.mjs:2`、`docs/releasing.md` の Versions、`.github/workflows/release-dry-run.yml`、`release.yml` の `test` | Q5＝A。記述を書き足した |
| P6 | Deployment「リリースを区切るときは `main` 上のタグを使う」 | `v0.1.1` が `main` にないコミット（`f25a193`、`origin/sso-login`）に付いて残っている。Release は作られていない | `git ls-remote --tags origin`、`gh api .../rulesets`、Release ワークフローの実行履歴 | Q6＝C。タグは残し、次は `v0.1.2` から。後片付けの作業はない |

質問にしなかったもの（記録だけ）：

- マージ済みの `sso-login` ブランチがリモートに残っている（`delete_branch_on_merge: false`）。「長命のブランチを作らない」の文面に反するものではなく後片付けの話なので、面談では聞いていない。人間が望めば別の作業で消す。
- `code-quality-assessment.md` の D1〜D3・D6〜D8 はコードの負債で、チームの慣行のずれではない。今回の intent の要件・設計で扱う。

## 後の作業

この段階ではリポジトリのファイルを変えていない。決まりに合わせるための作業は次のとおり。

1. `.github/workflows/ci.yml:76` のコメントを、`deny.toml` の `all-features = false`（実際にビルドする機能だけを検査する）に合わせて直す（Q3）。
2. 行カバレッジの計測を CI に組み込み、80% を下回る PR を止める（例：ライブラリは `cargo-llvm-cov`、画面は Vitest のカバレッジ。必須の状態確認にも加える）（Q2）。今回の intent では、Build and Test で 80% の下限を確かめる。
3. タグ `v0.1.1` については何もしない（Q6）。次のリリースのタグを `v0.1.2` にするだけ。

## 残っている不確かさ

- カバレッジを測る範囲（Tauri のアプリ層を含めるか、ライブラリと画面のどこまでを数えるか）と、Rust と画面を別々に 80% とするか合算するかは決めていない。面談では Q2 の C（範囲を絞る）は選ばれなかったが、計測を CI に組み込むときに具体的な設定を決める。
- CI への組み込みが済むまでは、CI の合否でカバレッジを確かめることはできない。その間は今回の Build and Test で下限を確かめる。

## Sources

- Initial description: [desc] 「ロググループ全体の横断取得を高速化したい（IB-07、MVP 後の最優先）。今はストリームを 1 つずつ順に取得しているので、ストリームの多いロググループほど待ち時間が延びる。読み取り API 3 つ（DescribeLogGroups / DescribeLogStreams / GetLogEvents）だけで速くしたい。」
- Workflow-selected scope: [scope] `mvp`
- 面談：`practices-discovery-questions.md` の Q1〜Q6 の回答とまとめの確認 [Q1] [Q2] [Q3] [Q4] [Q5] [Q6]
- `aidlc/spaces/default/memory/team.md`、`project.md`、`org.md`、`phases/inception.md`
- 逆解析：`code-structure.md`、`technology-stack.md`、`dependencies.md`、`code-quality-assessment.md`、`architecture.md`、`business-overview.md`（`aidlc/spaces/default/codekb/local-sights/`）
- `aidlc/spaces/default/intents/261002-cwlogs-viewer/inception/practices-discovery/`、`aidlc/spaces/default/intents/261011-local-release/inception/practices-discovery/`、`aidlc/spaces/default/intents/261011-local-release/inception/requirements-analysis/`、`construction/code-generation/reviews/review-01.md`、`construction/build-and-test/build-and-test-summary.md`
- `aidlc/spaces/default/intents/261012-parallel-fetch/aidlc-state.md`、`ideation/feasibility/constraint-register.md`、`.claude/scopes/aidlc-mvp.md`（`skeleton:` の値だけ）
- `git log`、`git ls-remote --tags origin`、`git fetch --tags origin`、`git for-each-ref refs/tags`、`git rev-parse HEAD`
- GitHub REST API（読み取りのみ）：`repos/kdnakt/local-sights`、`/rulesets`、`/rules/branches/main`、`/releases`、Release ワークフローの実行一覧
- `.github/workflows/ci.yml`、`release.yml`、`release-dry-run.yml`、`.github/dependabot.yml`、`deny.toml`、`Cargo.toml`、`package.json`、`vitest.config.ts`、`scripts/release/check-version.mjs`、`docs/releasing.md`、`docs/install.md`

## Assumptions & Open Questions

- [assumption] org.md の `mvp` の「マージ前の CI での実行」は、テストを CI でマージ前に流すことを指すと読んだ。これは今の CI で満たしている。80% の下限そのものは Q2 の回答で CI の合否で確かめることにした。
- [assumption] `v0.1.1` の Release ワークフローの失敗は、`verify` のタグのコミットが `main` にない確認、または版の不一致で止まったものと考えた。実行のログは開いていない（実行時間 14 秒と、Release が存在しないことからの推論）。Q6＝C でタグは残すため、決定には影響しない。
- [assumption] GitHub のルールセットと secret scanning の状態は、下書きの時点（2026-10-12）に API で読んだもの。
