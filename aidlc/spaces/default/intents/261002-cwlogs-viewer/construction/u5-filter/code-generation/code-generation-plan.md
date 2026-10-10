# Code Generation Plan — U5 絞り込み（u5-filter）

上流の成果物：`construction/u5-filter/functional-design/`（functional-spec.md・rules.md・entities.md、レビュー 1 回目 NOT-READY → 修正 → 2 回目 READY）、`inception/units-generation/unit-of-work.md`（U5）、`inception/domain-design/components.md`、`inception/requirements-analysis/requirements.md`。U1〜U4 のコード（`crates/local-sights-core`・`src-tauri`・`src/`）に機能を足す。

## 1. 目的と範囲

条件エリアの絞り込みの欄で、取得済みのログをメッセージの部分一致（大文字・小文字を区別しない）で絞り込む。入力が止まって約 0.3 秒後にかけ、保持ログを小分けに走査して見つかった行から一覧に出す。取得中も届いたページに逐次同じ絞り込みをかけ、取り直しても文字列は残る。ステータス行に「絞り込み後 / 全件」と絞り込み中を出す。AWS の API は呼ばない。10 万件で 10 秒以内、100 万件で 100 秒以内。

U5 の FR：FR6、FR6.1〜FR6.5。NFR：NFR1、NFR2、NFR12、NFR13。ルール：U5 の BR1.1〜BR3.6（`U1:`〜`U4:` の付いた ID は前の単位のルール）。

U5 では、機能設計で決めた振る舞いを実現するのに新しく選ぶ技術はない（部分一致と小文字化は Rust の標準、0.3 秒待ちは画面の標準の仕組みで作る）。そのため技術の質問はせず、計画の承認だけを確認する。

機能設計の再レビュー（上限の 2 回目）で出た指摘は、project.md の決まりに従って機能設計は直さず、ここで扱いを決める：

- R-02：保持ログ（EventTimeline）と絞り込みの状態（FilterEngine の条件・結果）を 1 つの構造体 `LogView` にまとめ、1 つの `Mutex` で守る。`LogView` に U3 の `TimelineStore`（`add`・`discard`）を実装し、`add` の中で保持ログへの追加と逐次の判定（BR2.1）を、`discard` の中で保持ログの破棄と世代（timelineEpoch）の進めと結果の空化（BR2.2）を、同じロックの中で続けて行う。これで BR2.5 の「同じロックの中での読み書き」を満たし、ロックの順はセッション → `LogView` の 2 段になる（機能設計の「保持ログ → 絞り込み結果」の 2 つのロックは、1 つのロックにまとめた形で満たす）。matchedKeys への挿入は、同じキーがすでにあれば入れない（冪等）。
- R-07：走査の再開は、scanCursor のキーより大きい最初の行の位置を保持ログの並びから二分探索で求める。開始直後は「status = Filtering、scanCursor なし（まだ 1 行も走査していない）」、Ready は「status = Ready、scanCursor なし」で、status で区別する。
- R-08：絞り込み中の位置の問い合わせは、(logStreamName, sequence) から U3 の索引で timestamp を引き、キーを作って matchedKeys を二分探索する。キーが結果にないとき・行が保持ログにないときは、U3 と同じく「ない」を返す（画面は U3 のとおり一番上に戻る）。
- R-09：matchedKeys のキーには、ストリーム名の文字列ではなく、`LogView` の中で振るストリームの番号（ストリーム名の文字列の順を保つ必要があるため、比較は番号の表から名前を引いて行うか、名前の `Arc<str>` を共有する）を使い、100 万件でも行ごとに文字列を複製しない。

前の作業単位から U5 に持ち越した点：U4 のコード生成のレビューの R-01〜R-03 は人間の判断で直さないことにした（U4 の日誌）。U5 で扱う持ち越しはない。

## 2. 構成と技術の選択

| 項目 | 選択 | 理由 |
|------|------|------|
| 一致の判定 | メッセージと文字列をどちらも `to_lowercase()` してから `contains`。文字列の小文字化は条件を決めたときに 1 回だけ | BR1.2、Q1 |
| 保持ログと結果のロック | `LogView`（EventTimeline と FilterEngine の状態）を 1 つの `std::sync::Mutex` で守る | BR2.5、R-02 |
| 結果の持ち方 | matchedKeys はキー（timestamp、ストリームの番号、sequence）の昇順の `Vec`。追加は二分探索で位置を求めて挿入 | BR2.1、BR2.3、R-09 |
| 走査 | 取得とは別の非同期の作業として、数千行ずつ `LogView` のロックを取って放しながら進める。filterId と timelineEpoch を確かめ、違えばやめる | BR1.5、BR2.4 |
| 画面への知らせ | `session-changed` と U3 の `fetch-progress` に filterSummary を入れる。走査の進みと Ready への変化は `fetch-progress` と同じ形の軽いイベントで送る | BR3.6 |
| 行の取り寄せ | U3・U4 の `get_rows` を、絞り込み中は matchedKeys の中の位置として扱う。返す値に filtered・allCount・resultVersion を足す | BR3.1 |
| 0.3 秒待ち | 画面で `setTimeout` を使い、入力のたびに待ち直す | BR1.3 |

部品とファイルの対応（ライブラリ側は `crates/local-sights-core/src/`）：

| 部品 | ファイル | U5 で作る範囲 |
|------|----------|---------------|
| FilterEngine | `filter.rs`（新規。条件の正規化・一致の判定・結果のキーの並び・逐次の判定・走査の 1 回分・世代と filterId の確認、純粋なロジック） | BR1.1、BR1.2、BR1.4、BR1.5、BR2.1〜BR2.4 |
| LogView | `log_view.rs`（新規。EventTimeline と FilterEngine をまとめ、TimelineStore を実装。絞り込みを考えた行の取り出しと位置の問い合わせ） | BR2.1、BR2.2、BR2.5、BR3.1、BR3.2、R-02、R-08 |
| AppSession | `session.rs`（絞り込みの文字列の受け取り、filterSummary、SessionView・進み具合への追加） | BR1.1、BR1.4、BR3.3、BR3.6 |
| DesktopUi | `src-tauri/src/lib.rs`（保持ログの置き場所を `LogView` にする、コマンド `set_log_filter(text)`、走査の作業の起動と中止、`get_rows`・`find_row_position` の絞り込み対応、知らせ）、`src/components/LogFilterInput.tsx`（新規）・`FetchForm.tsx`（条件エリアに置く）・`StatusLine.tsx`（「絞り込み後 / 全件」と絞り込み中）・`LogTable.tsx`、`src/hooks/useRowWindow.ts`（filterId・resultVersion で取り寄せ直す、古い版の行を捨てる）、`src/hooks/useDebouncedValue.ts`（新規）、`src/api.ts`、`src/App.tsx`、`src/i18n/messages.ts` | BR1.3、BR3.1〜BR3.4、BR3.6 |

## Testing Contract

```json
{
  "version": 1,
  "methodology": "custom",
  "source": "team",
  "ordering": "時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。",
  "scope": "local-tool",
  "test_strategy": "standard",
  "project_type": "greenfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "私たちはテストを各 Bolt の成果物の一部として扱う。純粋なロジックはテストを先に書き、外部とつながる層は実装してからテストを書く。\n\n- **Methodology**: custom\n- **Ordering**: 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。\n- テストの量と種類は、記録済みの Test Strategy（`Standard`）に従う。\n- 数値のカバレッジ下限は設けない（スコープ `local-tool` には org.md のスコープ別下限がない）。その代わり、GUI に依存しない層（ライブラリ側）の公開関数には必ずテストを書く。\n- 自動テストと CI では実際の AWS に接続しない。AWS 呼び出しは trait の裏に置き、テストではモック／スタブに差し替える。\n- テストは `cargo test` で実行し、CI でマージ前に実行する。既存のテストはすべて通った状態を保つ。"
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "custom",
    "runner_step": "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
      "Custom ordering - 時刻順マージ・ページ終端判定・再試行・タイムゾーン変換などの純粋なロジックはテストを先に書いてから実装し、AWS 接続と GUI の層は先に実装してからその層のテストを書いて実行する。",
      "Implementation and tests - preserve that exact ordering; do not convert it to layer-local TDD.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:1ba5c284be5a836c03c42abd086c6060b6df044056afea8de0055490fa1429c2",
  "contract_sha256": "sha256:12a8505da0cfbdc63db0b40dc33c162212ab7b3fb1ec6447799d4099dfd31ddf"
}
```

この契約の `ordering` をそのまま守る。純粋なロジック（条件の正規化、一致の判定、結果のキーの並び、逐次の判定と走査の位置の分担、世代・filterId による古い結果の破棄、`LogView` の追加・破棄・絞り込みを考えた取り出しと位置の問い合わせ）はテストを先に書いて失敗を確かめてから実装する。AppSession のつなぎ、Tauri のコマンドと走査の作業、画面は先に実装してから、その層のテストを書いて実行する。

## 3. 手順

### Step 1: 骨組みと設定

- [x] `src/lib.rs` に `filter`・`log_view` のモジュールを宣言する。新しい依存は足さない

### Step 2: テストの実行環境と単位を絞ったコマンドの確認

- [x] U1〜U4 の実行環境（`cargo test`・Vitest）をそのまま使う。`unit-test-instructions.md` の U5 のコマンドが実行できることを確かめる（ファイルができた時点で確かめる）

### Step 3: 純粋なロジックのテストを先に書く（失敗を確かめる）

各テストを書いたら実行し、失敗の出力を記録してから Step 4 に進む。

- [x] `filter.rs` のテスト：前後の空白を除き、空は条件なし（BR1.1）。大文字・小文字を区別しない（`error` で `Error: timeout`）、改行を含むメッセージの 2 行目でも一致、ストリーム名は比べない、日本語の部分一致（BR1.2）。同じ文字列では filterId が増えない、変われば増える（BR1.4）。結果はキーの昇順で、同じキーは二重に入らない（BR2.3、R-02）。走査の 1 回分が scanCursor を進め、終わりで Ready（BR1.5）。Filtering 中の逐次の判定は scanCursor 以下のキーだけ（またぐページで取りこぼしも重複もない）、Ready ではすべて（BR2.1、R-01）。filterId か timelineEpoch が違う作業の結果は捨てる（BR2.4）。結果が変わるたびに resultVersion が増える（BR3.6）
- [x] `log_view.rs` のテスト：`add` で保持ログへの追加と逐次の判定が一緒に起きる（絞り込み中は合う行だけ結果に入る）。`discard` で timelineEpoch が進み結果が空、文字列は残る（BR2.2）。絞り込み中の行の取り出しは matchedKeys の中の位置で、totalCount は matchedCount、allCount は全件、filtered = true（BR3.1）。絞り込んでいないときは U3・U4 のまま。走査の途中でも、ここまでに見つかった行が取り出せる（R-02）。絞り込み中の位置の問い合わせは結果の中の位置、結果にない・保持ログにないときは「ない」（R-08）。走査の再開位置は二分探索で求める（R-07）

### Step 4: 純粋なロジックを実装する（テストを通す）→ 整理する

- [x] `filter.rs`・`log_view.rs` を実装し、Step 3 のテストを通す。テストが通ったまま整理する。テスト以外で `unwrap()` / `expect()` を使わない。matchedKeys のキーはストリームの番号を使い、行ごとにストリーム名を複製しない（R-09）

### Step 5: AppSession を広げる → テストする（BR1.1、BR1.4、BR3.3、BR3.6）

- [x] `session.rs`：絞り込みの文字列を受け取り、正規化して `LogView` の条件を変える指示を返す。filterSummary（filterId・matchedCount・allCount・status・resultVersion）を SessionView と進み具合（`fetch-progress` の中身）に入れる。取得中も文字列を受け付ける。確認待ちの間は U2 のとおり受け付けない
- [x] `session.rs` のテスト（実装の後）：文字列の受け取りと filterSummary、取得中も受け付ける、取り直しで文字列が残る、確認待ちで拒む、SessionView と進み具合に filterSummary が入る

### Step 6: Tauri のつなぎを実装する

- [x] `src-tauri/src/lib.rs`：保持ログの置き場所を `Mutex<LogView>` にし、取得の流れには `LogView` を `TimelineStore` として渡す（R-02）。コマンド `set_log_filter(text)` を足し、条件を変えたら走査の作業を起動する（前の作業はやめる）。走査の 1 回分ごとと Ready のときに軽いイベントで filterSummary を送る。`get_rows`・`find_row_position` は `LogView` の絞り込みを考えた取り出しを使う。ロックの順はセッション → `LogView`。`capabilities/default.json`・`build.rs` に新しいコマンドの権限だけを足す
- [ ] 自動テストは置かず、`cargo build -p local-sights` で組み立てを確かめる

### Step 7: 取得と絞り込みの結合テストを書く（実装の後）

- [x] 結合テスト `crates/local-sights-core/tests/u5_filter_flow.rs`：偽物の gateway と `LogView` で、絞り込みの文字列を入れたまま取得すると届いたページの合う行だけが結果に入る。取り直しで結果が空になり文字列は残る。走査の途中にページが届いても取りこぼしも重複もない（FR6.4、BR2.1、BR2.2）

### Step 8: 画面を実装する → Vitest でテストする（BR1.3、BR3.1〜BR3.4、BR3.6）

- [x] `src/api.ts`（`setLogFilter`、filterSummary、行の取り出しの filtered・allCount・resultVersion）、`src/hooks/useDebouncedValue.ts`（0.3 秒待ち）、`src/components/LogFilterInput.tsx`（条件エリアの入力欄、ラベル・プレースホルダーは英日、取得中も使える）、`FetchForm.tsx`（条件エリアに置く）、`StatusLine.tsx`（「絞り込み後 N 件 / 全 M 件」、0 件の文字、絞り込み中）、`src/hooks/useRowWindow.ts`（filterId・resultVersion が変わったら件数が同じでも取り寄せ直す、古い版の行を捨てる、filterId が変わったら一番上に戻る）、`src/App.tsx`、`src/i18n/messages.ts`（U5 の文言キーを英日で足す）。操作できる要素に `data-testid` を付ける
- [x] 画面のテスト（実装の後）：`LogFilterInput.test.tsx`（0.3 秒待ちで最後の文字列だけ渡す。待ちはテストで実時間を待たない）、`StatusLine.test.tsx`（「絞り込み後 / 全件」、0 件、絞り込み中）、`App.test.tsx`（版が変わると件数が同じでも取り寄せ直す、古い版の行は捨てる）、`messages.test.ts`（英日のそろい）の更新

### Step 9: 速さの確かめとビルドの設定

- [x] `filter.rs` か `log_view.rs` にリリースビルドの `#[ignore]` のテストを置き、10 万件・100 万件の全体の絞り込みの時間を測る（上限は NFR1 の 10 秒・NFR2 の 100 秒）。100 万件を持った状態での 1 ページ（約 1 万件）の追加と逐次の判定の時間も測る
- [x] `README.md` の手元の確認の項目に U5 の分（絞り込み、取得中の逐次、取り直しで文字列が残る、100 万件近いときの絞り込み中の操作）を足す
- [ ] `cargo fmt --check`・`cargo clippy --workspace --all-targets`・`npx tsc --noEmit`・`npx prettier --check .`・`npx eslint .`・`npm audit`・`cargo build -p local-sights` が通ることを確かめる

### Step 10: ドキュメントとトレーサビリティ

- [x] 公開関数に doc コメントを付ける
- [x] `code-summary.md`・`source-manifest.json`・`traceability.json`（U5 の BR・FR・NFR を実装またはテストのファイルに対応付ける）を書く

## 4. 要件・ルールと手順の対応

| 要件・ルール | 手順 |
|--------------|------|
| FR6.1・FR6.5・BR1.1・BR1.2・BR2.3 | Step 3、Step 4、Step 5、Step 8 |
| FR6.2・NFR1・NFR2・BR1.5 | Step 3、Step 4、Step 6、Step 9 |
| FR6.3・BR3.3・BR3.6 | Step 5、Step 6、Step 8 |
| FR6.4・BR2.1・BR2.2・BR2.4（R-01） | Step 3、Step 4、Step 6、Step 7 |
| BR1.3・BR1.4 | Step 3、Step 8 |
| BR2.5（R-02） | Step 3、Step 4、Step 6 |
| BR3.1・BR3.2（R-07、R-08） | Step 3、Step 4、Step 6、Step 8 |
| NFR12・NFR13・BR3.4 | Step 8 |
| BR3.5 | 確認用プログラムは変えない |

## 5. 注意点

- U5 は AWS の API を新しく呼ばない（FR6.2）。
- 自動テストは実際の AWS に接続しない。0.3 秒の待ちは実時間で待たない。
- 100 万件の速さのテストはリリースビルドでの `#[ignore]` のテストとし、通常の `cargo test` の時間を延ばさない。ディスクの空きが少ないため、リリースビルドの後は `target/release` を消す。
- 画面での絞り込みの体感は、開発者本人の手元で確かめる。
