# Code Generation Plan — Windows 対応（IB-16）

## Summary

- Builds: Windows 11（x64）でアプリを動かし、NSIS のインストーラを CI で作って macOS の `.dmg` と一緒に配れるようにする
- Touches: `src-tauri/src/sso_cli.rs`（OS ごとに分割）、`src-tauri/src/lib.rs`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`.github/workflows/*.yml`、`.gitattributes`、`scripts/release/signing.ts`、`docs/install.md`、`docs/releasing.md`、`README.md`、版の 5 ファイル
- Tests: Rust 約 14 件（AWS CLI の探索の純粋なロジック 4 件を先に、Windows の起動と停止 6 件と Unix の既存テストの整理、キャッシュの Windows 1 件、`cli_candidates` の Unix 1 件）、TypeScript 約 4 件（`signing.ts` のジョブ分割）

## 前提と進め方

- 上流：`inception/requirements-analysis/requirements.md`（FR1〜FR5、NFR1〜NFR6）、`inception/practices-discovery/team-practices.md`（承認済み、team.md に反映済み）。作業単位はなく、要件分析の Q7 のとおり 1 つの PR にまとめる。
- ただし team.md の Walking Skeleton に従い、**Part A（薄い一本）を先に作って開発者の Windows 11 の PC で確かめてから Part B に進む**。Part A の終わりで枝を push し、GitHub の「Release dry run」を手で動かして Windows のインストーラを得る。それを Windows の PC に入れて、起動・取得・表示を目で確かめる（team.md の確認 2 と 3）。その確認を人間が承認してから Part B を始める。
- 進め方の順序（Testing Contract の `ordering`）：純粋なロジック（AWS CLI の候補の並べ方、改行の扱いなど）はテストを先に書く。OS に触れる境界の層（プロセスの起動と停止、ブラウザーを開く、ファイルの権限）は先に実装してからその層のテストを書く。CI とワークフローは、書いたあとに CI で確かめる。
- 既存の macOS・Linux の動きは変えない。OS ごとの処理は `cfg(unix)` / `cfg(windows)` で分けたモジュールに閉じ込め、判断は OS に依存しない関数に寄せる（team.md Code Style）。
- レビューで指摘された点のうち、この計画で扱うもの：R-01（タグを付ける順序は `docs/releasing.md` に書く。タグ付けと公開はこの PR の範囲外で、マージ後に行う）、R-02（FR4.4 の合否基準を Step A6 に書く）、R-03（Step A5）、R-04（Ctrl+Y を手順書と README に含める）、R-05（`PATH` で見つけた絶対パスで起動する）、R-06（手順書に「閉じてから入れる」を書く）。

## Testing Contract

```json
{
  "version": 1,
  "methodology": "custom",
  "source": "team",
  "ordering": "時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続・GUI・OS ごとに分かれる境界の層（プロセスの起動と停止、ファイルの置き場所と権限など）は先に実装してからその層のテストを書いて実行する。",
  "scope": "bugfix",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "私たちはテストを各 Bolt の成果物の一部として扱う。純粋なロジックはテストを先に書き、外部とつながる層は実装してからテストを書く。\n\n- **Methodology**: custom\n- **Methodology evidence**: 261002-cwlogs-viewer の面談（Q4 の回答 B）で、純粋なロジックはテスト先行、AWS 接続と GUI の層は実装後にテスト、という混在した進め方に決め、261011-local-release と 261012-parallel-fetch の面談でも変えていない。261015-windows-support の面談でも進め方そのものを変える回答はなく（Q2 は Windows のランナーでテストを流す決まりを加えるだけ）、増えるのは OS ごとに分かれる境界の層（AWS CLI の起動と停止、ファイルの置き場所と権限など）で、純粋なロジックの扱いは変わらない。今のコード（判断は副作用のない関数に寄せ、OS に触れる処理は `cfg(unix)` などで境界に閉じ込め、テストは各モジュールの中にある）と git の履歴にも、これと食い違う跡はない。\n- **Ordering**: 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続・GUI・OS ごとに分かれる境界の層（プロセスの起動と停止、ファイルの置き場所と権限など）は先に実装してからその層のテストを書いて実行する。\n- テストの量と種類は、記録済みの Test Strategy（`Standard`）に従う。\n- カバレッジ：行カバレッジを CI で測り、80% を下回る PR はマージしない（ライブラリは `cargo-llvm-cov`、画面は Vitest のカバレッジ）。計測は Linux のランナーで行う。これは org.md のスコープ別の下限（`mvp` などの 80% 行カバレッジとマージ前の CI での実行）を CI の合否で確かめるもので、下限を下げることはしない。そのほかのスコープ別の決まりも org.md に従う。\n- GUI に依存しない層（ライブラリ側）の公開関数には必ずテストを書く。\n- 自動テストと CI では実際の AWS に接続しない。AWS 呼び出しは trait の裏に置き、テストではモック／スタブに差し替える。AWS CLI もテストでは本物を使わず、偽の CLI に差し替える。\n- OS ごとに分かれるコード（`cfg(windows)`、`cfg(unix)` など）のテストは、その OS のランナーで実行されるように書く。偽の CLI などテストの道具も OS ごとに用意する。\n- Rust 側のテストは `cargo test`（ライブラリとアプリの両方）で、画面側（フロントエンド）のテストは Vitest で実行し、どちらも CI でマージ前に実行する。Rust 側のテストは CI の Linux・macOS・Windows（`windows-latest`）の各ランナーで実行し、画面側のテストは OS に依存しないため Linux のランナーで実行する。既存のテストはすべての OS で通った状態を保つ。"
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Include a targeted regression for the bug or vulnerability.",
      "Keep the existing test suite green."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "custom",
    "runner_step": "Verify the existing test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Verify the existing test runner/configuration and record the exact unit-scoped command.",
      "Custom ordering - 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続・GUI・OS ごとに分かれる境界の層（プロセスの起動と停止、ファイルの置き場所と権限など）は先に実装してからその層のテストを書いて実行する。",
      "Implementation and tests - preserve that exact ordering; do not convert it to layer-local TDD.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:c204f9e18687e8e7cb7ba26a06f514f19d2b72d60c2236a0cac5469a219cf637",
  "contract_sha256": "sha256:7fefd26226a18e728959970841137f713bebd120c812559c22e382021fd784ff"
}
```

## Part A — 薄い一本（Windows でビルド・テストが通り、CI で作ったインストーラで入れて取得・表示できる）

### Step A1: テストの実行環境を確かめ、実行する命令を記録する（runner step）

- [x] この Linux の作業環境で `cargo test -p local-sights sso_cli` と `cargo test -p local-sights-core cache::` が今のまま通ることを確かめる（既存の緑の状態を基準にする。NFR1）。
- [x] `npx vitest run scripts/release/signing.test.ts` が通ることを確かめる。
- [x] 命令は `unit-test-instructions.md` に記録済み。Windows でしか動かないテストは、CI の `Tauri app (windows-latest)` で確かめる（この環境では `cargo check --target x86_64-pc-windows-msvc` もリンカがないため通らないので、コンパイルの確認も CI に頼る。Step A8 の push で確かめる）。

### Step A2: 改行コードを LF に固定する（FR1.4、team.md Code Style）

- [x] `git ls-files --eol` で、作業ツリーに CRLF のテキストファイルがないことを確かめる（OQ3）。あれば LF にそろえ、`code-summary.md` にその差分を記録する。
- [x] `.gitattributes` を作る：`* text=auto eol=lf`。画像とバイナリ（`*.png`、`*.icns`、`*.ico`、`*.dmg`、`*.exe`）は `binary`。
- [x] `git add --renormalize .` で差分が出ないことを確かめる（出たら Step A2 の 1 つ目に戻る）。

### Step A3: AWS CLI の候補の並べ方（純粋なロジック、テスト先行）（FR4.1、R-05）

- [x] `src-tauri/src/sso_cli.rs` に、OS に依存しない関数 `cli_candidates(program_files: Option<&OsStr>, path: Option<&OsStr>) -> Vec<PathBuf>` の**テストを先に書く**（`cfg(windows)` のテスト 3 件と、Unix 側の 1 件）：
  - Windows：`program_files` が `C:\Program Files` なら、先頭は `C:\Program Files\Amazon\AWSCLIV2\aws.exe`。
  - Windows：`path` が `C:\a;C:\b` なら、続きは `C:\a\aws.exe`、`C:\b\aws.exe` の順（`std::env::split_paths`）。空の要素は飛ばす。
  - Windows：`program_files` が `None` なら `C:\Program Files` を既定にする。
  - Unix：引数にかかわらず `/opt/homebrew/bin/aws`、`/usr/local/bin/aws` の 2 つだけ（`PATH` は使わない。今の `CLI_CANDIDATES` と同じ）。
- [x] `cli_candidates` を実装する。`run_sso_login`（`lib.rs`）は `locate_cli(&cli_candidates(env::var_os("ProgramFiles"), env::var_os("PATH")))` を使う。`locate_cli` は見つけた**絶対パス**を返し、そのパスで起動する（R-05：`PATH` 上の別の `aws.exe` は、見つかった場所が記録されるだけで、起動するのはその 1 つ）。
- [x] `locate_cli` の「実行できるファイルか」の判定を OS ごとに分ける：Unix は今どおり実行ビット、Windows は `is_file()` かつ拡張子が `exe`（大文字小文字を問わない）。既存の `locate_cli` のテスト 2 件は Unix 専用のまま（`from_mode` を使うため `cfg(unix)`）。Windows には同じ 2 件を `.exe` の有無で書く。

### Step A4: AWS CLI の起動と停止を OS ごとに分ける（境界の層：実装してからテスト）（FR4.2〜FR4.4、NFR2）

- [x] `sso_cli.rs` を `sso_cli/mod.rs`（共通：`locate_cli`、`cli_candidates`、`LoginTimes`、`LoginControl`、`run_login`、出力の読み取り）と `sso_cli/unix.rs`・`sso_cli/windows.rs`（境界）に分ける。境界の API は両 OS で同じ形にする：
  - `fn spawn_cli(cli: &Path, args: &[String]) -> io::Result<(Child, ProcessGroup)>`
  - `struct ProcessGroup`（`Send`。Unix は `libc::pid_t`、Windows はジョブ オブジェクトのハンドル）
  - `impl ProcessGroup { fn terminate(&self, grace: Duration, reap: impl FnMut() -> bool); fn exists(&self) -> bool }`
  - `ControlState.process_group` は `Option<ProcessGroup>` にする（`stop_now` から呼ぶため）。
- [x] Unix 側：今の実装をそのまま移す（`process_group(0)`、SIGTERM → grace → SIGKILL、`killpg`）。動きは変えない。
- [x] Windows 側：
  - 起動は `std::os::windows::process::CommandExt::creation_flags(CREATE_NO_WINDOW)` で、コマンドプロンプトの窓を出さない（FR4.2）。シェルは通さず、引数はそのまま渡し、標準入力は閉じる（今と同じ）。
  - 起動した直後に**ジョブ オブジェクト**を作り（`CreateJobObjectW`）、`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` を設定して（`SetInformationJobObject`）、子プロセスを割り当てる（`AssignProcessToJobObject`、`Child::as_raw_handle`）。CLI が起こした子は自動的に同じジョブに入る。
  - `terminate` は `TerminateJobObject(job, 1)`。Windows には SIGTERM に当たる穏やかな停止がないため、grace は使わず即座に止める（この違いは `code-summary.md` と README に書く）。`exists` は `QueryInformationJobObject(JobObjectBasicAccountingInformation)` の `ActiveProcesses > 0`。
  - ハンドルは `ProcessGroup` の `Drop` で `CloseHandle`。`KILL_ON_JOB_CLOSE` のため、アプリが異常終了してハンドルが閉じられても OS が CLI の木を止める（R-02 の強制終了・クラッシュの扱い：アプリ側が死んでも残らない）。
  - Windows API は `windows-sys` クレートで呼ぶ（`Win32_Foundation`、`Win32_System_JobObjects`、`Win32_System_Threading`）。
- [x] `src-tauri/Cargo.toml`：`libc` を `[target.'cfg(unix)'.dependencies]` に移し、`[target.'cfg(windows)'.dependencies]` に `windows-sys` を足す。版は `Cargo.lock` にすでにある版（Tauri が使っているもの）に合わせ、`cargo tree -i windows-sys` で確かめる。`cargo deny check` が通ること（新しいライセンスはない：MIT OR Apache-2.0）。
- [x] **FR4.4 の合否基準（R-02）**：中断・時間切れ・アプリ終了（`stop_now`）のそれぞれで、`run_login` が戻ったあと 2 秒以内に、偽の CLI とその子プロセスの両方が存在しない。自動テストで確かめる（Step A5）。手元では、README の Windows の項目でタスク マネージャーに `aws.exe` が残らないことを見る。アプリの強制終了（タスク マネージャーでの終了やクラッシュ）では `KILL_ON_JOB_CLOSE` により OS が止める。これは自動テストでは確かめず、README の項目に 1 行で書く。

### Step A5: 境界の層のテストを書く（Windows の偽の CLI、Unix の既存テストの整理）（NFR2）

- [x] 既存の `sso_cli` のテスト 11 件は `#[cfg(unix)]` の `mod unix_tests` にまとめ、動きを変えない。
- [x] `#[cfg(windows)]` の `mod windows_tests` を書く（6 件）。偽の CLI は、テストが一時フォルダに書く `.cmd` のバッチ ファイル（Rust の `Command` は `.cmd` を `cmd.exe` 経由で起動するが、これはテストの道具であって、アプリの起動の仕方ではない）。子プロセスが要るテストでは、バッチから `powershell -NoProfile -Command` を起こし、PowerShell が自分の `$PID` をファイルに書いてから `Start-Sleep 600` する。
  1. 成功した CLI の URL とコードが届き、Succeeded になる（標準出力と標準エラーの両方から）。
  2. 失敗した CLI の終了コードが届く（`exit /b 3`）。
  3. 起動できない CLI は `Failed { exit_code: None }`。
  4. 時間切れで `TimedOut` になり、CLI と子（PowerShell）が残らない。
  5. `request_stop`（中断）で `Cancelled` になり、CLI と子が残らない。
  6. `stop_now`（アプリ終了）で、戻る前に子が止まり、`Cancelled` になる。
  - 「残らない」は、`tasklist /FI "PID eq <pid>"` の出力に `<pid>` が含まれなくなることで判定する（2 秒まで待つ）。ジョブ オブジェクトで止めるため、Unix の「SIGTERM を無視する CLI」のテストに当たるものは不要（即座に止める）。
  - 一度に 1 つだけ動かす `SERIAL` のロックは両 OS で共通。
- [x] `crates/local-sights-core/src/cache/mod.rs` に `#[cfg(windows)]` のテストを 1 件足す：`ensure_private_dir` が Windows でフォルダを作って `Ok` を返し、2 回目も `Ok`（権限は OS の既定に任せる。FR3 の NFR3、Q3）。ライブラリの既存テストが Windows でも通ることは CI で確かめる（R-03）。

### Step A6: ブラウザーを開く処理と macOS 専用の部分を OS ごとに分ける（FR3.5、FR3.6、FR4.5）

- [x] `lib.rs` の `open_sso_url` が使う起動部分を、`src-tauri/src/browser.rs` の `open_url(url: &str) -> io::Result<()>` に移す：Unix は今どおり `/usr/bin/open`（固定パス、シェルなし）。Windows は `windows-sys` の `ShellExecuteW`（`Win32_UI_Shell`、動詞 `open`、`SW_SHOWNORMAL`）で既定のブラウザーに渡す。シェルは通さない。URL の許可の検査（`is_allowed_login_url` と今の試行の URL か）は `open_sso_url` に残し、変えない。
- [x] `app_menu` と `handle_menu_event` が Windows でコンパイルできることを確かめる。`builder.menu(app_menu)` は今どおり `cfg(target_os = "macos")` だけ（Q2：Windows ではメニューバーを出さない）。`handle_menu_event` が macOS 専用の項目に依存しているなら、`on_menu_event` も同じ `cfg` で囲む。
- [x] `WindowEvent::CloseRequested`（閉じるボタン、Alt+F4）と `RunEvent::ExitRequested` の扱いは変えない（FR3.4）。`RunEvent::Exit` で `stop_sso_cli` を呼ぶ今の流れで、Windows でもジョブが止まる。
- [x] `lib.rs` の doc コメント（`/usr/bin/open`、Unix 専用の記述）を両 OS の説明に直す。

### Step A7: CI に Windows を足す（FR1.1、FR1.2、FR1.5）

- [x] `.github/workflows/ci.yml` の `tauri-app` の matrix を `[ubuntu-latest, macos-latest, windows-latest]` にする。Linux 専用のパッケージの手順は今どおり `if: runner.os == 'Linux'`。
- [x] 同じジョブに `tests (library)`（`cargo test -p local-sights-core`）を足し、`if: runner.os != 'Linux'` にする（Linux のライブラリのテストは `rust-core` でカバレッジ付きで動く。team.md Testing Posture）。
- [x] 既存のジョブ名（`Rust library (fmt, clippy, test)`、`Front end (...)`、`cargo-deny`、`Tauri app (ubuntu-latest)`、`Tauri app (macos-latest)`）は変えない（project.md Corrections）。
- [x] `Swatinem/rust-cache` の `key: ${{ matrix.os }}` は Windows にも効く。`npm ci` と `npm run build` は Windows でもそのまま動く。

### Step A8: Windows のインストーラを dry run で作れるようにする（FR2.1、FR2.2、FR2.4）

- [x] `src-tauri/tauri.conf.json` の `bundle` に `windows` を足す：`nsis.installMode: "currentUser"`（管理者の権限なし、利用者ごと。FR2.1）、`webviewInstallMode: { "type": "downloadBootstrapper" }`（Q5：ない PC でだけ取得。Tauri の既定を明示する）、`allowDowngrades: true`（FR2.8、OQ2：前の版のインストーラで戻せる）。`targets` は `"all"` のまま、どのインストーラを作るかはワークフローの `--bundles` で決める。
- [x] `.github/workflows/release-dry-run.yml` に `build-windows` ジョブを足す：`windows-latest`、`npm run check:version`、`npm ci`、`npm run tauri build -- --bundles nsis`、`target/release/bundle/nsis/` に `*-setup.exe` がちょうど 1 つあり名前に `x64` を含むことを PowerShell で確かめ、`local-sights-windows-dry-run` という成果物として 7 日残す。既存の macOS のジョブは `build-dmg` のまま変えない。
- [x] ここまでを 1 つのコミットにして枝 `claude/windows-support` に push する。**ここで Part A は終わり**：CI の `Tauri app (windows-latest)` が通ること、「Release dry run」を手で動かして Windows のインストーラができることを確かめ、開発者が Windows 11 の PC に入れて、起動・プロファイルとリージョンの選択・取得・表示を目で確かめる。macOS でも前と同じに動くことを確かめる（NFR1）。人間の承認を待つ。

## Part B — 残りの機能、リリース、手順書

### Step B1: リリースのワークフローを両 OS にする（FR2.3、FR2.5、FR2.6）

- [x] `.github/workflows/release.yml` を 5 つのジョブにする。ファイル内の並びは **`verify` → `test` → `build-macos` → `build-windows` → `publish`** にする（`scripts/release/signing.ts` は、`tauri build` を含む最初のステップの `APPLE_SIGNING_IDENTITY` と、`codesign --verify` が `tauri build`・`.dmg` の選別の後、`gh release` の前にあることを、ファイル全体のステップの並びで見るため）。
  - `build-macos`：今の `build-and-publish` から、`.dmg` を作る・1 つだけ確かめる・署名を検証するまでを移し、`Local-Sights_<ver>_aarch64.dmg` を成果物として上げる。
  - `build-windows`：`windows-latest` で `npm run tauri build -- --bundles nsis`、`*-setup.exe` が 1 つで `x64` を含むことを確かめ、`Local-Sights_<ver>_x64-setup.exe` という名前の複製を成果物として上げる。
  - `publish`：`needs: [build-macos, build-windows]`。両方の成果物を取得し、`gh release create`（または既存の Release への `upload --clobber`）で 2 つを 1 つの Release に載せる。どちらかのビルドが失敗すれば `publish` は動かない（FR2.3）。`NOTES` に Windows が未署名で SmartScreen が出ることを書く。
- [x] `scripts/release/signing.test.ts` に、ジョブが分かれた形のワークフローのテストを足す（`readTauriBuildSigningIdentity` が macOS のジョブの `tauri build` を拾うこと、`findSignatureCheckStep` が `publish` ジョブの `gh release` の前にあると判定すること、Windows のジョブの `tauri build` が先にあると失敗すること、の 3〜4 件）。既存のテストは変えない。`signing.ts` の読み方（ファイル全体のステップの並び）で足りるなら、`signing.ts` は変えない。
- [x] `versions.ts` と `check-version.mjs` は変えない。

### Step B2: 版を 0.2.0 にする（FR2.9）

- [x] `Cargo.toml` の `[workspace.package] version = "0.2.0"`、`npm version 0.2.0 --no-git-tag-version`（`package.json` と `package-lock.json`）、`src-tauri/tauri.conf.json` の `"version": "0.2.0"`、`cargo update -w`。`npm run check:version` が `0.2.0` を出すことを確かめる。

### Step B3: 手順書（FR5.1、FR5.3、R-06）

- [x] `docs/install.md` を「macOS」と「Windows」の節に分ける。macOS の節は今の内容のまま。Windows の節に書くこと：
  - 対象（Microsoft がサポートしている版の Windows 11、x64）。
  - 会社のルールの確認（未署名のアプリ。**スマート アプリ コントロール**がオンだと開けないこと、Windows セキュリティでの確認の場所。A4）。
  - 取得（`x64-setup.exe`）と入れ方（ダブルクリック、管理者の権限は不要、利用者ごとに入る）。
  - 初回起動の SmartScreen の警告：「詳細情報」→「実行」。
  - AWS CLI v2 の入れ方（AWS の MSI）と、アプリが探す場所（`C:\Program Files\Amazon\AWSCLIV2\aws.exe`、なければ `PATH`）。
  - 設定とキャッシュの場所（`%APPDATA%\dev.local-sights.app\`、`%LOCALAPPDATA%\dev.local-sights.app\`）と、権限は Windows の既定のままであること（Q3）。
  - 更新：**アプリを閉じてから**新しい版のインストーラを実行する（R-06）。戻し方：前の版のインストーラを実行する。アンインストール：設定 → アプリ。
  - 署名についての言い回しは、`signing.test.ts` の `findUnsignedClaims` が「署名がまったくない」と読む書き方（`unsigned`、`not signed`）を避け、何の署名がないか（`not signed with a code-signing certificate`）を書く。`npx vitest run scripts/release/signing.test.ts` で確かめる。
- [x] `docs/releasing.md`：Windows のインストーラも作られ、両方がそろったときだけ公開されること。dry run が両方を作ること。**タグを付ける前の順序**（R-01）：dry run を通し、両 OS で入れて起動・取得を確かめてからタグを付ける。公開後に壊れていたら、タグは動かさず次のパッチ版（`v0.2.1`）で直す。

### Step B4: README（FR5.2、FR4.1、R-04）

- [x] 「Install」「Prerequisites」「Sign in with SSO」に Windows の記述を足す（AWS CLI の探し方、Ctrl+C／Ctrl+V／Ctrl+X／Ctrl+A／Ctrl+Z／Ctrl+Y の標準の操作、メニューバーがないこと）。
- [x] 「Manual GUI check」に Windows の項目を足す。macOS の各項目に対応させる：Cmd+Q とアプリのメニューの終了 → Alt+F4 と閉じるボタン、Dock → タスクバー、Cmd+Tab → Alt+Tab、VoiceOver → ナレーター、アクティビティモニタ → タスク マネージャー、Cmd+C／Cmd+V → Ctrl+C／Ctrl+V、Dock とアイコン → タスクバーとスタートメニューのアイコン。Dock の「終了」に当たる項目は「タスク マネージャーからの終了：確認は出ない。`aws.exe` が残らない」とする（OQ4）。
- [x] 「Tests and checks」に、Windows でしか動かないテストと CI の Windows のジョブのことを書く。

### Step B5: 仕上げ

- [x] `cargo fmt --all`、`cargo clippy --workspace --all-targets`（この環境では Linux だけ）、`cargo test --workspace`、`npx prettier --write .`、`npx eslint .`、`npx tsc --noEmit`、`npx vitest run`、`cargo deny check`、`npm audit --audit-level=high` を通す。
- [ ] push して CI の 3 つの OS が通ることを確かめる。`Tauri app (windows-latest)` を `main` のルールセットの必須の状態確認に加える（FR1.3）。これはリポジトリの設定で、コードではないため、**開発者のエージェントは行わず、PR のときに人間（または進行役）が GitHub の設定で行う**。
- [x] `code-summary.md` と `traceability.json` を書く（Step 5）。

## 要件との対応

| 要件 | ステップ |
|---|---|
| FR1.1, FR1.2, FR1.5 | A7 |
| FR1.3 | B5（リポジトリ設定） |
| FR1.4 | A2 |
| FR2.1, FR2.2, FR2.4, FR2.8 | A8 |
| FR2.3, FR2.5, FR2.6 | B1 |
| FR2.7 | B3（手順書）、A8（`allowDowngrades`） |
| FR2.9 | B2 |
| FR3.1〜FR3.3 | A6, A8（コードは変えず、Windows で動くことを Part A の確認で確かめる） |
| FR3.4〜FR3.6 | A6 |
| FR4.1 | A3 |
| FR4.2〜FR4.4 | A4, A5 |
| FR4.5 | A6 |
| FR4.6 | B3（手順書。画面の文言は変えない） |
| FR5.1, FR5.3 | B3 |
| FR5.2 | B4 |
| NFR1 | A1, A8, B5 |
| NFR2 | A5, A7 |
| NFR3 | A4（CLI の出力の扱いは変えない）, A5（キャッシュ） |
| NFR4 | A8, B3 |
| NFR5 | A3（`Program Files` の空白）, Part A の確認 |
| NFR6 | B4 |
