# Unit of Work Story Map — CloudWatch Logs ローカルビューア

上流の成果物：`unit-of-work.md`（作業単位 U1〜U7）、`inception/requirements-analysis/requirements.md`（FR1〜FR8）。このワークフローではユーザーストーリーのステージを実施していないため、ストーリーの代わりに機能要件（FR）を作業単位に対応付ける。質問票の回答は `units-generation-questions.md`。アーキテクチャレビューの指摘（R-01、R-02、R-04、R-06）を受けた人間の判断を反映済み。

## 機能要件と作業単位の対応

| FR | 内容 | Unit ID | Directory |
|----|------|---------|-----------|
| FR1 | AWS 接続の切り替え（親。子 FR は主に U2、FR1.2 は U1、FR1.5 は U7） | U2 | u2-connection-selection |
| FR1.1 | プロファイル一覧 | U2 | u2-connection-selection |
| FR1.2 | 4 種の認証方式（SDK に任せる） | U1 | u1-walking-skeleton |
| FR1.3 | リージョン一覧と既定 | U2 | u2-connection-selection |
| FR1.4 | 接続変更でロググループ再取得 | U2 | u2-connection-selection |
| FR1.5 | 認証失敗時の文と次の行動 | U7 | u7-ui-polish |
| FR2 | ロググループの選択（親。子 FR はすべて U2） | U2 | u2-connection-selection |
| FR2.1 | 全ロググループの列挙 | U2 | u2-connection-selection |
| FR2.2 | 名前での絞り込み | U2 | u2-connection-selection |
| FR2.3 | 1 つだけ選ぶ | U2 | u2-connection-selection |
| FR3 | 時間範囲とタイムゾーン（親。子 FR は主に U4、FR3.1・FR3.5 は U1） | U4 | u4-time-range |
| FR3.1 | 絶対日時の入力（U1 は UTC のみ、U4 で仕上げ） | U1 | u1-walking-skeleton |
| FR3.2 | ローカル／UTC の切替 | U4 | u4-time-range |
| FR3.3 | 同じ瞬間を保つ | U4 | u4-time-range |
| FR3.4 | [Fetch] を押せる条件 | U4 | u4-time-range |
| FR3.5 | 終了日時の境界（U1 で作り U3・U6 も使う） | U1 | u1-walking-skeleton |
| FR3.6 | 夏時間の扱い | U4 | u4-time-range |
| FR4 | 取得と時刻順表示（親。子 FR は主に U3、FR4.3・FR4.4・FR4.6 は U1、FR4.9 は U7） | U3 | u3-fetch-robustness |
| FR4.1 | ストリームの列挙と取得 | U3 | u3-fetch-robustness |
| FR4.2 | 時刻によるストリームの選定 | U3 | u3-fetch-robustness |
| FR4.3 | 開始・終了時刻の指定 | U1 | u1-walking-skeleton |
| FR4.4 | ページ終端の判定 | U1 | u1-walking-skeleton |
| FR4.5 | スロットリング時の再試行 | U3 | u3-fetch-robustness |
| FR4.6 | 件数の上限なし | U1 | u1-walking-skeleton |
| FR4.7 | 逐次・時刻順の表示 | U3 | u3-fetch-robustness |
| FR4.8 | 取得中のロックと状態表示 | U3 | u3-fetch-robustness |
| FR4.9 | 取得中の終了確認（中断の経路は U3） | U7 | u7-ui-polish |
| FR4.10 | 部分失敗の表示 | U3 | u3-fetch-robustness |
| FR4.11 | 件数と 0 件の表示 | U3 | u3-fetch-robustness |
| FR5 | 長いメッセージの表示（親。子 FR は主に U7、FR5.1 は U1） | U7 | u7-ui-polish |
| FR5.1 | 1 行分だけの表示 | U1 | u1-walking-skeleton |
| FR5.2 | 行の直下に展開 | U7 | u7-ui-polish |
| FR5.3 | 全文のコピー | U7 | u7-ui-polish |
| FR5.4 | JSON は整形しない | U7 | u7-ui-polish |
| FR6 | 部分一致の絞り込み（親。子 FR はすべて U5） | U5 | u5-filter |
| FR6.1 | 部分一致 | U5 | u5-filter |
| FR6.2 | AWS を呼ばない | U5 | u5-filter |
| FR6.3 | 絞り込み後 / 全件 | U5 | u5-filter |
| FR6.4 | 再取得で引き継ぐ | U5 | u5-filter |
| FR6.5 | 大文字・小文字（Functional Design で決める） | U5 | u5-filter |
| FR7 | ディスクキャッシュ（親。子 FR はすべて U6） | U6 | u6-disk-cache |
| FR7.1 | 有効・無効（既定は無効） | U6 | u6-disk-cache |
| FR7.2 | 有効化時の注意表示 | U6 | u6-disk-cache |
| FR7.3 | 保存場所 | U6 | u6-disk-cache |
| FR7.4 | 範囲が含まれれば再利用 | U6 | u6-disk-cache |
| FR7.5 | 有効期限なし | U6 | u6-disk-cache |
| FR7.6 | 全削除 | U6 | u6-disk-cache |
| FR7.7 | 無効時は書かない | U6 | u6-disk-cache |
| FR7.8 | 期限が厳しいときは外せる | U6 | u6-disk-cache |
| FR7.9 | 失敗・中断時は書かない | U6 | u6-disk-cache |
| FR8 | エラーと入力不備の表示（親。子 FR は主に U7、FR8.3 は U1） | U7 | u7-ui-polish |
| FR8.1 | 何が起きたかと次の行動 | U7 | u7-ui-polish |
| FR8.2 | 色やアイコンだけにしない | U7 | u7-ui-polish |
| FR8.3 | 認証情報を出さない（U1 から全単位で守る） | U1 | u1-walking-skeleton |
## 複数の単位にまたがる要件

| FR / NFR | 主に担う単位 | ほかに関わる単位 | 内容 |
|----------|--------------|------------------|------|
| FR3.1 | U1 | U4 | U1 は UTC の日時だけを解釈し、U4 でタイムゾーン切替・検証を仕上げる |
| FR3.5 | U1 | U3、U4、U6 | 取得に使うミリ秒範囲の算出（終了はその秒の終わりまで）は U1 で作り、他の単位は同じものを使う（レビュー R-04） |
| FR4.9 | U7 | U3 | 終了確認の画面とルールは U7、取得の中断の経路は U3（ADR-008） |
| FR8.3、NFR5 | U1 | U2〜U7 | 秘密の認証情報とアクセスキー ID を出さないルールは、U1 で境界を作り、全単位で守る |
| NFR2 | U3 | U5 | 100 万件の保持と、表示範囲だけを取り寄せる仮想スクロールの一覧は U3、絞り込みは U5（レビュー R-01） |
| NFR12、NFR13 | U1 | U2〜U7 | 英日の文言の仕組みとキーボード操作の土台は U1、各画面の文言とキー操作は各単位、通しの見直しは U7（レビュー R-02） |
| NFR14 | U1 | U2〜U6 | AWS 呼び出しの trait と偽物によるテストは U1 で作り、各単位が使う |

## 各単位の中での実装の順

単位の中で、要件をどの順に実装するかの目安。単位どうしの優先度は決めない。

- **U1**：ワークスペースと画面側の土台（文言の仕組み・キーボード操作・画面側のチェック設定を含む）→ CloudWatchLogsGateway（GetLogEvents と偽物）→ EventFetcher のページ終端判定（FR4.3、FR4.4、FR4.6、テスト先行）→ UTC の日時の解釈とミリ秒範囲の算出（FR3.1、FR3.5、テスト先行）→ 最小版 FetchCoordinator と最小の AppSession・画面（FR5.1）→ 確認用プログラム
- **U2**：プロファイル・リージョン一覧（FR1.1、FR1.3）→ ロググループの列挙と絞り込み（FR2.1〜FR2.3）→ 接続変更時の取り直し（FR1.4）
- **U3**：ストリームの列挙と選定（FR4.1、FR4.2、テスト先行）→ 時刻順の保持と逐次追加（FR4.7、テスト先行）→ 表示範囲の取り寄せと仮想スクロールの一覧（NFR2）→ 再試行と部分失敗（FR4.5、FR4.10、テスト先行）→ 取得の束ね役と中断 → 取得中のロックと件数表示（FR4.8、FR4.11）
- **U4**：タイムゾーン変換・夏時間（FR3.3、FR3.6、テスト先行）→ TZ 切替（FR3.2）→ [Fetch] の条件（FR3.4）
- **U5**：部分一致と結果の保持（FR6.1〜FR6.3）→ 再取得時の引き継ぎ（FR6.4）→ 性能の確認（NFR1、NFR2）
- **U6**：保存場所と有効・無効（FR7.1〜FR7.3、FR7.7）→ 範囲の判定と再利用（FR7.4、FR7.5）→ 書き込み条件（FR7.9）→ 全削除と設定ダイアログ（FR7.6）
- **U7**：行の展開とコピー（FR5.2〜FR5.4）→ エラー文（FR1.5、FR8.1、FR8.2）→ 終了確認（FR4.9）→ ダークモード・最小ウィンドウ（NFR11）→ 英日の文言とキーボード操作の通しの見直し（NFR12、NFR13）

## 対応の確認

- すべての FR（FR1〜FR8 とその下位 46 件）が、ちょうど 1 つの主な単位に対応している。
- すべての単位に 1 つ以上の FR がある：U1（FR1.2、FR3.1、FR3.5、FR4.3、FR4.4、FR4.6、FR5.1、FR8.3）、U2（FR1、FR1.1、FR1.3、FR1.4、FR2 系）、U3（FR4 系の多く）、U4（FR3 系）、U5（FR6 系）、U6（FR7 系）、U7（FR1.5、FR4.9、FR5 系、FR8 系）。
