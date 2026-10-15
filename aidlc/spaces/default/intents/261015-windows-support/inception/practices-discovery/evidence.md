# Evidence

協力エージェントなしの実行（collaborators: off）のため、調べたのはリード（aidlc-pipeline-deploy-agent）1 人で、寄稿ファイルはない。再実行なので、確定済みの `team.md` を基準にし、Windows 対応（IB-16）で変わる点と、今の証拠と食い違う点だけを洗い出した。下の「提案する変更」（C1〜C12）は下書きの案で、面談（`practices-discovery-questions.md` の Q1〜Q8、すべて回答済み）の答えで確定させた。答えは Q5 を除いてリードの推奨（A）どおりで、Q5 だけが下書きと違った（下の「面談の決定」と「下書きから変えたこと」）。

## 調べたもの・推論したこと（リード：aidlc-pipeline-deploy-agent）

| 対象 | 内容 |
|------|------|
| `aidlc/spaces/default/memory/team.md` | 5 セクションとも確定済み（261012-parallel-fetch で更新）。再実行の基準として使った。Deployment に「配布の対象：macOS（Apple Silicon）だけ。Intel の Mac と Windows には配らない」とあり、今回の作業と食い違う。 |
| `aidlc/spaces/default/memory/project.md` | `## Forbidden` の 4 件と `## Mandated` の `None.`（3 回）。`## Corrections` に、`aws sso login` の起動は Forbidden の対象外とする補足、CI のジョブ名を変えたら必須チェックの名前も合わせる決まり（261012-parallel-fetch）、「macOS 版と同じ」の解釈（261015-windows-support）がある。今回の成果物はこれらを重ねて書かない。 |
| `aidlc/spaces/default/memory/org.md` | トランクベース・スクワッシュマージ。Testing Posture のスコープ別の下限：`bugfix` は対象の不具合への回帰テストと既存のテストが通ることを加える。Deployment の既定（ステージング・本番）は、team.md のとおり適用する環境がない。 |
| `aidlc-state.md`（261015-windows-support） | Brownfield、スコープ `bugfix`（計画名 `platform-port`）、Test Strategy `Standard`、reverse-engineering は SKIP、Operation はすべて SKIP、collaborators off。Construction は code-generation・build-and-test・ci-pipeline。 |
| intent-capture の `intent-statement.md` と `intent-capture-questions.md` | Windows 11（x64）だけ（Q5）、GitHub Releases でインストーラを配る（Q3）、macOS 版と同じ機能すべて（Q3＝C）、手元に AWS 認証情報の使える Windows の PC がある（Q4＝A）、コード署名・自動更新・Intel の Mac と Linux・Microsoft Store は含めない（Q6）、会社の Windows の PC に署名のないアプリを入れてよい（Q8＝A）、進め方は本人が決める（Q7＝A）。 |
| 261012-parallel-fetch の `inception/practices-discovery/` | 前回の書き方と決定（Q1〜Q6）。Testing Posture の `custom`、カバレッジ 80% を CI で確かめる決まり、`v0.1.1` の扱い（Q6＝C）。 |
| 261012-adhoc-signing の `operation/deployment-execution/deployment-log.md` | `v0.1.1` のタグは最初 `main` にない古いコミットに付いて失敗し（Release #2）、その後 `main` のコミットに付け直して公開された。アドホック署名（`APPLE_SIGNING_IDENTITY: "-"`）を入れた経緯もここにある。 |
| 逆解析の成果物：`code-structure.md`、`technology-stack.md`、`dependencies.md`、`code-quality-assessment.md`、`architecture.md`、`business-overview.md`（`aidlc/spaces/default/codekb/local-sights/`、走査したコミット `9d08cbf`） | 中核（GUI に依存しないライブラリ）とアプリの分離、AWS は `CloudWatchLogsGateway` trait の裏、判断は純粋関数、という構成（`architecture.md`、`code-structure.md:90`、`dependencies.md`）は OS に依存しない。OS に触れるのは `sso_cli.rs`（AWS CLI の起動と停止、`code-structure.md:62`）と `cache`（ファイルと権限、`code-structure.md:42`・`:44`）。`technology-stack.md` と `business-overview.md` は対象 OS を macOS（Apple Silicon）、配布物を「未署名の `.dmg`」と書いており、#18 以降のアドホック署名と食い違う（走査が #18 より前のため）。`code-quality-assessment.md:30` は ワークスペース全体の clippy を Linux と macOS で行うと記録している。今回は reverse-engineering が SKIP なので、知識ベースは古いまま使った。 |
| `git log`、`git branch -a`、`git tag`、`git ls-remote --tags origin` | `origin/main` の先頭は `f09f448`（#22、取得の高速化）。下書きの時点では `claude/parallel-fetch` を未マージと書いたが誤りで、PR #22 としてすでに `main` にスクワッシュマージされている（`f09f448`）。`main` のコミットはすべて PR 由来のスクワッシュで、直接 push の跡はない。手元の作業ブランチは `claude/parallel-fetch`（HEAD は `6218363`。この intent の記録が載る）。リモートのタグは `v0.1.0` → `9d08cbf`（#17）、`v0.1.1` → `621f0f0`（#19、`main` 上）。手元の `v0.1.1` は古いコミット（`origin/sso-login` 上）を指したままで、リモートと違う。 |
| GitHub の設定（REST API で読み取りのみ） | `main` のルールセット：削除禁止、強制 push 禁止、PR 必須（承認 0 人、マージ方法はスクワッシュのみ）、必須の状態確認は CI の 5 ジョブ（`Rust library (fmt, clippy, test)`、`Front end (tsc, eslint, prettier, vitest, build)`、`cargo-deny`、`Tauri app (macos-latest)`、`Tauri app (ubuntu-latest)`）。Releases は `v0.1.0`（`Local-Sights_0.1.0_aarch64.dmg`）と `v0.1.1`（`Local-Sights_0.1.1_aarch64.dmg`）で、どちらも `draft: false`。 |
| `.github/workflows/ci.yml` | PR と `main` への push で実行。`rust-core`（Linux：fmt、ライブラリの clippy、`cargo llvm-cov -p local-sights-core --fail-under-lines 80`、doc と example のテスト）、`frontend`（Linux：版の一致、`tsc --noEmit`、ESLint、Prettier、`npm run test:coverage`、`vite build`、`npm audit`）、`cargo-deny`、`tauri-app`（`ubuntu-latest` と `macos-latest` の matrix：ワークスペースの clippy、アプリのビルド、`cargo test -p local-sights`）。Windows のランナーはない。ジョブ名は matrix の OS 名を含むため、Windows を足すと新しい名前の状態確認が増える。 |
| `.github/workflows/release.yml` | `v*` タグで実行。`verify`（タグの形と版の一致、タグのコミットが `main` にあるか）→ `test`（Linux：ライブラリのテストと Vitest）→ `build-and-publish`（macOS：Apple Silicon の確認、`APPLE_SIGNING_IDENTITY: "-"` でアドホック署名した `.dmg`、`aarch64` の `.dmg` が 1 つだけあるか、公開する `.dmg` の中の `.app` の署名の検証、`gh release create`／再実行時は `upload --clobber`）。ビルドと公開が 1 つのジョブにあり、Windows の配布物を同じ Release に載せる仕組みはない。 |
| `.github/workflows/release-dry-run.yml` | 手動実行。macOS のランナーで `.dmg` だけを作り、7 日間成果物として残す。Windows はない。 |
| `src-tauri/tauri.conf.json` | 版 `0.1.1`。`bundle.targets: "all"`、アイコンに `icon.ico` がすでにある。`bundle.windows` の設定（NSIS／WiX、WebView2 の入れ方、利用者ごとの導入）はない。 |
| `Cargo.toml`、`package.json`、`src-tauri/Cargo.toml` | 版はどれも `0.1.1`（`npm run check:version` で一致を確かめる）。アプリ側は `libc`（`killpg` 用）に依存する。 |
| `src-tauri/src/sso_cli.rs` | 「Unix only」と明記。CLI を探す場所は `/opt/homebrew/bin/aws` と `/usr/local/bin/aws` だけ。停止は `libc::killpg` によるプロセスグループへのシグナル、テストは `/bin/sh` の偽の CLI と `/proc` を使う。Windows ではこのままビルドもテストもできない。 |
| `src-tauri/src/lib.rs` | `sso_cli` を無条件に読み込む（106 行目）。アプリのメニューは `cfg(target_os = "macos")` だけ（1453 行目）。 |
| `crates/local-sights-core/src/cache/mod.rs`、`cache/settings.rs` | 所有者だけが読めるフォルダとファイル（`0o700`／`0o600`）は `cfg(unix)` のときだけ設定し、Unix 以外では何もしない（674 行目）。テストの一部も `cfg(unix)`。 |
| `docs/install.md`、`docs/releasing.md`、`README.md` | install.md は「Intel Macs and Windows are not supported」と書き、手順・設定とキャッシュの場所（`~/Library/...`）・更新・戻し方は macOS だけ。releasing.md は Apple Silicon の `.dmg` だけ。README の手動の GUI 確認は macOS の操作（Cmd+Q、Dock、VoiceOver、Activity Monitor）を前提にした項目が多い。AWS CLI を探す場所も macOS の 2 か所だけ。 |
| `scripts/release/versions.ts`、`signing.ts` | 版の一致はファイル 4 種（`package-lock.json` は `npm version` でそろえる）で確かめる。`signing.ts` は release.yml の macOS の署名の手順の順序と、文書の「unsigned」という表現を Vitest で検査する。Windows の手順を足すときは、この検査が macOS の手順を正しく見つけ続けるかの確認が要る。 |
| `deny.toml` | `all-features = false`、対象のプラットフォーム（`[graph] targets`）の指定はないため、Windows 向けの依存（`windows-sys` など）もすでに検査の対象に入っている。例外は RUSTSEC-2024-0370（Linux の GTK 経由）だけ。 |
| `.prettierrc.json`、リポジトリの直下 | Prettier の `endOfLine` は既定（`lf`）。`.gitattributes` はない。Windows で `core.autocrlf=true` のまま取り出すと、`prettier --check` が改行の違いで失敗しうる。 |

推論：

- Way of Working は、git の履歴と GitHub のルールセットで裏付けられ、Windows 対応で変える理由はない。ただし CI に Windows のランナーを足すと、必須の状態確認に新しい名前を加えない限り、Windows のジョブが赤でもマージできてしまう。project.md の「名前を変えるときは合わせる」は名前の変更だけを扱うので、「足すとき」も含める 1 行を提案する（C1）。
- Walking Skeleton は、確認の 2 と 3 が「本人の手元」としか書いておらず、どの OS の機械かが決まっていない。今回は Windows の PC で行うことが intent（Q4）で決まっている。macOS を置き換えるのか加えるのかは面談で決める（C2・C3）。
- Testing Posture の進め方（`custom`）は変える理由がない。OS ごとの境界の層（`sso_cli` と `cache` の権限）が増えるので、「先に実装してからテスト」の層にそれを加える（C4）。今の CI は Windows でテストを流しておらず、`sso_cli` のテストは Unix 専用なので、Windows のランナーでのテストと OS ごとの偽の CLI が要る（C5・C6）。
- Deployment は、配布の対象・形・作り方・dry run・署名・更新・戻し方・手順書のすべてに Windows が関わる（C7〜C11）。加えて、今の証拠と食い違う点が 2 つある（下の「実態とのずれ」の P1・P2）。
- Code Style は、決まりそのものは OS に依存しない。Windows で取り出したときの改行コードと、OS ごとに分かれるコードの clippy を Windows でも流すことを加える（C12）。

## 提案する変更（team.md との違い）

| ID | セクション | 今の team.md | 提案（`team-practices.md` の記述） | 根拠 | 面談の問い |
|----|------------|--------------|--------------------------------------|------|------------|
| C1 | Way of Working | （なし） | CI のジョブを足す・名前を変えるときは、同じ PR でルールセットの必須の状態確認も合わせて直す | `ci.yml` の `tauri-app` の matrix のジョブ名、ルールセットの必須 5 件、project.md の 261012 の学び | Q2 |
| C2 | Walking Skeleton | 薄い一本の例は 261012 だけ | 例に「Windows 11 の PC にインストーラで入れて起動し、取得して表示する」を加える | intent の Q3・Q4 | Q3 |
| C3 | Walking Skeleton | 2 と 3 は「本人の手元」 | 対象の OS の機械で行う。両方の OS に効く変更は両方で行う。Windows では CI で作ったインストーラで入れたアプリを使う。確認の 1 は CI の各 OS のランナーでも通ること | intent の Q4、`sso_cli.rs`・`cache` が共通のコードを触る | Q3 |
| C4 | Testing Posture | Ordering の「先に実装」は AWS 接続と GUI の層 | OS ごとに分かれる境界の層（プロセスの起動と停止、ファイルの置き場所と権限）を加える。Methodology は `custom` のまま | `sso_cli.rs`、`cache/mod.rs:644-677` | なし（確認のみ） |
| C5 | Testing Posture | Rust のテストは CI でマージ前に実行（OS は書いていない） | Rust のテストは Linux・macOS・Windows の各ランナーで実行。カバレッジは Linux だけで測る。Vitest は Linux だけ | `ci.yml`（Windows なし）、`rust-core` は Linux だけ | Q2 |
| C6 | Testing Posture | AWS はモック／スタブ | AWS CLI も偽物に差し替える。OS ごとに分かれるコードのテストはその OS のランナーで実行され、偽の CLI も OS ごとに用意する | `sso_cli.rs` のテストが `/bin/sh` と `/proc` に依存 | Q2 |
| C7 | Deployment | 対象は macOS（Apple Silicon）だけ。Windows には配らない | macOS（Apple Silicon）と、Microsoft がサポートしている版の Windows 11（x64）（開発者の PC で確かめ、ビルド番号は書かない）。Intel の Mac、Linux、Windows 10、arm64 の Windows、Microsoft Store には配らない | intent の Q5・Q6 | Q6（最低の版） |
| C8 | Deployment | `.dmg` だけ | Windows は NSIS のインストーラ（`-setup.exe`、利用者ごとに管理者の権限なしで入れられる） | `tauri.conf.json` の `bundle`、`docs/install.md` の「管理者の権限がないとき」の手順 | Q1 |
| C9 | Deployment | タグで macOS のランナーが作って公開 | 1 つのタグで macOS と Windows の両方を作り、両方そろってから 1 つの Release に公開。dry run も両方 | `release.yml`（作ると公開が 1 ジョブ）、`release-dry-run.yml` | Q4 |
| C10 | Deployment | 「今は Apple の署名・公証をしない」 | Developer ID の署名と公証はしない。macOS は CI でアドホック署名だけ。Windows もコード署名なしで、SmartScreen の開き方を手順書に書く | #18・#19、`release.yml`、`docs/install.md`、intent の Q6・Q8 | なし（P1 の記述の直し） |
| C11 | Deployment | 「次の版のタグは `v0.1.2`。`v0.1.1` は `main` にないコミットに残ったタグで、使い回さない」 | 次の版のタグは `v0.2.0`（取得の高速化 #22 と Windows 版を合わせて配り、`v0.1.2` は付けない）。公開済みの `v0.1.0`・`v0.1.1`（どちらも `main` 上）を含め、一度公開したタグは付け替えも使い回しもしない。更新・戻し方・手順書（`docs/install.md` に OS ごとの節）を Windows にも広げる（インストーラの上書き、設定とキャッシュは `AppData` の下、AWS CLI の入れ方と探す場所を OS ごとに） | `git ls-remote --tags origin`、REST API の Releases、261012-adhoc-signing の deployment-log | Q5、Q7 |
| C12 | Code Style | 改行コードの決まりはない。ワークスペースの clippy は OS を書いていない | テキストは LF にそろえ `.gitattributes` で固定。ワークスペースの clippy とビルドは Linux・macOS・Windows で行う。OS ごとの処理は `cfg` で分けた境界のコードに閉じ込める | `.prettierrc.json`（`endOfLine` は既定の `lf`）、`.gitattributes` なし、`code-quality-assessment.md:30` | Q8 |

変えていないもの：Way of Working の本文（C1 の 1 行を足すだけ）、Testing Posture の Methodology・カバレッジ 80%・ライブラリの公開関数のテスト・実際の AWS に接続しないこと、Deployment の版をそろえる 5 ファイル・自動更新なし・サーバーがないこと・`main` 上のタグ、Code Style のフォーマッタ・リンタ・型検査・命名・`unwrap()`／`expect()`・依存関係と秘密情報。

## 実態とのずれ

| ID | 基準（team.md）の記述 | 実態 | 証拠 | 扱い |
|----|------------------------|------|------|------|
| P1 | Deployment「今は Apple の署名・公証をしない」 | 261012-adhoc-signing（#18、#19）で、`.app` を CI でアドホック署名し、公開前に署名を検証するようになった。Developer ID の署名と公証はしていない | `release.yml:16-19`・`:106-167`、`docs/releasing.md:9-14`、`docs/install.md:21-24` | 決まりの中身（Apple の証明書を使わない）とは食い違わないため、記述を実態に合わせて直す案にした（C10）。面談では確認だけ |
| P2 | Deployment「次の版のタグは `v0.1.2`。`v0.1.1` は `main` にないコミットに残ったタグで、使い回さない」 | `v0.1.1` はその後 `main` のコミット `621f0f0`（#19）に付け直され、`Local-Sights_0.1.1_aarch64.dmg` で公開済み。今の版のファイルも `0.1.1` | `git ls-remote --tags origin`、REST API の Releases（`v0.1.1`、`draft: false`、2026-10-12T12:26:49Z）、261012-adhoc-signing の deployment-log | 記述が古い。面談の Q5（回答 C）で、次の版を `v0.2.0` とし `v0.1.2` は付けないと決めたため、古い記述をこの事実に置き換えた |
| P3 | （逆解析の知識ベース）`technology-stack.md`・`business-overview.md` の「未署名の `.dmg`」 | アドホック署名済み | 走査したコミット `9d08cbf` は #18 より前 | 決まりではないため面談では聞かない。今回は reverse-engineering が SKIP なので、知識ベースは次に走査するときに直る |
| P4 | （手元のリポジトリ）手元のタグ `v0.1.1` | 古いコミットを指したまま。リモートとは違う | `git tag -l -n1` と `git ls-remote --tags origin` | 決まりではない。リリースの作業で `git fetch --tags --force` などで合わせる必要がある、とだけ記録する |

## 後の段階で扱うもの（慣行ではなく要件・設計の話）

面談では聞かない。requirements-analysis 以降で扱う。

1. `sso_cli.rs` は Unix 専用。Windows では AWS CLI の探し方（例：`C:\Program Files\Amazon\AWSCLIV2\aws.exe`、`PATH` を使うかどうか）、子プロセスごと止める方法、テストの偽の CLI を Windows 向けに作り直す必要がある。
2. キャッシュと設定の「所有者だけ」の扱いは Unix だけで、Windows では今は何もしていない（`cache/mod.rs:674`）。Windows の既定のアクセス権（利用者の `AppData` の下）で足りるかを決める。
3. Windows での設定とキャッシュの置き場所（Tauri の既定では `%APPDATA%\dev.local-sights.app` と `%LOCALAPPDATA%\dev.local-sights.app`）を手順書と README に書く。
4. README の手動の GUI 確認は macOS の操作を前提にした項目が多い（Cmd+Q、Dock、VoiceOver、Activity Monitor）。Windows 版の確認項目（Alt+F4、タスクバー、ナレーター、タスク マネージャーなど）をどう用意するか。
5. アプリのメニュー（編集のショートカットなど）は macOS だけ（`lib.rs:1453`）。Windows でのコピーと貼り付けの扱いは、project.md の「見た目の細部を OS の慣習に合わせてよいかは要件分析で決める」に従う。
6. `scripts/release/signing.ts` の検査が、Windows の手順を足した release.yml でも macOS の手順を正しく見つけるか。
7. WebView2：Windows 11 には最初から入っているので、インストーラにどう含めるか（既定のダウンロード方式のままか）。

## 面談の決定

`practices-discovery-questions.md` の 8 問すべてに回答があった（リード 1 人の実行で、寄稿ファイルはない）。

| 問い | 回答 | 決まったこと | 反映した箇所 |
|------|------|--------------|--------------|
| Q1. Windows のインストーラの形 | A | NSIS の `-setup.exe`（利用者ごとに入れ、管理者の権限は要らない） | Deployment「配布の形」（C8） |
| Q2. CI で Windows をどこまで確かめるか | A | Tauri アプリのジョブに `windows-latest` を足し（clippy・ビルド・アプリのテスト）、ライブラリのテストも Windows で流し、マージに必須のチェックにする | Way of Working（必須チェックを同じ PR で直す）、Testing Posture（各 OS のランナー）、Code Style（clippy とビルドを各 OS で）（C1・C5・C6・C12） |
| Q3. 薄い一本の手元の確認の OS | A | その作業が対象にする OS で行い、両方の OS に効く変更や OS ごとに分かれるコードの変更なら両方で行う | Walking Skeleton（C2・C3） |
| Q4. リリースのワークフローの形 | A | 1 つのタグで両方の OS を作り、両方が成功したときだけ 1 つの Release として公開する。Release dry run も両方を作る | Deployment「作る場所と置き場所」「リリース前の確認」（C9） |
| Q5. Windows 版を初めて配るときの版 | C | `v0.1.2` は付けず、マージ済みの取得の高速化（PR #22）と Windows 版を合わせて `v0.2.0` で配る | Deployment の版の行（C11、P2） |
| Q6. 対象にする Windows 11 の最低の版 | A | Microsoft がサポートしている Windows 11（x64）とし、開発者の PC で確かめる | Deployment「配布の対象」（C7） |
| Q7. 手順書の置き場所と AWS CLI の書き方 | A | `docs/install.md` に macOS と Windows の節を分けて書き、AWS CLI v2 の入れ方、OS ごとに AWS CLI を探す場所、SmartScreen の「詳細情報 → 実行」を含める | Deployment の手順書の行（C11） |
| Q8. 改行コード | A | `.gitattributes` で LF に固定する | Code Style「改行コード」（C12） |

## 下書きから変えたこと

- **Q5（下書きと違う答え）**：下書きは「公開済みの版は `v0.1.0` と `v0.1.1`。一度公開したタグは付け替えず、次の版のタグは `v0.1.1` より大きい番号にする」と、番号を決めない書き方にしていた。答えが C だったため、「次の版のタグは `v0.2.0` とする。マージ済みの取得の高速化（PR #22）と Windows 版を合わせてこの版で配り、`v0.1.2` は付けない。公開済みの `v0.1.0` と `v0.1.1`（どちらも `main` 上のコミット）を含め、一度公開したタグは付け替えも使い回しもしない」に直した。これで team.md の古い記述（「次の版のタグは `v0.1.2`。`v0.1.1` は `main` にないコミットに残ったタグで、使い回さない」）を今の事実で置き換える。リモートで確かめた事実：`v0.1.0` → `9d08cbf`、`v0.1.1` → `621f0f0`（`main` 上）、`v0.1.2` はない。
- **Q6**：推奨どおりだが、答えの文面（Microsoft がサポートしている版、開発者の PC で確かめる）を「配布の対象」の行に明記した。
- **Q7**：推奨どおりだが、手順書の置き場所（`docs/install.md`、OS ごとの節）と SmartScreen の開き方を手順書の行に明記した。
- **Testing Posture の Methodology evidence**：261015-windows-support の面談でも進め方を変える回答がなかったこと（Q2 はテストを流す OS を足すだけ）を加えた。Methodology（`custom`）と Ordering は下書きのまま。
- **証拠の訂正**：下書きで `claude/parallel-fetch` を未マージと書いたのは誤りで、PR #22 として `main` に `f09f448` でスクワッシュマージ済み。上の表の git の行を直した。

### 面談の問いと推奨（下書きの時点の記録）

1. **Q1. Windows のインストーラの形**（C8）：A. NSIS（`-setup.exe`、利用者ごとに管理者の権限なしで入れられる）／B. MSI（WiX、ふつうは管理者の権限が要る）／C. 両方。推奨は A。macOS で「管理者の権限がないとき」の手順を用意しているのと同じく、会社の PC で権限なしで入れられるため。
2. **Q2. CI で Windows をどこまで確かめるか**（C1・C5・C6）：A. `tauri-app` の matrix に `windows-latest` を足し、ワークスペースの clippy・アプリのビルド・アプリのテストに加えてライブラリのテストも Windows で流し、必須の状態確認に加える／B. A と同じだが必須にはしない（参考の結果だけ）／C. Windows は Release と dry run でビルドするだけ。推奨は A。`sso_cli` とキャッシュの権限は Windows で別のコードになり、Windows で流さないとその層のテストが一度も実行されないため。
3. **Q3. 薄い一本の確認（取得の確認用プログラムと GUI の目視）の OS**（C2・C3）：A. その作業の対象の OS で行い、両方の OS に効く変更は macOS と Windows の両方で行う／B. Windows の作業のあいだは Windows だけで行う（macOS は CI に任せる）／C. いつも両方で行う。推奨は A。今回の作業は `sso_cli` など共通のコードも触るので、macOS で前と同じに動くことも手で確かめたいため。
4. **Q4. リリースの作り方**（C9）：A. 1 つのタグで macOS と Windows の両方を作り、両方そろってから 1 つの Release に公開する（どちらかが失敗したら何も公開しない）。dry run も両方を作る／B. 1 つのタグで OS ごとに作り、できた順に同じ Release に足す／C. OS ごとに別のタグや別のワークフローにする。推奨は A。片方だけの Release が出ると、その版で入れられない OS ができるため。
5. **Q5. 次の版の番号**（C11、P2）：A. `v0.2.0`（Windows という新しい配布先を足すため、MINOR を上げる）／B. `v0.1.2`／C. Windows 対応では版を上げず、次のリリースのときに決める。推奨は A。ただし 261012-parallel-fetch（未リリース）を先に出すかどうかで変わるので、人間が決める。どれでも「一度公開したタグは付け替えない」は残す。
6. **Q6. 対象にする Windows 11 の最低の版**（C7）：A. Microsoft がサポートしている Windows 11（x64）の版。手元の Windows の PC の版で確かめる／B. 特定のビルド（例：23H2 以降）を書く／C. 書かない（「Windows 11（x64）」だけ）。推奨は A。番号を固定すると古くなるため。
7. **Q7. Windows の手順書の置き方と、AWS CLI の書き方**（C11）：A. `docs/install.md` に macOS と Windows の節を分けて書き、AWS CLI v2 の入れ方とアプリが探す場所を OS ごとに書く／B. Windows 用に `docs/install-windows.md` を別に作る／C. README に書く。推奨は A。更新・戻し方・会社のルールの確認など、共通の部分を一度だけ書けるため。SmartScreen の開き方（「詳細情報」→「実行」）もここに書く。
8. **Q8. 改行コード**（C12）：A. `.gitattributes` でテキストを LF に固定する／B. 何もしない（Windows で取り出すときは `core.autocrlf=false` にする、と README に書く）／C. 改行の扱いを Prettier の `endOfLine: auto` にする。推奨は A。Windows の PC でソースから動かすときにも、CI と同じ結果になるため。

## 残っている不確かさ

- Windows 11 の「スマート アプリ コントロール」が有効な PC では、署名のないアプリは SmartScreen の「実行」でも開けない。intent の Q8 で「会社の Windows の PC に署名のないアプリを入れてよい」と確かめてあるが、開発者の PC と会社の PC でこの機能が切れているかは確かめていない。要件分析で確かめる（遅くとも最初の Windows の確認で分かる）。開けなければ、Deployment の「禁止と分かったら見直す」に当たる。
- Windows のランナー（`windows-latest`）でのビルドとテストにかかる時間はまだ分からない。CI が遅くなりすぎるなら、Windows で流すテストの範囲を後で見直す。
- NSIS の利用者ごとの導入で、前の版に戻す（ダウングレードする）ときにインストーラが止めないか。Tauri の設定（`allowDowngrades`）で決まるので、設計で確かめる。
- `v0.2.0` は取得の高速化（PR #22）と Windows 版をまとめて出すため、Windows 対応が終わるまで取得の高速化は配られない。リリースノートには両方の変更を書く必要がある。
- 改行を LF に固定する `.gitattributes` を入れたとき、既存のファイルに CRLF が混じっていれば一度だけ正規化の差分が出る。今の Linux・macOS での取り出しでは出ない見込みだが、入れる PR で確かめる。
- Windows のランナーを必須チェックに加えると、ジョブ名（matrix の OS 名を含む `Tauri app (windows-latest)` など）をルールセットにそのまま書く必要がある。名前は CI を直す PR で決まるので、その PR の中で合わせる（Way of Working の決まりと project.md の 261012 の学びのとおり）。

## Sources

- Initial description: [desc] 「Windows 対応をしたい（IB-16）。今は macOS（Apple Silicon）だけに配っているが、Windows でも動かして配れるようにしたい。終わったら次は Logs Insights 互換のクエリ（IB-11）をやる予定。」
- Workflow-selected scope: [scope] `bugfix`（計画名 `platform-port`）
- `aidlc/spaces/default/intents/261015-windows-support/ideation/intent-capture/intent-statement.md`、`intent-capture-questions.md`（Q1〜Q8）、`aidlc-state.md`
- `aidlc/spaces/default/memory/team.md`、`project.md`、`org.md`、`phases/inception.md`
- 逆解析：`code-structure.md`、`technology-stack.md`、`dependencies.md`、`code-quality-assessment.md`、`architecture.md`、`business-overview.md`、`reverse-engineering-timestamp.md`（`aidlc/spaces/default/codekb/local-sights/`）
- `aidlc/spaces/default/intents/261012-parallel-fetch/inception/practices-discovery/`、`aidlc/spaces/default/intents/261012-adhoc-signing/operation/deployment-execution/deployment-log.md`
- `git log`、`git log origin/main`、`git branch -a`、`git tag -l -n1`、`git ls-remote --tags origin`、`git show --stat 645d77e`、`git rev-parse HEAD`
- GitHub REST API（読み取りのみ）：`repos/kdnakt/local-sights/rules/branches/main`、`/releases`
- `.github/workflows/ci.yml`、`release.yml`、`release-dry-run.yml`、`src-tauri/tauri.conf.json`、`Cargo.toml`、`src-tauri/Cargo.toml`、`package.json`、`deny.toml`、`.prettierrc.json`、`scripts/release/versions.ts`、`scripts/release/signing.ts`、`src-tauri/src/sso_cli.rs`、`src-tauri/src/lib.rs`、`crates/local-sights-core/src/cache/mod.rs`、`docs/install.md`、`docs/releasing.md`、`README.md`
- `aidlc/spaces/default/intents/261015-windows-support/inception/practices-discovery/practices-discovery-questions.md`（Q1〜Q8 の回答）
- `aidlc engine now`、`git rev-parse --short HEAD`（下書きと統合の時刻・コミット）

## Assumptions & Open Questions

- [assumption] スコープ `bugfix` の org.md の決まり（対象の不具合への回帰テストと、既存のテストが通ること）は、今回は「Windows で動かない」ことへの回帰テスト（Windows のランナーでのテスト）と、既存のテストが全 OS で通ることに当たると読んだ。チームのカバレッジ 80% の決まりはスコープによらず CI で確かめる。
- [assumption] GitHub のルールセットと Releases の状態は、下書きの時点（2026-10-15）に API で読んだもの。
- [assumption] Tauri の Windows での設定とキャッシュの既定の置き場所（`%APPDATA%`／`%LOCALAPPDATA%` の下）と、NSIS の利用者ごとの導入ができることは、Tauri 2 の一般的な振る舞いとして書いた。このリポジトリではまだ確かめていない。
- 面談の Q1〜Q8 はすべて回答済み（上の「面談の決定」）。人間に残っている判断は、affirmation の承認だけ。
