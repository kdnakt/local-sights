# Delivery Planning 質問票

上流：`inception/units-generation/unit-of-work.md`・`unit-of-work-dependency.md`・`unit-of-work-story-map.md`（U1〜U5。U2〜U5 はどれも U1 だけに依存）、`inception/contract-design/contract-summary.md`、`inception/requirements-analysis/requirements.md`、`ideation/scope-definition/scope-document.md`、`inception/practices-discovery/team-practices.md`。

このステージでは、作業単位を作る順番と、1 回の Bolt（作業を 1 回通して、動くものができるところまで進める区切り）に何を入れるかを決める。

前提として決まっていること：
- 最初は U1（薄い一本：同時に読んで画面に出す）。team-practices の Walking Skeleton（最初に端から端まで通る薄い一本を作り、動くことを確かめてから機能を足す）に従う。U1 の確かめは、`cargo test` がすべて通る・手元の確認用プログラムで実際の AWS から取得する・GUI で表示を目で見る、の 3 つ。
- U1 の前に、作業単位とは別の下準備（CI のカバレッジの検査、cargo-deny のコメントの修正、FR6）を小さな PR で入れる。
- 期限は 1 週間。収まらないときは、まず中断（U5）、次にかかった時間（U4）を外す。呼び出しの速さ（U2）とあきらめの決まり（U3）は外さない（scope Q2、RA Q5）。
- Construction（作るフェーズ）の今の設定は、作業単位ごとに設計からコードまで通す（unit-major）、1 つずつ順に進める（serial）、作業単位ごとに確認して承認する（checkpoints enabled）。

## Q1. U1 の後、残りの作業単位はどの順で作りますか？

背景：U2〜U5 は互いに依存しないので、どの順でも作れます。期限に収まらないときに外す順番は「U5 → U4」です。

A. 外さないものを先に、外すかもしれないものを後に：U1 → U2（呼び出しの速さ）→ U3（あきらめの決まり）→ U4（かかった時間）→ U5（中断）。期限が足りなくなっても、最後の U5、その前の U4 を外すだけで済む
B. 利用者に見える価値を先に：U1 → U5（中断）→ U4（かかった時間）→ U2 → U3。早く画面の変化を見せられるが、期限が足りなくなると外せない U2・U3 が残る
C. リスクを先に：U1 → U2 → U5 → U3 → U4。呼び出しの量と止め方という、AWS とのやり取りに関わるところを先に確かめる
X. Other (please specify)

[Answer]: A. 外さないものを先に、外すかもしれないものを後に：U1 → U2（呼び出しの速さ）→ U3（あきらめの決まり）→ U4（かかった時間）→ U5（中断）。期限が足りなくなっても、最後の U5、その前の U4 を外すだけで済む

## Q2. 1 回の Bolt の大きさはどうしますか？

背景：U1 は 12 部品ほどに手が入る大きめの作業単位です（作業単位の切り分けのレビュー R-04）。ただし、U1 は端から端まで通る薄い一本なので、途中で切ると「動くもの」にならない区切りができます。

A. 1 つの作業単位 = 1 回の Bolt：U1 も分けない。U1 の中は、作業単位の中の順番（FetchStatus の切り出し → StreamSchedule → CallPacer → 同時の読み取り → …）に沿って小さなコミットに分ける。`main` へは team-practices のとおり 1 つの作業単位を 1 つのスクワッシュコミットにする
B. U1 を 2 回の Bolt に分ける：前半でライブラリ（同時の読み取りと速さの上限）を作り、後半でアプリと画面につなぐ。前半だけでは画面で確かめられない
X. Other (please specify)

[Answer]: A. 1 つの作業単位 = 1 回の Bolt：U1 も分けない。U1 の中は、作業単位の中の順番（FetchStatus の切り出し → StreamSchedule → CallPacer → 同時の読み取り → …）に沿って小さなコミットに分ける。`main` へは team-practices のとおり 1 つの作業単位を 1 つのスクワッシュコミットにする

## Q3. 作業単位どうしを同時に進めますか？

背景：開発者は 1 人で、U2〜U5 には同じファイルに手を入れる組み合わせがあります（U2 と U5 は CallPacer・EventFetcher、U3 と U5 は StreamSchedule、U4 と U5 は AppSession・StatusLine）。

A. 1 つずつ順に進める（今の設定のまま）：1 つの作業単位を `main` にマージしてから次に進む。同じファイルの衝突が起きない
B. U1 の後は同時に進める：AI が複数の作業単位を並べて作る。早く終わる可能性はあるが、同じファイルの衝突をマージのたびに解く必要がある
X. Other (please specify)

[Answer]: A. 1 つずつ順に進める（今の設定のまま）：1 つの作業単位を `main` にマージしてから次に進む。同じファイルの衝突が起きない

## Consolidated Summary Confirmation

- 順番：下準備（CI のカバレッジの検査など、作業単位の外）→ U1 薄い一本 → U2 呼び出しの速さ → U3 あきらめの決まり → U4 かかった時間 → U5 中断。期限が足りなければ U5、次に U4 を外す（Q1）
- 区切り：1 つの作業単位 = 1 回の Bolt。U1 も分けず、中は小さなコミットに分ける。`main` へは 1 作業単位 = 1 スクワッシュコミット（Q2）
- 進め方：1 つずつ順に進める。1 つを `main` にマージしてから次へ（Q3、今の設定 unit-major・serial のまま）
- 薄い一本（U1）の確かめ：`cargo test` がすべて通る、手元の確認用プログラムで実際の AWS から取得する、GUI で取得結果の表示を目で見る（team-practices Walking Skeleton）
- 外部への依存：手元の確認に使う開発者本人の AWS の認証情報とテスト用のロググループ（feasibility Q5 で用意済み）だけ。他のチームや承認待ちはない

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
