# Evidence

> 統合版（リード、Step 5）。協力エージェントなしの実行（collaborators: off）のため、調べたのはリード 1 人で、寄稿ファイルはない。逆解析（reverse-engineering）の成果物はこの記録にないので、ワークスペースそのものを証拠として使った。

## 調べたもの・推論したこと（リード：aidlc-pipeline-deploy-agent）

| 対象 | 内容 |
|------|------|
| `aidlc/spaces/default/memory/team.md` | 5 セクションとも確定済み（2026-10-03）。再実行の基準として使った。`## Deployment` は「MVP の配布はソースからのビルドだけ」「GitHub Releases と macOS の署名・公証は MVP の後に検討」。 |
| `aidlc/spaces/default/memory/project.md` | `## Forbidden` の 4 件（秘密の認証情報、読み取り API 以外、テストや CI から実際の AWS、テレメトリ）と `## Mandated` の `None.`。今回の配布の案はこれらに触れないように組んだ（例：自動更新はしない）。 |
| `aidlc/spaces/default/memory/org.md` | トランクベース・スクワッシュマージ、マージでステージングへ・本番は手動承認、の既定。サーバーがないため Deployment の既定はそのままでは当てはまらない（team.md の記述と同じ）。 |
| `aidlc-state.md`（261011-local-release） | Brownfield。スコープ `bugfix`、プラン `local-release`。Test Strategy `Standard`。実行するのは practices-discovery → requirements-analysis → code-generation → build-and-test → ci-pipeline。Operation の段階（deployment-pipeline ほか）はすべて SKIP。 |
| 261002-cwlogs-viewer の `inception/practices-discovery/evidence.md` | 前回の決定（Q6：MVP はソースビルドのみ、Releases は後で）と、未解決として残した「配布の後続：GitHub Releases、タグ規約、macOS の署名・公証は MVP 後に決める」。前回の initiative brief の要約では対象は macOS のみ（D-16）、個人開発・OSS。 |
| `git log`（12 コミット、2026-10-03〜2026-10-10） | すべて PR 番号付きのスクワッシュコミット（`(#1)`〜`(#12)`）。`main` への直接 push の跡はない。Dependabot の PR も同じ流れでマージされている。タグは 1 つもない（`git tag --list` が空）。下書きの時点の HEAD は `e722b0a`。統合の時点の HEAD は `d1cdac8` で、その間の 2 コミット（`bb6b21e`、`d1cdac8`）は AI-DLC の記録だけの変更のため、上の調査結果は変わらない。 |
| `.github/workflows/ci.yml` | `pull_request` と `main` への push で実行。ジョブは `rust-core`（`cargo fmt --all --check`、`cargo clippy -p local-sights-core --all-targets`、`cargo test -p local-sights-core`）、`frontend`（`tsc --noEmit`、`eslint`、`prettier --check`、`vitest run`、`vite build`、`npm audit --audit-level=high`）、`cargo-deny`、`tauri-app`（ubuntu と macOS で `npm run build`、`cargo clippy --workspace --all-targets`、`cargo build -p local-sights`）。`permissions: contents: read`。リリース用のワークフロー、タグを合図にするジョブ、成果物（`.app` / `.dmg`）を作るジョブはない。AWS の認証情報は使っていない。 |
| `.github/dependabot.yml` | cargo・npm・github-actions を毎週更新。aws-sdk・tauri・テスト関連をまとめる。 |
| `deny.toml` | 脆弱性・unmaintained は失敗。許可するライセンスを列挙。取得元は crates.io のみ。例外は RUSTSEC-2024-0370（tauri → gtk 経由、Linux のみ）。 |
| `src-tauri/tauri.conf.json` | `productName` `local-sights`、`version` `0.1.0`、`identifier` `dev.local-sights.app`、`bundle.active: true`、`bundle.targets: "all"`、アイコンは `.icns`・`.ico` まで用意済み。署名（`macOS.signingIdentity` 等）、公証、更新（updater）の設定はない。 |
| `Cargo.toml`（ワークスペース）、`src-tauri/Cargo.toml`、`package.json` | 版は 3 か所とも `0.1.0`（`workspace.package.version`、`package.json` の `version`、`tauri.conf.json` の `version`）で、それぞれ別に書かれている。`publish = false`。`custom-protocol` 機能はソースから `cargo install` するときに必要。 |
| `README.md` | 実行方法として `npm run tauri dev` / `npm run tauri build`（`target/release/bundle/` に成果物）と、`cargo install --path src-tauri --features custom-protocol`。手作業の確認リストは macOS 前提（`~/Library/...`、Cmd+C、Dock）。ライセンスは Apache-2.0（`LICENSE` あり）。 |
| `.gitignore`、`.prettierrc.json`、`eslint.config.js`、`tsconfig.json`、`src/**/*.test.ts(x)` | 画面側にも Prettier・ESLint・TypeScript の設定と Vitest のテスト（23 ファイル）がある。`src-tauri/` には Rust のテストがない。 |

推論：

- 会社の PC も macOS の可能性が高い（前回の D-16、README の手順、CI の macOS ランナー）。ただし会社の PC は本人の私物の Mac と条件が違う（管理者権限、MDM による制限、未署名アプリの扱い、社内ネットワークやプロキシ）ので、OS と CPU と会社の決まりは人間に確かめる必要がある。
- `tauri.conf.json` はすでに配布用のバンドルを作れる状態で、足りないのは「どこで作り、どこに置き、どう署名し、どう番号を付けるか」の決めごと。
- 自動更新（Tauri の updater）は外部への定期的な通信を増やす。`project.md` の「テレメトリを送らない」には直接は当たらないが、「外に何も送らない」というこれまでの方針に合わせ、下書きでは入れない案にした。
- 署名・公証を CI で行う場合は Apple の証明書を GitHub の secrets に置くことになる。`project.md` が禁じているのは AWS の認証情報だけなので禁止には当たらないが、人間の判断が要る。

## Sources

- Initial description: [desc] 「ざっくりできたから、このアプリをリリースする手順を整えたい。つまり、自分の会社のPCでインストールして使うための準備。」
- Workflow-selected scope: [scope] `bugfix`（プラン `local-release`）
- `aidlc/spaces/default/memory/team.md`、`project.md`、`org.md`、`phases/inception.md`
- `aidlc/spaces/default/intents/261002-cwlogs-viewer/inception/practices-discovery/evidence.md`、`team-practices.md`、`discovered-rules.md`
- `git log`、`git tag --list`、`git rev-parse --short HEAD`
- `.github/workflows/ci.yml`、`.github/dependabot.yml`、`deny.toml`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`Cargo.toml`、`crates/local-sights-core/Cargo.toml`、`package.json`、`README.md`、`.gitignore`

- インタビューの回答：`aidlc/spaces/default/intents/261011-local-release/inception/practices-discovery/practices-discovery-questions.md`（Q1〜Q7、すべて回答済み）

## インタビューでの決定

人間が Q1〜Q7 で決めたこと。`team-practices.md` にはこれ以外の新しい決定を加えていない。

| 質問 | 回答 | 決定 | 反映先 |
|------|------|------|--------|
| [Q1] 会社 PC の OS と CPU | A | macOS（Apple Silicon）だけに配る | `## Deployment`（配布の対象） |
| [Q2] 会社 PC に何を入れるか | A | Tauri のインストーラ（`.app` を入れた `.dmg`）を作って入れる。`cargo install` は開発用に残す。回答の前に人間から「インストーラにすると版の更新が大変にならないか」と質問があり、更新は「新しい `.dmg` を取得し、`.app` を『アプリケーション』にドラッグして置き換える」、設定とキャッシュは `.app` の外にあるので残る、戻すときは前の `.dmg` を入れ直す、と説明したうえで A が選ばれた | `## Deployment`（配布の形、更新、戻し方） |
| [Q3] どこでビルドして、どこに置くか | A | `main` に `vX.Y.Z` のタグを付けると GitHub Actions がビルドし、GitHub Releases に下書きとして置く。会社 PC はそこから取得する。手でファイルを運ぶ予備の手順（B）は選ばれなかった | `## Deployment`（作る場所と置き場所） |
| [Q4] 署名と公証 | A | 今はしない。初回起動時の警告を開く方法を手順書に書く。会社のルールで未署名アプリが禁止と分かったら見直す | `## Deployment`（署名と公証）。署名用の秘密情報を扱わないため、R1 は `discovered-rules.md` に加えない |
| [Q5] バージョンと更新・巻き戻し | A | セマンティックバージョニング（`v0.1.0` から）。3 か所の版をそろえ、タグと合わなければリリースのビルドを失敗させる。自動更新は付けず手で入れ替え、巻き戻しは前の版を入れ直す（前の版は Releases に残す） | `## Deployment`（バージョン、更新、戻し方） |
| [Q6] 記述を CI の実態に合わせるか | A | 画面側のテスト（Vitest）と、Prettier・ESLint・`tsc --noEmit`・`npm audit --audit-level=high` を書き足す。下限の説明は「スコープ `local-tool`」ではなく「org.md のスコープ別の決まりに従う」に書き直す | `## Testing Posture`、`## Code Style` |
| [Q7] GitHub 側の設定 | A | `main` の保護と秘密情報スキャンはどちらも有効にしてある | `## Way of Working` と `## Code Style` は文面を変えずに確定 |

下書きで挙げた D7（リリース前とインストール後の確認）は質問に含めなかったため、`team-practices.md` には書いていない。D8（会社 PC の条件）は慣行ではないため質問に含めず、下の「未解決」として requirements-analysis に引き継ぐ。

## 下書きで挙げた検討項目（記録として残す）

各項目に、下書きの既定案と、その理由を書いた。最終的な決定は上の「インタビューでの決定」を正とする。

### Deployment（今回の中心）

| ID | 決めること | 下書きの既定案 | 理由 |
|----|------------|----------------|------|
| D1 | 会社の PC の OS と CPU（macOS か Windows か。macOS なら Apple Silicon か Intel か、両方か） | macOS・Apple Silicon のみ | 前回の決定（D-16：macOS のみ）、README と手作業の確認が macOS 前提、CI の `macos-latest` は Apple Silicon。Windows なら CI にも Windows のビルドがなく、作業が大きく変わる。 |
| D2 | 配布の形（`.dmg` / `.app` のコピー / ソースから `cargo install`） | `npm run tauri build` で作る `.app` を `.dmg` にまとめる。`cargo install` は開発用に残す | `cargo install` は `~/.cargo/bin` に実行ファイルを置くだけで、Rust と Node の開発環境が会社の PC にも要る。`.dmg` なら会社の PC には開発環境が要らない。 |
| D3 | どこでビルドし、どこに置くか（自分の Mac で作って手で運ぶ / タグを合図に GitHub Actions で作り GitHub Releases に置く） | タグ `vX.Y.Z` を `main` に付けたら GitHub Actions の macOS ランナーでビルドし、GitHub Releases（まず下書き）に添付する | 毎回同じ環境で作れ、どのコミットから作ったかが残る。手元のビルドは環境の違いが混ざりやすい。会社のネットワークから GitHub のダウンロードが許されない場合は、手で運ぶ案に切り替える。 |
| D4 | 署名と公証（Apple Developer Program の証明書で署名・公証する / ad-hoc 署名のまま配る） | 今回は ad-hoc 署名のまま配り、初回起動の警告を開く手順（「開く」を選ぶ、または隔離属性を外す）を README に書く | 署名・公証には年会費のかかる Apple Developer Program への加入と、証明書を CI の secrets に置く判断が要る。使うのが本人 1 人なら警告を開く手順で足りる見込み。ただし会社の MDM や決まりで未署名のアプリが禁止なら、署名・公証が必須になる。 |
| D5 | バージョンの付け方 | セマンティックバージョニング、タグは `vX.Y.Z`、最初は `v0.1.0`。3 か所の版（`Cargo.toml`・`package.json`・`tauri.conf.json`）をそろえ、タグと合わなければリリースのビルドを止める | 現状 3 か所に同じ `0.1.0` が別々に書かれていて、ずれやすい。どの版を会社の PC に入れたか後から分かるようにする。 |
| D6 | 更新と戻し方（自動更新を入れるか、手で入れ替えるか） | 自動更新は入れず、手で入れ替える。前の版は Releases に残し、戻すときは前の版を入れ直す | 自動更新は外部への定期的な通信と、更新用の署名鍵の管理を増やす。「外に何も送らない」方針と、利用者 1 人という規模に合わせた。 |
| D7 | リリース前とインストール後の確認 | タグの前：`main` の CI がすべて緑。入れた後：起動する／プロファイルとリージョンを選ぶとロググループが並ぶ／取得して表示される、を会社の PC で目で確かめる | 配布は「入れて動いた」で完了とする。実際の AWS への接続は本人の手元・本人の認証情報で行い、CI からはしない（project.md の `NEVER`）。 |
| D8 | 会社の PC の条件（管理者権限の有無、インストールの社内承認、社内ネットワークのプロキシ） | 管理者権限がなければ `~/Applications` に入れる。社内承認とプロキシは人間に確かめる | 入れる場所と、AWS への接続が社内ネットワークで通るかに関わる。プロキシ対応が要るなら、それは配布の慣行ではなく requirements-analysis で扱う要件になる。 |

### ほかの 4 セクションで実際の設定とずれている点（基準の文面は変えていない）

| ID | 気づいたこと | 下書きの既定案 | 理由 |
|----|--------------|----------------|------|
| T1 | Testing Posture は「テストは `cargo test` で実行」だけを書いているが、実際は画面側の Vitest（`npm test`、23 ファイル）も CI でマージ前に実行している | 「画面側のテストは Vitest で実行し、CI でマージ前に実行する」の 1 行を足す | 実際の慣行を記録に合わせるため。足さなくても禁止にはならないが、後の段階のエージェントが画面側のテストを見落としやすい。 |
| T2 | Testing Posture の「スコープ `local-tool` には org.md のスコープ別下限がない」は前回の作業を前提にした書き方。今回の作業はスコープ `bugfix` で、org.md では「直す対象の回帰テストと、既存のテストが通り続けること」が下限になる | 括弧書きを「スコープごとの下限は org.md に従う」と一般化し、数値のカバレッジ下限を設けない点はそのまま残す | team.md は作業ごとではなくチーム全体の慣行なので、特定のスコープ名に結び付けないほうが今後ずれない。 |
| C1 | Code Style は Rust のツールだけを書いているが、CI では Prettier（`prettier --check`）、ESLint、`tsc --noEmit`、`npm audit --audit-level=high` も必須になっている | 画面側のフォーマッタ・リンタ・型検査・依存関係の監査を 1〜2 行で足す | 前回の未解決事項（「Web 技術ベースの GUI を選んだら Code Style の見直しが要る」）が、Tauri を選んだことで現実になっている。 |
| W1 | Way of Working は git の履歴と一致している（PR 番号付きのスクワッシュコミットのみ）。ただし GitHub 側の設定（`main` の保護、secret scanning の有効化）はリポジトリのファイルからは確かめられない | 変更なし。GitHub の設定が有効かどうかだけ人間に確認する | 慣行の文面にずれはない。設定の有無は人間しか確かめられない。 |

### ハードな制約の候補

| ID | 候補 | 下書きの既定案 | 理由 |
|----|------|----------------|------|
| R1 | 「署名用の証明書・秘密鍵（と公証用の Apple の認証情報）をリポジトリに置かない」を `NEVER` として加えるか | D4 で署名・公証をする場合だけ `NEVER` として尋ねる。しない場合は加えない | 人間が明言したハードな制約だけを記録する決まりのため、下書きでは `None.` のままにした。 |

## Assumptions & Open Questions

- [Q1] 会社の PC は macOS（Apple Silicon）。使うのは開発者本人 1 人だけ、という点は前回の記録からの推論で、今回の質問では確かめていない。
- [assumption] 今回の作業では Operation の段階はすべて SKIP のため、配布の手順（リリース用のワークフローと手順書）は code-generation と ci-pipeline で作る前提で考えた。
- 未解決（requirements-analysis に引き継ぐ）：会社の PC で管理者権限があるか。ない場合に `.app` をどこに入れるか（例：`~/Applications`）。
- 未解決（requirements-analysis に引き継ぐ）：会社でアプリのインストールに社内承認が要るか。また、未署名のアプリが会社のルールや MDM で禁止されていないか（禁止なら [Q4] の見直しが要る）。
- 未解決（requirements-analysis に引き継ぐ）：社内ネットワークのプロキシの有無と、アプリから AWS への接続がプロキシ越しに通るか。
- 未解決（requirements-analysis に引き継ぐ）：会社のネットワークから GitHub Releases に届くか。[Q3] で手で運ぶ予備の手順を選ばなかったため、届かない場合は配布の経路がなくなる。
- 未解決（requirements-analysis に引き継ぐ）：`identifier` が `dev.local-sights.app` と `.app` で終わっている。Tauri は macOS のバンドルの拡張子と紛らわしいとして勧めていない。ただし変えると、キャッシュや設定の置き場所（`~/Library/Caches/dev.local-sights.app/` など）も変わり、[Q2] で説明した「置き換えても設定とキャッシュが残る」に影響する。慣行の話ではないため、requirements-analysis で扱うかどうかを決める。
