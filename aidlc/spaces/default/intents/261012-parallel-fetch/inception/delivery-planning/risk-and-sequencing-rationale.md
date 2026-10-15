# Risk and Sequencing Rationale — ロググループ全体の取得の高速化

上流：`bolt-plan.md`、`inception/units-generation/unit-of-work-dependency.md`、`ideation/scope-definition/scope-document.md`、`ideation/feasibility/raid-log.md`、`inception/requirements-analysis/requirements.md`、質問票 `delivery-planning-questions.md`。

## 使った考え方

順番は、薄い一本を最初に置き（Cockburn の walking skeleton：端から端まで通る最小の版を最初に作って動くことを確かめるやり方）、その後は「外さないものを先に、外すかもしれないものを後に」並べた（Q1）。数字で点数を付ける WSJF（価値と急ぎ度と危険の減らしやすさを、作業の大きさで割って順を決めるやり方、Reinertsen／SAFe）は使っていない。作業単位が 5 つと少なく、期限で外す順が scope で先に決まっているため、点数を付けても順は変わらないと判断した。

## 順番の理由

| 順 | Bolt | 理由 |
|----|------|------|
| 1 | U1 薄い一本 | team-practices の Walking Skeleton。いちばんの不安（同時に読んでも時刻順の表示と呼び出しの量が保てるか、feasibility R-01）を最初に確かめる。U2〜U5 はどれも U1 に依存する |
| 2 | U2 呼び出しの速さ | Must で外さない（FR2）。同じアカウントの他の利用者への影響を抑える（feasibility Q2）。U1 で固定の上限を入れたので、ここで制限を受けたときの引き下げと戻しを足す |
| 3 | U3 あきらめの決まり | Must で外さない（FR3）。U1 の暫定の動きを、同時に読むときの正しい決まりに置き換える |
| 4 | U4 かかった時間 | 期限が足りないときに 2 番目に外す（scope Q2）。速さを利用者が目で判断する手段（scope の「成功の判断」） |
| 5 | U5 中断 | 期限が足りないときに最初に外す（scope Q2）。最後に置くことで、外すときに他の作業単位を崩さずに済む |

## 依存の順との関係

作業単位の依存は「U2〜U5 はどれも U1 だけに依存する」で、この順は依存の順（U1 が最初）に沿っている。U2〜U5 の間の順は依存では決まらず、上の理由で決めた。順から外れた並べ方はない。

## 1 つずつ順に進める理由

開発者は 1 人で、U2〜U5 には同じファイルに手を入れる組み合わせがある（U2 と U5 は CallPacer・EventFetcher、U3 と U5 は StreamSchedule、U4 と U5 は AppSession・StatusLine）。1 つを `main` にマージしてから次に進めば、衝突を解く手間がなく、各作業単位のチェックポイントで動くものを確かめられる（Q3）。

## 早めに確かめる危険

| 危険 | 確かめる Bolt | 確かめ方 |
|------|---------------|----------|
| 同時に読んでも、呼び出しの量が上限の半分程度に収まるか（SDK の再試行を含む） | U1 | 偽物のゲートウェイで呼んだ時刻を記録する自動テスト（NFR3）、SDK の再試行を止める |
| 古い時刻のページが後から届いても、100 万件で滑らかに動くか | U1 | 既存の速度測定テスト（NFR5） |
| 本当に速くなるか | U1 | 偽物での自動テスト（NFR1）と、手元の実際の AWS での確認 |
| 制限を受け続けたときに無駄に待たないか | U3 | StreamSchedule の純粋なテスト（FR3） |
| 中断が速く効くか、閉じるときの今の動きを壊さないか | U5 | 止め方の自動テスト（NFR4）、閉じるときの回帰テスト |

## Assumptions & Open Questions

None.
