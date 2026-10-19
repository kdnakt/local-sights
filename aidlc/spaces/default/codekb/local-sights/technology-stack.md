# 技術の構成（Local Sights）

走査したコミット：`2760b13b8bc5b6daba36a6c283b3958c974a5a2d`（2026-10-17）。版は `Cargo.lock` と `package-lock.json` に固定されたもの（宣言の範囲はかっこ内）。

## 言語と実行環境

| 種類 | 中身 | 根拠 |
|------|------|------|
| Rust | edition 2024、rust-version 1.85 以上（stable） | `Cargo.toml` の `[workspace.package]` |
| TypeScript | 6.0.3（`tsc --noEmit` で型検査） | `package.json`、`tsconfig.json` |
| Node.js | 22（CI と開発） | `.github/workflows/ci.yml`、`README.md` |
| 実行する OS | macOS（Apple Silicon）、Windows 11（x64）。CI は Linux・macOS・Windows で組み立てて試す | team.md Deployment、`ci.yml` |
| WebView | macOS は WKWebView、Windows は WebView2（インストーラが取得する `downloadBootstrapper`）、Linux（CI だけ）は WebKitGTK 4.1 | `tauri.conf.json`、`ci.yml` |

## 枠組みとライブラリ（Rust）

| 名前 | 版 | 使いどころ |
|------|----|------------|
| tauri | 2.12.3（`2`） | デスクトップアプリ、コマンドとイベント、パスの解決、メニュー |
| tauri-build | 2.7.2（`2`） | ビルド時の生成 |
| aws-config | 1.12.0（`1`） | 認証と接続先の解決（SDK の既定の連鎖） |
| aws-sdk-cloudwatchlogs | 1.156.0（`1`、default-features 無し、`default-https-client`・`rt-tokio`） | 読み取り 3 操作。古い rustls 0.21 を引く `rustls` 機能を外している |
| aws-credential-types | `1` | 認証情報の型（扱うだけで保存しない） |
| tokio | 1.53.2（`rt-multi-thread`・`macros`・`sync`、中核は `time`） | 非同期実行、待ち、停止信号、別スレッド |
| futures-util | 0.3.34（`std` だけ） | 同時取得の `FuturesUnordered` |
| serde / serde_json | 1.0.229 / 1.0.152 | 画面への JSON、キャッシュの JSON Lines、設定ファイル |
| chrono | 0.4.45（`std`・`clock`） | 時刻 |
| chrono-tz | 0.10.4 | IANA のタイムゾーン（夏時間） |
| iana-time-zone | 0.1.65 | OS のタイムゾーン名 |
| regex | 1.13.1 | 失敗の詳細の伏せ字 |
| sha2 | 0.11.0 | キャッシュのファイル名 |
| thiserror | 2.0.21 | エラー型 |
| uuid | 1.28.0（`v4`） | セッション ID など |
| fastrand | 2.5.0 | 再試行の待ちの揺らぎ |
| home | 0.5.12 | ホームフォルダ（AWS の設定ファイルの場所） |
| libc | 0.2.190（`cfg(unix)` だけ） | AWS CLI のプロセスグループの停止（`killpg`） |
| windows-sys | `0.61`（`cfg(windows)` だけ、Tauri が組み立てる版） | ジョブオブジェクト、`ShellExecuteW` |
| tempfile | 3.27.0（開発時だけ） | テストの一時フォルダ |
| aws-smithy-runtime-api / aws-smithy-types | `1`（開発時だけ） | SDK のエラーを作るテスト |

## 枠組みとライブラリ（画面）

| 名前 | 版 | 使いどころ |
|------|----|------------|
| React / react-dom | 19.3.0 | 画面 |
| @cloudscape-design/components | 3.0.1396 | AWS コンソールと同じ部品（TextFilter、Table、DatePicker、Modal など） |
| @cloudscape-design/design-tokens / global-styles | `^3.0.114` / `^1.0.71` | 色と明暗 |
| @tauri-apps/api | 2.12.2 | `invoke` と `listen` |
| @tauri-apps/cli | 2.12.1 | 開発とビルド |
| Vite | 8.3.4 | 開発サーバー（1420 番）とビルド（`es2022`、`dist/`） |
| Vitest | 5.0.3（`@vitest/coverage-v8`） | 画面のテスト（jsdom）と行カバレッジ |
| Testing Library | dom 10.4、react 16.3、user-event 14.6、jest-dom 7.0 | 部品のテスト |
| ESLint | 10.12.0（typescript-eslint 8、react-hooks 7、react-refresh 0.5） | 画面の lint |
| Prettier | 3.9.10 | 画面の整形（`printWidth` 100） |
| @resvg/resvg-js | 2.6 | アイコンの書き出し |

## ビルド・テスト・配布の道具

| 道具 | 使いどころ |
|------|------------|
| Cargo（ワークスペース） | 中核とアプリの組み立てとテスト |
| rustfmt、clippy | 整形と lint（CI、clippy の警告はエラーにしない） |
| cargo-llvm-cov 0.9.1 | 中核の行カバレッジ（80% 未満で失敗） |
| cargo-deny | 脆弱性・ライセンス・出どころの確認（`deny.toml`、実際に組み立てる機能だけ） |
| npm、`npm audit --audit-level=high` | 画面の依存と監査 |
| Tauri の bundler | macOS は `.dmg`（中に `.app`、アドホック署名）、Windows は NSIS（`currentUser`、管理者の権限なし） |
| GitHub Actions | CI と Release（`v*` タグ） |
| Dependabot | cargo・npm・Actions を毎週 |

## IB-11 から見た技術の手持ち

- JSON の読み取りは、中核がすでに `serde_json` を依存に持つ（キャッシュ用）。新しい依存なしにメッセージの JSON を読める。
- 正規表現は `regex` 1 が中核にある（今は伏せ字にだけ使う）。Logs Insights の `like /…/` などに使うなら、同じ版で足りる。
- クエリの構文解析のための依存（パーサーのライブラリ）は今は無い。足すなら cargo-deny のライセンス許可リスト（MIT・Apache-2.0 など）と team.md の依存の決まりに合わせる。
- 画面の入力欄は Cloudscape の部品だけで作っている。複数行の入力には `Textarea` などが使える。補完や色分け（Should の B8・B9）に使えそうな Cloudscape の `CodeEditor` は、外部のエディタ（Ace）を別に読み込む前提の部品で、今は使っておらず依存にも無い（採否は後のステージで確かめる）。
