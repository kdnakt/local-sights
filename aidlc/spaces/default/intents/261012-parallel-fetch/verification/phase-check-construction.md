# Phase Check — Construction → Operation

**判定：PASS（利用者の判断による受け入れあり）**。作業単位はすべてビルド・テスト済みで、目標 89 件はすべて Met。CI のゲートは Build and Test のコマンドを確かめる。要件の対応表で直接の行がない 45 件は、間接の対応として利用者が受け入れた（`construction/ci-pipeline/ci-pipeline-questions.md` Q2 の回答 A）。

## 1. 作業単位のビルドとテスト

| 項目 | 結果 |
|------|------|
| 作業単位 | U1〜U5 のすべてが code-generation まで完了し、チェックポイントを承認済み |
| ビルド | 成功（`construction/build-and-test/test-results.md`） |
| テスト | Rust 548 件成功・失敗 0 件、画面 385 件成功・失敗 0 件、速度測定 5 件成功、確認用プログラム 3 件成功 |
| 品質目標 | 89 件すべて Met（`construction/build-and-test/build-and-test-summary.md` の Target Verification Matrix） |

## 2. code-generation の対応表

| 作業単位 | `traceability.json` | 状態が OK でない行 | 対象のファイル |
|----------|---------------------|--------------------|----------------|
| u1-concurrent-skeleton | あり | なし | すべてある |
| u2-call-pacing | あり | なし | すべてある |
| u3-give-up-rule | あり | なし | すべてある |
| u4-elapsed-time | あり | なし | すべてある |
| u5-cancel | あり | なし | すべてある |

レビューで残した Minor（U4・U5）は、作業単位のレビューで「次に回す」と決めたもので、この対応表の未解決の指摘ではない。一覧は `build-and-test-summary.md` の「残っている項目」にある。

## 3. 作業単位をまたぐ要件の対応表

| 項目 | 結果 |
|------|------|
| 数え上げた ID | 94（requirements.md の FR・NFR と stories.md の 3 段の AC） |
| 直接 OK | 49 |
| 間接 | 45：親の ID（FR1〜FR5 とその子、NFR1〜NFR10）、AC2.2.2・AC2.2.5（U1 で実装とテスト）、FR6〜FR6.4（PR #20 の下準備）、AC1.1.4（手元の確認で合格） |
| 直接の対応での判定 | FAIL（`construction/build-and-test/cross-unit-traceability.md`、指摘 BT-TR-1〜4） |
| 扱い | 45 件はいずれも子の ID・別の場所を通じて実装とテストまでたどれるため、利用者が間接の対応として受け入れた（ci-pipeline Q2=A）。code-generation の対応表は書き直さない |

## 4. CI のゲートと Build and Test のコマンド

| 項目 | 結果 |
|------|------|
| Build and Test のコマンドが CI のゲートにあるか | ある（`construction/ci-pipeline/quality-gates.md` の対応表）。確認用プログラムのテストはこのステージで足した |
| CI にないもの | 100 万件の一覧の速度測定と、実際の AWS での速さの比べ。どちらも意図して手元に残す（実行機の速さに左右される／CI から AWS に接続しない決まり） |
| この枝の PR の CI | まだ走っていない（PR は頼まれたときに作る） |

## 5. Operation への移り

この作業の実行計画では Operation の各ステージ（deployment-pipeline から feedback-optimization まで）を飛ばす。配布は team.md の Deployment どおり、`main` にマージした後に `v0.1.2` のタグを付けて行う。
