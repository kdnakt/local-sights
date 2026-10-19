# Build and Test Summary — Logs Insights 互換のクエリ（IB-11）

上流：U1〜U4 の `code-generation-plan.md`（Testing Contract `sha256:d60a0643…`）・`unit-test-instructions.md`・`code-summary.md`、`inception/requirements-analysis/requirements.md`。この計画（refactor）には NFR Requirements と NFR Design の段がないため、測れる目標は要件の NFR・SM と、team.md・計画の Testing Contract から集めた。

## 1. ビルドの状態

すべて成功（Linux のコンテナ）。fmt・clippy・tsc・eslint・prettier・vite build・Tauri アプリのビルド・npm audit・cargo-deny が通った。詳しくは `test-results.md` の 1。

## 2. 作ったテストの手順書

| 種類 | ファイル | 中身 |
|---|---|---|
| ビルド | `build-instructions.md` | 依存関係、ビルド、CI と同じ確かめ |
| 結合 | `integration-test-instructions.md` | 作業単位をまたぐ境目と、それを確かめるテスト |
| 速さ | `performance-test-instructions.md` | NFR1（人と決めた数値）・NFR2・今の速さの基準 |
| セキュリティ | `security-test-instructions.md` | NFR4・NFR5・NFR12、依存関係 |

単体テストは各作業単位の `unit-test-instructions.md` のとおり。

## 3. 作業単位ごとのカバレッジの見込み

| 作業単位 | 中身 | テスト |
|---|---|---|
| U1 filter の薄い一本 | 読み取り・評価・QueryStore・QueryRunner・画面の骨組み | 中核の単体と画面のテスト。ライブラリ全体で 80% 以上 |
| U2 クエリの言語 | JSON の項目・fields・sort・limit・エラーの印・書く手助け | 同上 |
| U3 取得済みの範囲 | 取り足し・キャッシュ・50 個のロググループ・入れ子の上限 | 同上 |
| U4 互換の確認用プログラム | 確認用プログラムとスクリプト | 例のテスト 42 件（カバレッジの対象の外）。比べ方の公開関数はすべてテストあり |

実際の値：ライブラリ 96.52%、画面 89.88%。

## Target Verification Matrix

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| T-NFR1-100K | requirements.md NFR1、Build and Test の答え（C） | 10 万件の評価が 10 秒以内 | 93.6 ms | test-results.md の 4 | build-and-test | Met |
| T-NFR1-1M | requirements.md NFR1、Build and Test の答え（C） | 100 万件の評価が 100 秒以内 | 1.75 s（50 個のロググループで 2.44 s） | test-results.md の 4 | build-and-test | Met |
| T-NFR2 | requirements.md NFR2 | 100 万件でも落ちずに結果が出て、評価中も止められる | 落ちずに結果。最も長い 1 区切り 0.97 s | test-results.md の 4 | build-and-test | Met |
| T-BASE-SPEED | team.md Testing Posture、C5 | 今の速さのテストが通る | 5 件とも通った | test-results.md の 4 | build-and-test | Met |
| T-NFR8-CORE | NFR8、Testing Contract | ライブラリの行カバレッジ 80% 以上 | 96.52% | test-results.md の 3 | build-and-test | Met |
| T-NFR8-UI | NFR8、Testing Contract | 画面の行カバレッジ 80% 以上 | 89.88% | test-results.md の 3 | build-and-test | Met |
| T-NFR8-OS | NFR8・NFR6、team.md | Rust のテストが Linux・macOS・Windows で通る | 3 つの OS で CI が成功 | test-results.md の 8（PR kdnakt/local-sights#34） | build-and-test | Met |
| T-C5 | C5 | 既存のテストがすべて通る | 通った（Linux） | test-results.md の 2 | build-and-test | Met |
| T-STYLE | team.md Code Style | fmt・clippy・eslint・prettier・tsc が通る | 通った | test-results.md の 1 | build-and-test | Met |
| T-DEPS | team.md Code Style | npm audit と cargo-deny が通る | 通った | test-results.md の 1 | build-and-test | Met |
| T-NFR5 | NFR5 | 読み取りの 3 つの API だけ | 新しい API なし。テストが通った | security-test-instructions.md | build-and-test | Met |
| T-NFR12 | NFR12 | 外部に何も送らない | 足した行に送信なし | security-test-instructions.md | build-and-test | Met |
| T-SM1 | SM1・NFR3、U4 BR3.3 | 互換を確かめるクエリでコンソールとアプリの結果が同じ | 23 本すべて Same（3 回目、Loop-back 2・3 のあと） | test-results.md の 11・13・15 | 開発者の手元 | Met |
| T-WS-MAC | team.md Walking Skeleton | macOS で実際の AWS で取得して結果が出る | 開発者の Mac でひととおり問題なし | test-results.md の 9 | 開発者の手元 | Met |
| T-WS-WIN | team.md Walking Skeleton | Windows 11 で同じ | 確かめていない（開発者の答えで飛ばした、2026-10-19） | test-results.md の 7 | 開発者の手元 | Unverified |
| T-ACE | U2 review R-04、FR9.2 | 本物の Ace で色分け・補完・エラーの印が出る | 開発者の Mac で問題なし | test-results.md の 9 | 開発者の手元 | Met |

## 4. 準備の具合

- **ビルドの準備**：できている。
- **テストの準備**：このコンテナで流せるものはすべて通った。
- **配る準備**：残る `Unverified` は T-WS-WIN（Windows 11 の手元の確かめ）だけで、開発者がこの作業では飛ばすと決めた。macOS・Windows の CI は PR kdnakt/local-sights#34 で通っている。IB-11 が終わるまで `v0.2.0` のタグは付けない（team.md Deployment）。`v0.2.0` を配る前に Windows 11 で確かめるのがよい。
- **Loop-back**：3 回（U4 のスクリプトの指摘、互換の確かめの違い 2 回）。記録は test-results.md の Loop-Back Log。上限に達した。

## 5. 分かっている限界と残り

- **手元で確かめること**：
  1. 互換の確かめ（U4）：`PROFILE=… REGION=… GROUP=CloudTrail START=… END=… OUT=~/insights-export ./scripts/compat/insights-export.sh` のあとに `cargo run -p local-sights-core --example compat_check -- --export-dir ~/insights-export`。結果の要約（ロググループの種類、期間の長さ、件数、結果の数、違いのあったクエリと原因の見立て）を残す。ログの中身は残さない。
  2. 薄い一本（macOS と Windows 11）：複数のロググループで実行して結果が出る、同じ期間の 2 回目は AWS から取得しない、期間を広げると足りない部分だけを取得する、キャッシュから読む、接続を変えると捨てる、ログの画面が前と同じ。
  3. 本物の Ace の目視。
  4. macOS のメモリ（`/usr/bin/time -l`）。
- **macOS・Windows の CI**：PR kdnakt/local-sights#34 で 7 つのジョブがすべて通った。
- **U4 のコードのレビューで残った指摘**（READY、助言）：
  - Major R-01：スクリプトが start-query の失敗をすべて「Logs Insights がクエリを断った」として書く。認証切れやアクセスの拒否でも比べに使われ、互換と誤って判定しうる。
  - Minor R-02：書き出しのフォルダを使い回すと、前回のファイルが残って比べられうる。
  - Minor R-03：start-query に `--limit 10000` を渡していない。
  - Minor R-04：CLI の標準エラーをクエリの ID に混ぜている。
  - Minor R-05：U4 の code-summary の書式の記述が古い（store.rs は U4 で直した）。
  - Minor R-06：スクリプトのテストは Linux の CI だけで流れ、macOS の bash 3.2 では未確認。
  - R-01〜R-05 は Loop-back 1 で直した。R-06 は手元の互換の確かめ（macOS の bash 3.2）で動くことを確かめた。
- **U3 の残り**（Minor）：`read_overlaps` がキャッシュのファイルを丸ごと読む、接続を変えるまで保持が増え続ける、`holdings_epoch` を使っていない。
- **仮の決まり**（U2）：互換の確かめ（p01〜p12）でコンソールと合わせた。真偽値は 1／0、数として読めない文字列の大小は合わない、項目どうしの比べに対応し項目がないときは合わない、`limit 0` は 0 行。左辺の項目がないときの項目どうしの比べは、まだ仮の決まり。
- **レビューで持ち越した Minor**：`DEFAULT_RESULT_LIMIT` の説明と U2 の code-summary の部品の表に `limit 0` の古い記述が残る。U4 の確認用プログラムは `limit 0` を絞りなしと扱う（比べには影響しない）。`parse` の正規表現の中の `limit` はスクリプトがコマンドとみなす。
- **次の版の改善の候補（開発者の気づき、2026-10-18）**：コンソールは検索中に結果を少しずつ表示するが、このアプリは終わってから出す。開発者の答えで次の版に回した（`sort`・`limit` があると途中の結果は並びが変わる点も考える）。
- **今後の改善の候補（開発者の気づき、2026-10-18）**：リージョンを選ぶ欄に名前（東京、バージニア北部など）が出ず、検索もできない。前からある接続の画面のこと。
- **JSON の数**：CloudTrail には JSON の数の項目がほとんどなく、JSON の数の比べは確かめられないまま残る（U4 R-04）。

## 6. 結果

失敗：落ちたコマンドと `Not Met` の目標はない。`Unverified` は T-WS-WIN の 1 つで、開発者がこの作業では飛ばすと決めた（`test-results.md` の 7）。
