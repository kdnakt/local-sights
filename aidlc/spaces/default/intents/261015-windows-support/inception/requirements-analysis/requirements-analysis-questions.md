# Requirements Analysis Questions — Windows 対応（IB-16）

意図の整理（`ideation/intent-capture/intent-statement.md`）とチームの決まりの見直し（`inception/practices-discovery/team-practices.md`）で決まったことは聞き直しません。ここでは、Windows で動かすときにコードと配布物のふるまいを決めるのに要ることだけを聞きます。

## Q1. Windows で、アプリは AWS CLI（`aws.exe`）をどこで探しますか？

背景：SSO ログインのボタンは AWS CLI を起動します。macOS では、Finder や Dock から起動したアプリはターミナルの `PATH` を受け継がないため、決まった 2 か所（Homebrew と公式インストーラの場所）だけを探しています。Windows では、スタートメニューから起動したアプリも利用者の `PATH` を受け継ぎます。

A. 公式インストーラの場所（`C:\Program Files\Amazon\AWSCLIV2\aws.exe`）を先に探し、なければ `PATH` から探す
B. 公式インストーラの場所だけを探す（`PATH` は使わない）
C. `PATH` だけから探す
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q2. Windows 版のメニューとキーボード操作はどうしますか？

背景：今のアプリのメニュー（コピーや貼り付けなどの編集のショートカット）は macOS だけにあります。Windows では、画面の中の文字欄で Ctrl+C／Ctrl+V などの標準の操作がそのまま使えます。project.md では、見た目の細部を OS の慣習に合わせてよいかをここで決めることにしています。

A. Windows ではメニューバーを出さず、Ctrl+C／Ctrl+V／Ctrl+A などの標準の操作だけにする（OS の慣習に合わせる）
B. Windows にも macOS と同じ項目のメニューバーを出す
C. Not yet defined
X. Other (please specify)

[Answer]: A

## Q3. Windows で、キャッシュと設定のファイルを本人だけが読めるようにする扱いはどうしますか？

背景：macOS と Linux では、キャッシュと設定のフォルダとファイルを所有者だけが読み書きできる権限で作っています。Windows では今は何もしていません。利用者の `AppData` の下は、Windows の既定のアクセス権で、本人と管理者と SYSTEM だけが読めます。

A. Windows の既定のアクセス権（利用者の `AppData` の下）に任せ、アプリでは権限を変えない。その旨を手順書に書く
B. アプリがフォルダとファイルに本人だけのアクセス権を明示的に付ける
C. Not yet defined
X. Other (please specify)

[Answer]: A

## Q4. 手元の Windows 11 の PC で「スマート アプリ コントロール」はどうなっていますか？

背景：この機能が「オン」だと、署名のないアプリは SmartScreen の「実行」を押しても開けません。今回は署名をしないので、確かめておく必要があります（Windows セキュリティ → アプリとブラウザーの制御 → スマート アプリ コントロール で見られます）。

A. オフになっている
B. 評価モードになっている
C. オンになっている
D. 分からない（最初の Windows での確認のときに見る）
E. Not yet defined
X. Other (please specify)

[Answer]: D

## Q5. 画面を表示する部品（WebView2）は、インストーラにどう含めますか？

背景：Windows 版の画面は Microsoft Edge の WebView2 で表示します。Windows 11 には最初から入っています。

A. Tauri の既定のまま（WebView2 がない PC でだけ、インストール時にダウンロードして入れる）
B. WebView2 をインストーラに同梱する（インストーラが大きくなる）
C. 何もしない（WebView2 がない PC では動かない）
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q6. Windows 版の手動の画面確認の項目はどこまで用意しますか？

背景：README の手動の画面確認は、macOS の操作（Cmd+Q、Dock、VoiceOver、アクティビティモニタ）を前提にした項目が多くあります。

A. macOS の項目に対応する Windows の項目（Alt+F4、タスクバー、ナレーター、タスク マネージャーなど）をすべて用意する
B. 取得・表示・SSO ログイン・中断・キャッシュなど、機能の項目だけを用意する（OS 固有の操作の項目は省く）
C. Not yet defined
X. Other (please specify)

[Answer]: A

## Q7. この作業を PR にどう分けますか？

背景：チームの決まりでは、最初の作業単位で「Windows 11 の PC にインストーラで入れたアプリで、取得して結果を表示する」ところまでの薄い一本を作り、確かめてから先に進みます。

A. 3 つに分ける：(1) Windows でビルド・テストが通り、インストーラで入れて取得・表示できる薄い一本と CI、(2) SSO ログイン・キャッシュなど残りの機能をそろえる、(3) リリースのワークフローと手順書
B. 2 つに分ける：(1) 薄い一本と CI、(2) 残りの機能とリリースと手順書
C. 1 つの PR にまとめる
D. Not yet defined
X. Other (please specify)

[Answer]: C
