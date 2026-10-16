# Requirements — Windows 対応（IB-16）

上流：`ideation/intent-capture/intent-statement.md`（意図）、`inception/practices-discovery/team-practices.md`（チームの決まり、承認済み）。回答は `requirements-analysis-questions.md` の Q1〜Q7 を指す。

## Intent analysis

- **目的：** 会社の Windows の PC でも、macOS 版と同じようにこのアプリを使えるようにする。Windows 版のインストーラを GitHub Releases から配る（意図 Q1・Q3）。
- **成功の目安：** Windows 11（x64）の PC で、GitHub Releases から取得したインストーラで入れたアプリが、macOS 版でできることをすべてできる。取得・表示・絞り込み・キャッシュ・SSO ログイン・中断・経過時間の表示が含まれる（意図 Q3、project.md の解釈）。開発者の手元の Windows 11 の PC で、本人の AWS 認証情報を使って確かめる（意図 Q4）。
- **種類と規模：** 既存のアプリを別の OS に移す作業。画面の機能は増やさない。変更が入るのは次のところ。
  - OS ごとに分かれる境界のコード：AWS CLI の起動と停止、ブラウザーを開く処理、アプリのメニュー、ファイルの権限
  - CI とリリースのワークフロー
  - 手順書
- **進め方：** 1 つの PR にまとめる（Q7）。ただしチームの決まり（Walking Skeleton）に従い、PR の中で最初に薄い一本を作る。薄い一本は「Windows でビルド・テストが通り、CI で作ったインストーラで入れたアプリで取得・表示できる」ところまで。開発者の Windows の PC で確かめ、骨組みのチェックポイントの承認を得てから、残りの機能・リリース・手順書を足す。

## Functional requirements

### FR1. Windows でのビルドと CI

- **FR1.1** CI の Tauri アプリのジョブ（今は `ubuntu-latest` と `macos-latest`）に `windows-latest` を加える。clippy・アプリのビルド・アプリのテストが Windows で通る。
- **FR1.2** ライブラリ（`local-sights-core`）のテスト（`cargo test`）も Windows のランナーで実行する。カバレッジの測定と画面側の Vitest は、今どおり Linux のランナーで行う（team.md Testing Posture）。
- **FR1.3** `Tauri app (windows-latest)` を、同じ PR の中で `main` のルールセットの必須の状態確認に加える（team.md Way of Working）。
- **FR1.4** `.gitattributes` でリポジトリのテキストファイルを LF に固定する。Windows で取り出しても、`cargo fmt --check` と `prettier --check` が Linux と同じ結果になる（team.md Code Style）。
- **FR1.5** 既存の Linux と macOS のジョブは、名前も中身も変えない。どちらも引き続き通る。

### FR2. Windows のインストーラと配布

- **FR2.1** Windows の配布物は NSIS のインストーラ（`-setup.exe`）とする。管理者の権限なしで、利用者ごとに入れられる（team.md Deployment）。
- **FR2.2** WebView2 は Tauri の既定の扱いにする。WebView2 がない PC でだけ、インストール時にダウンロードして入れる（Q5）。
- **FR2.3** `main` に `vX.Y.Z` のタグを付けると、リリースのワークフローが次の順に動く。
  1. テストを流す。
  2. macOS のランナーで `.dmg` を、Windows のランナーで `-setup.exe` を作る。
  3. 両方がそろったときだけ、1 つの Release として下書きにせず公開する。どちらかが作れなければ、何も公開しない（team.md Deployment）。
- **FR2.4** 手動の「Release dry run」で、`.dmg` と `-setup.exe` の作成だけを試せる。タグも公開もしない。
- **FR2.5** 版の一致の検査（5 つのファイルとタグ）は今どおり行う。Windows の配布物の名前にも同じ版の番号が入る。
- **FR2.6** macOS の配布物の作り方と検査は変えない。アドホック署名と、公開前の署名の検証（`scripts/release/signing.ts`）も、Windows の手順を足したワークフローで今どおり動く。
- **FR2.7** 新しい版のインストーラを上書きで実行すると、更新できる。設定とキャッシュ（利用者の `AppData` の下）は残る。
- **FR2.8** 前の版のインストーラを実行すると、前の版に戻せる。インストーラが前の版への入れ替えを拒まない（team.md Deployment の戻し方）。
- **FR2.9** 5 つのファイル（`Cargo.toml`・`Cargo.lock`・`package.json`・`package-lock.json`・`src-tauri/tauri.conf.json`）の版を `0.2.0` にする。次のタグは `v0.2.0` とし、`v0.1.2` は付けない（project.md Corrections）。

### FR3. Windows でも macOS 版と同じに動くこと

- **FR3.1** プロファイルとリージョンの選択、ロググループの一覧、時間範囲の指定、取得、表示、絞り込み、行の展開、ディスクキャッシュ、取得の中断、経過時間の表示が、Windows でも macOS と同じに動く（意図 Q3）。
- **FR3.2** AWS の設定ファイルは、Windows では利用者のホームの `.aws\config` と `.aws\credentials` から読む。環境変数（`AWS_CONFIG_FILE`、`AWS_SHARED_CREDENTIALS_FILE`、`AWS_PROFILE`、`AWS_REGION` など）による指定も、今どおり効く。
- **FR3.3** 設定とキャッシュは、Windows では OS が決めるアプリの置き場所（利用者の `AppData` の下）に置く。画面の設定で表示されるキャッシュの場所も、その実際の場所になる。
- **FR3.4** 取得中にウィンドウを閉じたとき（閉じるボタン、Alt+F4、タスクバーの「ウィンドウを閉じる」）は、macOS と同じ確認ダイアログを出す。取得していないときは、すぐに閉じる。
- **FR3.5** Windows ではメニューバーを出さない。画面の入力欄と展開したメッセージでは、Windows の標準の操作が使える（Q2）。
  - Ctrl+C・Ctrl+V・Ctrl+X・Ctrl+A：コピー、貼り付け、切り取り、すべて選択
  - Ctrl+Z：元に戻す
- **FR3.6** macOS のアプリのメニュー（編集の項目と、自前の終了の項目）は、今どおり macOS だけで動く。

### FR4. Windows での SSO ログイン

- **FR4.1** AWS CLI は、まず公式インストーラの場所（`C:\Program Files\Amazon\AWSCLIV2\aws.exe`）を探す。なければ、アプリが受け継いだ `PATH` から `aws.exe` を探す（Q1）。macOS の探し方（決まった 2 か所だけ、`PATH` は使わない）は変えない。
- **FR4.2** CLI はシェルを通さずに起動する。今どおり、決まった引数だけを渡し、標準入力を閉じる。コマンドプロンプトの黒い窓は出さない。
- **FR4.3** CLI の出力の扱いは macOS と同じにする。行ごとに読み、許可した URL とコードだけを画面に渡す。それ以外は保持もログ出力もしない。
- **FR4.4** 次のときは、CLI と、CLI が起こした子のプロセスをまとめて止める。何も残さない。CLI が自分で終わったときは何もしない（ブラウザーを閉じないため）。
  - 利用者の中断
  - 5 分の時間切れ
  - アプリの終了
- **FR4.5** サインインの URL は、Windows の既定のブラウザーで開く。シェルを通さない。今の試行の URL で、許可した宛先に当てはまるときだけ開く（今の許可の検査を変えない）。
- **FR4.6** CLI が見つからないとき、起動できないとき、時間切れのときは、macOS と同じ種類のエラーを画面に出す。CLI が見つからないときの案内には、Windows での AWS CLI v2 の入れ方を含める。

### FR5. 手順書と手動の確認

- **FR5.1** `docs/install.md` に Windows の節を足す。macOS の節とは分ける（team.md Deployment）。Windows の節には次を書く。
  - インストーラの取得と入れ方
  - SmartScreen の警告の開き方（「詳細情報」→「実行」）
  - スマート アプリ コントロールがオンだと開けないこと
  - AWS CLI v2 の入れ方と、アプリが探す場所
  - 設定とキャッシュの場所
  - 更新・前の版への戻し方・アンインストール
- **FR5.2** README の手動の画面確認に、Windows の項目を足す。macOS の各項目に対応する Windows の項目をすべて用意する（Q6）。

  | macOS の項目 | Windows の項目 |
  |---|---|
  | Cmd+Q とアプリのメニューの終了 | Alt+F4、閉じるボタン |
  | Dock | タスクバー |
  | Cmd+Tab | Alt+Tab |
  | VoiceOver | ナレーター |
  | アクティビティモニタ | タスク マネージャー |
  | Cmd+C／Cmd+V | Ctrl+C／Ctrl+V |
  | Dock とアイコン | タスクバーとスタートメニューのアイコン |

- **FR5.3** `docs/releasing.md` に、Windows のインストーラも作られて公開されること、dry run で両方を試すことを書く。

## Non-functional requirements

- **NFR1（macOS を壊さない）** 既存のテストは、Linux・macOS・Windows のすべてで通る。共通のコードと OS ごとに分かれるコードを触るため、薄い一本の手元の確認（実際の AWS からの取得と、画面を目で見る確認）は、macOS と Windows の両方で行う。macOS で前と同じに動くことも確かめる（team.md Walking Skeleton）。
- **NFR2（テスト）**
  - OS ごとに分かれるコード（`cfg(windows)`、`cfg(unix)` など）のテストは、その OS のランナーで実行される。テストで使う偽の AWS CLI は Windows 向けにも用意する。
  - 行カバレッジは Linux のランナーで測り、80% を下回らない（team.md Testing Posture）。
  - 自動テストと CI は、実際の AWS につながない。
- **NFR3（セキュリティ）**
  - project.md の Forbidden はそのまま守る。
    - 呼ぶのは CloudWatch Logs の読み取りの 3 つの API だけ。
    - 秘密の認証情報とトークンを書き込まない。
    - テレメトリやクラッシュレポートを外部に送らない。
  - Windows では、キャッシュと設定のアクセス権は OS の既定に任せる。利用者の `AppData` の下は、本人・管理者・SYSTEM だけが読める。アプリでは権限を変えず、その旨を手順書に書く（Q3）。
- **NFR4（対象の環境）** 対象は、Microsoft がサポートしている版の Windows 11（x64）。開発者の Windows の PC で動くことを確かめる。Windows 10 と arm64 の Windows では、動くことを確かめない（team.md Deployment）。
- **NFR5（パスと名前）** 次の場合でも、設定・キャッシュ・AWS の設定ファイルを正しく扱う。
  - 利用者のフォルダの名前に空白や日本語が入っている
  - AWS CLI の場所に空白が入っている（`Program Files`）
- **NFR6（アクセシビリティ）** VoiceOver で確かめている読み上げ（経過時間の秒は読まないこと、警告を 1 回だけ読むことなど）を、Windows ではナレーターで同じように確かめる。

## Constraints

- 配布は GitHub Releases だけで行う。Windows のコード署名、Microsoft Store、自動更新はしない（意図 Q6、team.md Deployment）。
- macOS と Windows の配布物は、同じタグ・同じ版の番号で同時に出す（team.md Deployment）。
- 呼ぶ AWS の API は増やさない。SSO ログインは、今どおり利用者の操作で AWS CLI の `aws sso login` を起動するだけにする。アプリ自身は SSO の API を呼ばず、トークンを読み書きしない（project.md）。
- 開発は Linux のクラウド環境と開発者の Mac で行う。Windows での実機の確認は、開発者の手元の Windows 11 の PC で行う（意図 Q4）。

## Assumptions

- **A1** 利用者のフォルダの `AppData` の既定のアクセス権は、本人・管理者・SYSTEM だけに読み取りを許す。会社の PC でもこの既定が変えられていないとみなす（Q3）。
- **A2** Windows 11 には WebView2 が最初から入っている。入っていない PC では、インストーラが入れる（Q5）。
- **A3** スタートメニューから起動したアプリは、利用者の `PATH` を受け継ぐ。AWS CLI v2 を公式インストーラ以外で入れた場合も、`PATH` に通っていれば見つかる（Q1）。
- **A4** 手元の PC のスマート アプリ コントロールの状態は分かっていない。最初の Windows での確認のときに見る。オンで開けなければ、team.md Deployment の「未署名のアプリが使えないと分かったら見直す」に当たる（Q4）。

## Out of scope

- Windows のコード署名、Microsoft Store での配布、自動更新（意図 Q6）。
- Intel の Mac・Linux・Windows 10・arm64 の Windows への配布（意図 Q5・Q6）。
- Windows でのメニューバー（Q2）。
- 画面の新しい機能。Logs Insights 互換のクエリ（IB-11）は次の作業で扱う（依頼文）。
- Windows のキャッシュと設定に、アプリが明示的にアクセス権を付けること（Q3）。

## Open questions

- **OQ1** 手元の Windows 11 の PC のスマート アプリ コントロールの状態は、最初の Windows での確認のときに確かめる（Q4、A4）。
- **OQ2** NSIS で前の版に戻すときに入れ替えを拒まないための Tauri の設定（`allowDowngrades` など）は、コード生成の計画で確かめる（FR2.8）。
- **OQ3** `.gitattributes` を入れたときに、既存のファイルの改行がそろえ直されて差分が出るかは、コード生成の計画で確かめる（FR1.4）。
- **OQ4** README の Windows の項目のうち、macOS の Dock の「終了」（止められない終了、既知の制約）に当たる Windows の操作（タスク マネージャーでの終了など）をどう扱うかは、コード生成の計画で決める（FR5.2）。
