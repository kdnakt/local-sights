# Code Generation 質問票 — U1 薄い一本（u1-walking-skeleton）

上流の成果物：`construction/u1-walking-skeleton/functional-design/`（functional-spec.md・rules.md・entities.md）、`inception/units-generation/unit-of-work.md`、`inception/requirements-analysis/requirements.md`。

functional-spec.md の「4. 画面」で「具体的なつなぎ方は Code Generation で決める」とした点と、計画を書くのに必要な技術の選択だけを確認する（project.md Corrections：先送りした技術選定は必要になった時点で質問して決める）。

## Q1. 画面（TypeScript + React）とライブラリ（Rust）を、どうつなぎますか？

背景：画面は自分で状態を持たず、AppSession の状態を表示し、操作を伝えるだけにします（ADR-001）。取得の進み具合とログは、ページごとに受け口に届きます（BR4.5）。

A. 操作は Tauri のコマンドで伝え、状態の変化とページごとのログのまとまりは Tauri のイベントで画面に送る（ライブラリから画面へ押し出す）
B. 操作は Tauri のコマンドで伝え、画面が一定間隔でコマンドを呼んで状態とログを取りに行く（画面から引き出す）
X. Other (please specify)

[Answer]: A. 操作は Tauri のコマンドで伝え、状態の変化とページごとのログのまとまりは Tauri のイベントで画面に送る（ライブラリから画面へ押し出す）

## Q2. 画面側（DesktopUi）の自動テストをどうしますか？

背景：team.md の Testing Posture ではテストは `cargo test` で実行し、Test Strategy は Standard（部品ごとに 5〜8 件）です。CI には画面側の型チェック・Prettier・ESLint は入りますが、画面側のテストの実行方法はまだ決めていません。U1 の画面は、状態を持たず表示と操作の伝達だけを行います。

A. 画面側にも Vitest と React Testing Library を入れ、DesktopUi のテスト（押せる・押せない、取得中の無効化、1 行表示、件数と 0 件、エラー表示、Enter キー）を書く。文言の英日のそろいも Vitest で確かめる。実行は `npm test`（CI への組み込みは CI Pipeline ステージ）
B. 画面側の自動テストは置かない。画面に出す状態の判断はすべて Rust の AppSession に寄せて `cargo test` で確かめ、文言の英日のそろいも Rust のテストで文言ファイルを読んで確かめる。画面は目視で確認する
X. Other (please specify)

[Answer]: A. 画面側にも Vitest と React Testing Library を入れ、DesktopUi のテスト（押せる・押せない、取得中の無効化、1 行表示、件数と 0 件、エラー表示、Enter キー）を書く。文言の英日のそろいも Vitest で確かめる。実行は `npm test`（CI への組み込みは CI Pipeline ステージ）

## Consolidated Summary Confirmation

回答のまとめ：

- 画面とライブラリのつなぎ方：操作を Tauri のコマンドで伝え、状態の変化とページごとのログのまとまりを Tauri のイベントで画面に送る（Q1）
- 画面側のテスト：Vitest と React Testing Library で DesktopUi と文言の英日のそろいをテストし、`npm test` で実行する。ライブラリ側は `cargo test` で実行する（Q2）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct

## Plan Approval

対象：`code-generation-plan.md`（埋め込みの Testing Contract を含む）と `unit-test-instructions.md`。

[Approval Fingerprint]: sha256:v3:7a9548bf93326fcba51af5ada321a313d65e194cb04c66a2ed0d395a329a7ae9
[Planned Source]: 5cdb88f9dfe1c1e7b39d5b632ae42bdc0f391b7097c2885f078926ba889d9e83

- Approve Plan
- Request Changes

[Answer]: Approve Plan
