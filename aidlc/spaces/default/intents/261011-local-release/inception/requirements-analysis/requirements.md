# Requirements — Local Sights のリリース準備（会社 PC へのインストール）

## 意図の分析

利用者（開発者本人）は、作った CloudWatch Logs 閲覧アプリを、自分の会社の Mac に入れて日常的に使えるようにしたい [desc]。そのために、決まった手順でインストーラ（`.dmg`）を作り、取りに行ける場所に置き、迷わず入れ替えられる状態をつくることが目的である。

- 種類：既存アプリへの配布まわりの追加（ブラウンフィールド）。アプリの機能そのものは変えない。
- 範囲：アプリの設定（表示名・版）、リリース用の GitHub Actions ワークフロー、版番号の検査、インストール手順書。
- 配布の方針はチームの方針として確定済み（`team-practices.md` の `## Deployment`）。本書はその方針を、確かめられる要件に落とす。

## 機能要件

### FR1. アプリの表示名

- **FR1.1** アプリの表示名を `Local Sights` とする。インストール後の「アプリケーション」フォルダでの名前は `Local Sights.app`、Dock とメニューバーの名前も `Local Sights` になる [Q1]。
- **FR1.2** メインウィンドウのタイトルも `Local Sights` とする [Q1]。
- **FR1.3** 内部の識別子（`identifier`）は `dev.local-sights.app` のまま変えない [Q2]。設定やキャッシュの保存場所は今と同じになる。

受け入れ基準：
- Given `.dmg` から入れたアプリ、When 「アプリケーション」フォルダを開く、Then `Local Sights.app` という名前で表示される。
- Given アプリを起動した、When ウィンドウを見る、Then タイトルが `Local Sights` である。
- Given 識別子の設定、When リリース用にビルドする、Then 識別子は `dev.local-sights.app` のままである。

### FR2. 版番号の一致

- **FR2.1** 版番号は `Cargo.toml`（ワークスペースの `workspace.package.version`）・`package.json` の `version`・`src-tauri/tauri.conf.json` の `version` の 3 か所で同じ値とする [team-practices]。
- **FR2.2** 3 か所の版がそろっているか、またリリースのタグ（`vX.Y.Z`）と一致しているかを調べる仕組みを用意する。一致しないときは、どのファイルがどの値なのかを示して失敗する [team-practices]。
- **FR2.3** タグの形は `v` + セマンティックバージョン（`vMAJOR.MINOR.PATCH`）とする。この形でないタグ（例：`v1.2`、`1.2.3`、`release-1`）ではリリースのビルドを始めない [team-practices]。
- **FR2.4** 最初のリリースは `v0.1.0` とする（今の 3 か所の版は `0.1.0` でそろっている）[team-practices]。
- **FR2.5** 版の検査は、通常の CI（プルリクエストと `main` への push）でも 3 か所がそろっているかを調べ、ずれをマージ前に見つける [assumption：タグを付けてから気づくより早く直せるため]。

受け入れ基準：
- Given 3 か所の版が `0.1.0`、When タグ `v0.1.0` でリリースのワークフローが動く、Then 版の検査は成功する。
- Given `package.json` だけ `0.1.1`、When 検査を実行する、Then `package.json` と値を示して失敗する。
- Given 3 か所が `0.1.0`、When タグ `v0.2.0` でリリースのワークフローが動く、Then タグと版が合わないことを示してビルドの前に失敗し、リリースは作られない。
- Given タグ `v1.2`、When タグを push する、Then リリースのビルドは始まらない（または形が違うことを示して失敗する）。

### FR3. タグを合図にしたビルドと公開

- **FR3.1** `vX.Y.Z` のタグが push されると、GitHub Actions の macOS（Apple Silicon）ランナーでアプリをビルドし、`.dmg` を作る [team-practices]。
- **FR3.2** ビルドに成功したら、そのタグの GitHub Release を作り、`.dmg` を添付して、下書きにせずすぐに公開する [Q5][Q5-1]。
- **FR3.3** ビルドや検査が途中で失敗したときは、Release を作らない（中身のない Release や、壊れた添付ファイルを公開しない）[assumption：公開してしまうと会社 PC から誤って取得する恐れがあるため]。
- **FR3.4** タグの付いたコミットが `main` に含まれていないときは、リリースのビルドを始めない [team-practices：リリースは `main` 上のタグで区切る]。
- **FR3.5** 同じタグでワークフローを再実行したときに、既にある Release や添付ファイルを黙って二重に作らない。既にある場合は、添付ファイルを置き換えるか、理由を示して失敗するかのどちらかに決め、手順書に書く [assumption：ネットワークの一時的な失敗で再実行することがあるため]。
- **FR3.6** Release には、どのコミットから作ったかが分かる情報（タグとコミット）を含める。リリースノートは GitHub の自動生成で足りる [assumption：利用者 1 人で、変更点の確認が目的のため]。
- **FR3.7** 前の版の Release と添付ファイルは消さずに残す。前の版に戻すときに使う [team-practices]。
- **FR3.8** 作られる `.dmg` は macOS（Apple Silicon、`aarch64`）用だけとする。Intel の Mac と Windows 向けは作らない [team-practices]。

受け入れ基準：
- Given `main` 上のコミットに `v0.1.0` を付けて push した、When ワークフローが成功する、Then GitHub Releases に公開済みの `v0.1.0` があり、Apple Silicon 用の `.dmg` が 1 つ添付されている。
- Given ビルドが失敗した、When ワークフローが終わる、Then `v0.1.0` の Release は作られていない。
- Given `main` に含まれないブランチ上のコミットにタグを付けた、When タグを push する、Then ビルドは始まらず、理由が分かる形で失敗する。
- Given `v0.1.0` の Release がある、When `v0.2.0` を公開する、Then `v0.1.0` の Release と `.dmg` は残っている。

### FR4. 署名なしのアプリとして配る

- **FR4.1** Apple の署名・公証は行わない。Apple の証明書や認証情報は使わない [team-practices]。
- **FR4.2** 手順書に、初回起動時に出る警告（「開発元を確認できません」など）の開き方を書く。macOS の版によって手順が違う場合は、その違いも書く [team-practices]。
- **FR4.3** 手順書に、会社のルール（MDM で未署名のアプリの起動が禁止されていないか、インストールの前に申請が要るか）を、入れる前に自分で確かめる手順として書く。未署名のアプリが禁止されていると分かったら、署名・公証の方針を見直す [Q4]。

受け入れ基準：
- Given ダウンロードした `.dmg` から入れたアプリ、When 初めて起動して警告が出る、Then 手順書のとおりに操作すると起動できる。
- Given 手順書、When インストールの前の確認の節を読む、Then 会社のルールで確かめることと、禁止だった場合にどうするかが書かれている。

### FR5. インストールと更新・戻し方の手順書

- **FR5.1** README（または README から案内する手順書）に、会社 PC へのインストール手順を書く：GitHub Releases から `.dmg` を取得する → 開く → `Local Sights.app` をアプリのフォルダにドラッグする → 初回起動の警告を開く [team-practices]。
- **FR5.2** 管理者権限があるかどうか分からないため、`/Applications` に入れる方法と、管理者権限なしで `~/Applications` に入れる方法の両方を書く [Q3]。
- **FR5.3** 更新の手順を書く：新しい版の `.dmg` を取得し、`.app` をドラッグして置き換える。設定やキャッシュは `.app` の外（識別子 `dev.local-sights.app` の場所）にあるため残ることを書く [team-practices][Q2]。
- **FR5.4** 前の版に戻す手順を書く：GitHub Releases から前の版の `.dmg` を取得して、同じ手順で入れ直す [team-practices]。
- **FR5.5** 今入っている版の確かめ方を書く（例：Finder の「情報を見る」や「このアプリについて」）[assumption：入れ替えた後にどの版か確かめるため]。
- **FR5.6** リリースを出す側の手順を書く：3 か所の版を上げる → プルリクエストを `main` にマージする → `main` の CI が緑であることを確かめる → `vX.Y.Z` のタグを付けて push する → Release に `.dmg` が公開されたことを確かめる [team-practices]。
- **FR5.7** これまでのソースからのインストール（`cargo install`）は、開発用の手順として残す [team-practices]。
- **FR5.8** AWS の認証情報の準備（`~/.aws/config` など）は既存の README の説明を使い、会社 PC でも同じ準備が要ることを書く。秘密の認証情報を手順書の例に書かない [project.md：Forbidden]。

受け入れ基準：
- Given 手順書、When 手元の Mac で上から順に操作する、Then `Local Sights.app` が入って起動する。
- Given 管理者権限がない、When 手順書の `~/Applications` の方法に従う、Then アプリが入って起動する。
- Given 古い版が入っている、When 更新の手順に従う、Then 新しい版に置き換わり、前に選んでいた設定やキャッシュが残っている。

## 非機能要件

- **NFR1 秘密情報**：リリースのワークフローは、GitHub が自動で発行するトークン（`GITHUB_TOKEN`）だけを使う。AWS の認証情報、Apple の証明書、その他の秘密情報を CI に置かない。リリースのワークフローから実際の AWS に接続しない [project.md：Forbidden]。
- **NFR2 権限の最小化**：Release を作るジョブだけに書き込み権限（`contents: write`）を与え、それ以外のジョブは読み取り権限のままにする。既存の CI（`permissions: contents: read`）は変えない [assumption：GitHub Actions の一般的な守り方]。
- **NFR3 再現性**：リリースのビルドはクリーンなチェックアウトから行い、`Cargo.lock` と `package-lock.json` に従って依存関係を入れる（`npm ci`、ロックファイルどおりのビルド）。手元の環境に頼らない [team-practices：Code Style]。
- **NFR4 品質の門**：リリースのワークフローでも、ビルドの前に既存のテスト（`cargo test`、Vitest）を実行するか、タグの付いたコミットで `main` の CI が成功していることを確かめる。テストが失敗した状態では公開しない [team-practices：Testing Posture]。
- **NFR5 外部への送信なし**：アプリに自動更新・テレメトリ・クラッシュレポートの仕組みを入れない [team-practices][project.md：Forbidden]。
- **NFR6 既存の CI と品質を保つ**：既存のテスト・`cargo fmt --check`・clippy・Prettier・ESLint・`tsc --noEmit`・`npm audit`・`cargo-deny` はすべて通ったままにする [team-practices]。
- **NFR7 ワークフローの所要時間**：リリースのワークフローは、タグの push から公開まで 30 分以内に終わることを目安とし、ジョブにタイムアウトを設ける（止まったままにならない）[assumption：macOS ランナーでの Tauri のビルドは通常十数分のため]。

## 制約

- 配布先は macOS（Apple Silicon）だけ [team-practices][Q1]。
- 配布の形は Tauri の `.dmg`。ビルドと公開は GitHub Actions と GitHub Releases で行う。このリポジトリは公開（public）である [team-practices]。
- 署名・公証はしない [team-practices]。
- 版はセマンティックバージョニング、`v0.1.0` から [team-practices]。
- 自動更新は入れず、手で入れ替える [team-practices]。
- `main` への直接 push はしない。この作業もブランチ → プルリクエスト → CI 緑 → スクワッシュマージで進める [team-practices：Way of Working]。
- 会社のネットワークからは GitHub と AWS に直接つながる（プロキシなし）。プロキシへの対応は作らない [Q6]。

## 前提

- [assumption] 会社 PC も手元の Mac と同じく macOS（Apple Silicon）で、`.dmg` から入れたアプリが動く。
- [Q4] 会社のルール（未署名のアプリ・インストールの申請）はまだ確かめていない。未署名のアプリでも起動できる前提で進め、確かめる手順を手順書に書く（FR4.3）。
- [Q7] 今回の作業は、GitHub Releases に `v0.1.0` の `.dmg` が公開された時点で完了とする。会社 PC への実際のインストールと動作確認は、今回の作業の外で本人が行う。
- [assumption] リリースのワークフローはタグの push で動くため、`v0.1.0` の Release が実際に公開されるのは、この作業のプルリクエストが `main` にマージされ、本人がタグを push した後になる。それまでは、ワークフローの構文と版の検査、手元でのビルド手順の確認までを確かめる。
- [assumption] 表示名を `Local Sights` に変えると `.dmg` のファイル名に空白が入る（例：`Local Sights_0.1.0_aarch64.dmg`）。GitHub Releases は添付ファイル名の空白を `.` に置き換えることがあるため、手順書ではファイル名を決め打ちせず「`aarch64` の `.dmg`」と案内する。
- [assumption] 識別子を変えないため、手元の Mac で開発中に作った設定やキャッシュは、同じ Mac に `.dmg` から入れた版でもそのまま使われる。

## 対象外

- Apple の署名・公証、Apple Developer Program への加入。
- 自動更新（Tauri の updater）。
- Intel の Mac、Windows、Linux 向けのインストーラ。
- プロキシ経由の接続への対応。
- 識別子（`identifier`）の変更。
- 会社 PC への実際のインストールと動作確認（本人が作業の外で行う）[Q7]。
- 手でファイルを運ぶ配布経路（会社のネットワークから GitHub に直接つながるため）[Q6]。
- Homebrew などのパッケージマネージャでの配布。
- アプリの機能の変更。

## 未解決の点

- チームの方針（`team.md` の `## Deployment`）は「GitHub Releases に下書きとして置く」となっているが、今回「CI がすぐ公開する」に決めた（Q5、Q5-1）。この方針の記述の直しは、この工程の最後の記録の手順で行う。
- 会社のルールで未署名のアプリが禁止されていた場合は、署名・公証の方針を見直す必要がある（FR4.3）。その場合は別の作業として扱う。
- 同じタグで再実行したときの扱い（添付ファイルを置き換えるか、失敗させるか）は FR3.5 のとおりどちらかに決める。どちらにするかはコード生成の計画で決め、計画の承認で確かめる。
- push の際に GitHub から Dependabot のセキュリティ警告が 1 件あると表示された。今回の作業の外だが、リリースの前に確かめておくのが望ましい。

## Sources

- Initial description: [desc] 「ざっくりできたから、このアプリをリリースする手順を整えたい。つまり、自分の会社のPCでインストールして使うための準備。」
- Workflow-selected scope: [scope] `bugfix`（プラン `local-release`）
- [team-practices] `aidlc/spaces/default/intents/261011-local-release/inception/practices-discovery/team-practices.md`（承認済みで `team.md` に反映済み）
- [Q1]〜[Q7]、[Q5-1] `aidlc/spaces/default/intents/261011-local-release/inception/requirements-analysis/requirements-analysis-questions.md`
- `aidlc/spaces/default/memory/project.md`（`## Forbidden`）
- `src-tauri/tauri.conf.json`、`Cargo.toml`、`package.json`、`README.md`、`.github/workflows/ci.yml`
- `aidlc/spaces/default/intents/261011-local-release/inception/practices-discovery/evidence.md`（要件定義への引き継ぎ事項）

## Assumptions & Open Questions

- [assumption] FR2.5、FR3.3、FR3.5、FR3.6、FR5.5、NFR2、NFR7 は、利用者に聞かずに置いた既定の振る舞いである。違っていれば承認のときに直す。
- [Q4] 会社のルールは未確認のまま進める。
- [Q7] 完了は GitHub Releases への公開まで。
- 上の「未解決の点」を参照。
