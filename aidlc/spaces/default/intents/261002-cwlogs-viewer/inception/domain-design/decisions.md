# Architecture Decision Records — Domain Design

上流の成果物：`inception/requirements-analysis/requirements.md`、`inception/practices-discovery/team-practices.md`。部品の詳細は `components.md`。質問票の回答は `domain-design-questions.md` の Q1〜Q5、F1。ADR-008 と、ADR-006・ADR-007 の一部は、アーキテクチャレビューの指摘（R-01、R-03、R-04、R-06）を受けた人間の判断による。

## ADR-001: 画面のルールを GUI から切り離し、GUI 非依存の AppSession に置く

- **Context** — [Fetch] を押せる条件（FR3.4）、取得中のロック（FR4.8）、終了確認（FR4.9）、絞り込みの引き継ぎ（FR6.4）は画面の振る舞いだが、ルールとして自動テストできる。開発ルールは「GUI に依存しないライブラリと GUI アプリを分ける」「GUI に依存しない層の公開関数には必ずテストを書く」としている（team-practices）。GUI フレームワークはまだ決まっていない。
- **Decision** — 画面の状態とルールを AppSession としてライブラリ側に置く。DesktopUi は AppSession の状態を表示し、操作を伝えるだけにする（Q3）。
- **Consequences** — 良い点：ルールを `cargo test` で確かめられる。GUI フレームワークの選定・変更（Windows 対応 IB-16 を含む）の影響が DesktopUi に閉じる。悪い点：状態の受け渡しの層が 1 つ増え、MVP の作業量がわずかに増える。セキュリティ：AppSession はエラーの安全な詳細だけを扱い、認証情報を持たない（NFR5）。
- **Alternatives Rejected** — ルールを GUI の中に置く案。部品は少ないが、ルールの確認が GUI の手動確認になり、開発ルールのテスト方針と合わない。

## ADR-002: CloudWatch Logs の呼び出しを、読み取り 3 API だけを持つ境界（trait）の裏に置く

- **Context** — 自動テストと CI では実際の AWS に接続しない（project.md Forbidden、NFR14）。呼ぶ API は DescribeLogGroups・DescribeLogStreams・GetLogEvents だけに限る（NFR6）。エラー表示とログに秘密の認証情報とアクセスキー ID を出さない（FR8.3、NFR5）。
- **Decision** — CloudWatchLogsGateway を境界とし、3 API だけを操作として持つ trait と、AWS SDK による実装を置く。AWS のエラーは種類に分類し、安全な詳細だけを返す。LogGroupBrowser・StreamPlanner・EventFetcher はこの境界だけを通して AWS を呼ぶ。
- **Consequences** — 良い点：テストでは偽物に差し替えられ、ページ終端・スロットリング・部分失敗などの状況を再現できる。呼べる API が型の上で 3 つに限られる。悪い点：SDK の新しい機能を使うたびに境界を広げる必要がある。セキュリティ：読み取り専用であることと、エラー情報の無害化を 1 か所で保証できる。README に必要な IAM 権限（3 つ）を書ける。
- **Alternatives Rejected** — 各部品が AWS SDK を直接呼ぶ案。部品は減るが、テストで実際の AWS が必要になり、呼べる API の制限とエラーの無害化が散らばる。

## ADR-003: 取得を、ストリームの選定（StreamPlanner）とページ取得（EventFetcher）に分ける

- **Context** — 取得は「ストリームを列挙して時刻で絞る（FR4.1〜FR4.2）」と「ページをたどり、再試行し、失敗を記録する（FR4.3〜FR4.5、FR4.10）」の 2 段階からなる。MVP 後の最優先は横断取得の高速化（IB-07）で、取得のしかた（並列化など）を変えることになる。
- **Decision** — StreamPlanner と EventFetcher の 2 つの部品に分ける（Q1）。
- **Consequences** — 良い点：高速化のときに EventFetcher だけを差し替えればよい。選定ルール（余裕 1 時間、時刻なしのストリームは含める）とページ終端判定を、それぞれ純粋なロジックとして先にテストできる。悪い点：2 つを束ねる役が必要になる（ADR-004）。セキュリティ：変化なし（どちらも ADR-002 の境界を通る）。
- **Alternatives Rejected** — 1 つの FetchService にまとめる案。最初は作りやすいが、高速化の差し替え範囲が広がり、選定と取得のテストが混ざる。

## ADR-004: 取得の流れを FetchCoordinator が束ね、キャッシュの判断と「全成功時だけ書く」を担う

- **Context** — キャッシュは任意・既定無効で、範囲がキャッシュ済みに含まれれば使う（FR7.4）。失敗したストリームがある取得と途中で終了した取得は記録しない（FR7.9）。利用者は「取得側が判断し、呼ぶ側はキャッシュを意識しない」ことを選んだ（Q4）。ADR-003 で取得を 2 つに分けたため、束ねる入口が要る。
- **Decision** — FetchCoordinator を置き、キャッシュの判断、StreamPlanner と EventFetcher の呼び出し、EventTimeline への逐次追加、全ストリーム成功時だけのキャッシュ書き込み、部分失敗のまとめを担わせる。StreamPlanner と EventFetcher はキャッシュを知らない（F1）。
- **Consequences** — 良い点：キャッシュのルールが 1 か所にまとまり、「欠けたログをキャッシュ済みとして残さない」ことを確かめやすい（意図 SM1 に沿う）。キャッシュ（FR7）を外す場合も、FetchCoordinator の分岐を外すだけで済む。悪い点：部品が 1 つ増える。セキュリティ：機密情報を含みうるログをディスクに書く判断が 1 か所にあり、無効時に書かないこと（NFR8）を確かめやすい。
- **Alternatives Rejected** — AppSession がキャッシュを判断する案（Q4 の B）は、画面のルールとキャッシュのルールが混ざる。StreamPlanner を入口にする案（F1 の B）は、選定の部品がキャッシュと取得の流れまで抱え、ADR-003 の分割の意味が薄れる。

## ADR-005: 絞り込み（FilterEngine）をログの保持（EventTimeline）と分ける

- **Context** — 部分一致の絞り込み（FR6）は保持しているログに対して行う。MVP 後に Logs Insights 互換のクエリ（IB-11）を足す予定で、それは絞り込みの発展形である。性能目標は 10 万件で 10 秒以内（NFR1）、100 万件で 100 秒以内（NFR2）。
- **Decision** — FilterEngine を EventTimeline とは別の部品にし、「ログの並びと条件を受け取り、合う行を返す」役にする（Q2）。
- **Consequences** — 良い点：クエリへの置き換えや拡張が FilterEngine に閉じる。絞り込みのルールを単独でテストできる。悪い点：データの近くで処理する最適化はしにくくなる。目標の時間は余裕があるため、MVP では問題にならない見込み（Build and Test で確かめる）。セキュリティ：変化なし（手元のデータだけを扱い、AWS を呼ばない、FR6.2）。
- **Alternatives Rejected** — EventTimeline が絞り込みも担う案。速度は出しやすいが、保持とクエリという変わる理由の違う責任が混ざる。

## ADR-006: ライブラリはエラーの種類と安全な詳細だけを返し、表示文は DesktopUi が作る

- **Context** — エラーは「何が起きたか」と「次の行動」を文字で示す（FR8.1）。表示言語は英語と日本語（NFR12）。秘密の認証情報とアクセスキー ID は出さない（FR8.3）。
- **Decision** — CloudWatchLogsGateway がエラーを種類（認証切れ・権限不足・スロットリング・通信断・その他）に分類し、安全な詳細を添える。AppSession はそれをそのまま渡し、DesktopUi が言語に合わせて文を組み立てる（Q5）。あわせて、全部品に共通のルールとして、診断ログには同じ基準の安全な詳細だけを出し、出力先は標準エラー出力だけとする（NFR16、レビュー R-03）。
- **Consequences** — 良い点：翻訳と文言が GUI 側に集まり、ライブラリは言語に依存しない。エラーの種類ごとの扱いをテストできる。悪い点：新しいエラーの種類を足すときに、DesktopUi の文言も足す必要がある。セキュリティ：安全な詳細への変換を境界で行うため、表示層に秘密が届かない。
- **Alternatives Rejected** — ライブラリが英語・日本語の表示文まで作る案。翻訳が GUI 非依存の層に入り込み、表示言語の切り替えの責任が分散する。

## ADR-007: 取得したログを EventTimeline が逐次受け取り、常に時刻順で保持する。取得の進み具合は通知で伝える

- **Context** — 結果は取得したものから逐次表示し、常にストリームを跨いだ時刻順に並べる（FR4.7）。取得中も操作に 1 秒以内に応える（NFR3）。100 万件でスクロールに 1 秒以内に応える（NFR2）。
- **Decision** — EventFetcher のページ取得は FetchCoordinator から非同期に動かし、取得したログを EventTimeline に逐次追加する。FetchCoordinator は進み具合（追加件数・完了・中断・失敗数）を、取得開始時に AppSession が渡した受け口へ届ける。FetchCoordinator は AppSession を知らず、依存は AppSession → FetchCoordinator の一方向に保つ（レビュー R-04）。EventTimeline は時刻順を保ったまま追加を受け付ける。同じ時刻の並び順の決め方と、保持の具体的な形は Functional Design で決める。
- **Consequences** — 良い点：取得中も画面が固まらず、結果が順次見える。悪い点：取得と表示が並行して動くため、状態の受け渡しに注意が要る（AppSession のルールでテストする）。逐次追加でも時刻順を保つ処理の性能を確かめる必要がある。セキュリティ：変化なし（メモリ上で扱い、キャッシュ無効時はディスクに書かない、NFR8）。
- **Alternatives Rejected** — すべて取得してからまとめて並べ替えて表示する案。作りは簡単だが、要件分析で選んだ逐次表示（レビュー R-02 の判断）に合わない。

## ADR-008: 取得の中断は AppSession → FetchCoordinator → EventFetcher の順に伝え、中断した取得は捨ててキャッシュに書かない

- **Context** — 取得中にウィンドウを閉じて [Close] を選ぶと、取得途中の結果を捨てて終了する（FR4.9）。途中で終了した取得はキャッシュ済みとして記録しない（FR7.9）。利用者向けの「中断」操作は MVP の対象外（IB-08）。
- **Decision** — AppSession が終了時に FetchCoordinator へ中断を指示し、FetchCoordinator が EventFetcher に取得を止めさせる。中断した取得は中断済みとして扱い、キャッシュには書かず、EventTimeline に保持ログを破棄させて途中の結果を捨てる。取得し直しの開始時も、FetchCoordinator が EventTimeline に前回の保持ログを破棄させる（レビュー R-06）。この経路は終了時の内部処理だけに使い、画面に「中断」ボタンは置かない（レビュー R-01）。
- **Consequences** — 良い点：終了時に AWS への呼び出しが残らず、欠けたログがキャッシュに残らない（SM1、FR7.9）。MVP 後に「中断」操作（IB-08）を足すときは、この経路を画面から呼ぶだけで済む。悪い点：取得中の各部品が中断の指示を確認する必要があり、取得の処理が少し複雑になる。セキュリティ：中断時に途中の結果をディスクに残さないため、キャッシュ無効時に書かないこと（NFR8）と矛盾しない。
- **Alternatives Rejected** — 中断の経路を持たず、プロセスの終了に任せる案。作りは簡単だが、終了処理の途中でキャッシュへの書き込みが走る余地が残り、FR7.9 を確かめられない。中断を Functional Design まで先送りする案は、部品の責任とやり取りに関わるため採らなかった。
