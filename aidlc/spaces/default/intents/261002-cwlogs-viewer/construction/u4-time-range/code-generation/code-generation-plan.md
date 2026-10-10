# Code Generation Plan — U4 時間範囲とタイムゾーン（u4-time-range）

上流の成果物：`construction/u4-time-range/functional-design/`（functional-spec.md・rules.md・entities.md、レビュー 2 回とも READY）、`inception/units-generation/unit-of-work.md`（U4）、`inception/domain-design/components.md`、`inception/requirements-analysis/requirements.md`。U1〜U3 のコード（`crates/local-sights-core`・`src-tauri`・`src/`）に機能を足す。質問票の回答は `code-generation-questions.md`（Q1：iana-time-zone と chrono-tz）。

## 1. 目的と範囲

上部バーで、日時の入力とログ一覧の時刻表示のタイムゾーンを、ローカルと UTC から選べるようにする（起動のたびにローカル、保存しない）。入力欄は文字列と瞬間を持ち、切り替えても瞬間を持つ入力は同じ瞬間のまま文字列だけを作り直し、瞬間を持たない入力は文字列を残して新しいタイムゾーンで解釈し直す。ローカルの夏時間で存在しない日時は形式の誤りとは別の理由、2 回現れる日時は早い方。タイムゾーンの変換はすべてライブラリが持ち、画面は変換をせず、ライブラリが文字列化した時刻を表示する。[Fetch] を押せない理由は FR3.4 の条件の並びで出す。

U4 の FR：FR3、FR3.1〜FR3.6。NFR：NFR12、NFR13。ルール：U4 の BR1.1〜BR3.5（`U1:`〜`U3:` の付いた ID は前の単位のルール）。

機能設計の再レビュー（上限の 2 回目）で出た指摘は、project.md の決まりに従って機能設計は直さず、ここで扱いを決める：

- R-08：表示用の時刻（displayTime）は、AppSession の行の取り寄せの処理で合成する。EventTimeline は TimeRangeModel を呼ばない（components.md にない依存を増やさない）。AppSession が EventTimeline から取り出した RowWindow の各行に、いまのタイムゾーンで TimeRangeModel が文字列化した時刻を付けて画面に返す。タイムゾーンを切り替えたら、SessionView の timeZone が変わり、画面はそれを合図に表示範囲の行を取り寄せ直す（timelineVersion は変えない）。
- R-09：状態遷移の「Valid が切替で誤りになる経路」は、新しいタイムゾーンで 4 桁の年に表せない瞬間のときだけ起きる（BR1.4）。entities の「instant は変わらない」の例外（遅い方の瞬間でも作り直した文字列は解釈し直さない、表せない瞬間は解釈し直す）は、コードの doc コメントとテストで明示する。機能設計の文書は直さない。

前の作業単位から U4 に持ち越した点はない。

## 2. 構成と技術の選択

| 項目 | 選択 | 理由 |
|------|------|------|
| ローカルのタイムゾーン | 起動時に `iana-time-zone` で OS のタイムゾーン名を読み、`chrono-tz` で変換する。名前が読めない・知らない名前のときは UTC として扱い、診断ログ（標準エラー）にその旨だけを出す | Q1、BR1.1、BR3.4 |
| 差し替えの境界 | `TimeZoneContext`（ローカルとして使う `chrono_tz::Tz` を持つ値）を AppSession に持たせ、テストでは決めた名前（America/New_York、Asia/Tokyo、UTC）で作る | BR3.4 |
| 入力の解釈 | `TimeRangeModel` に「選んだタイムゾーンで yyyy-mm-dd hh:mm:ss を解釈し、瞬間・形式の誤り・存在しない日時を返す」純粋な関数を足す。夏時間の 2 回現れる時刻は早い方（`LocalResult::Ambiguous` の早い方） | BR1.2 |
| 入力欄の状態 | `DateTimeInput`（text・instant・error）を新しく作り、FetchInput の開始・終了の文字列を置き換える | BR1.3、entities |
| 時刻の文字列化 | `TimeRangeModel` に「瞬間を選んだタイムゾーンで yyyy-mm-dd hh:mm:ss（入力欄）と yyyy-mm-dd hh:mm:ss.mmm（一覧）にする。表せなければ None・数値」の関数を足す。画面の `formatUtcMillis` は使わなくなる | BR1.4、BR3.1、BR3.4 |
| 画面への時刻の渡し方 | U3 の `get_rows` が返す各行に `displayTime` を足す（AppSession で合成、R-08） | BR3.4、entities RowWindow |
| タイムゾーンの切替 | コマンド `select_time_zone(timeZone)` を足す。取得中・確認待ちでも受け付ける | BR1.4、BR3.3 |
| 確認用プログラム | 変えない（UTC のまま） | BR3.5 |

部品とファイルの対応（ライブラリ側は `crates/local-sights-core/src/`）：

| 部品 | ファイル | U4 で作る範囲 |
|------|----------|---------------|
| TimeRangeModel | `time_range.rs`（選んだタイムゾーンでの解釈・文字列化、純粋なロジック）、`time_zone.rs`（新規、TimeZoneChoice・TimeZoneContext と OS のタイムゾーン名の読み取り）、`date_input.rs`（新規、DateTimeInput と切替の作り直し、純粋なロジック） | BR1.1〜BR1.6、BR3.1、BR3.4 |
| AppSession | `request.rs`（検証の理由の種類を足す、瞬間からの範囲）、`session.rs`（タイムゾーン、入力欄、切替、[Fetch] の条件と確認待ち、行の displayTime の合成） | BR1.4、BR1.5、BR2.1、BR2.2、R-08 |
| DesktopUi | `src-tauri/src/lib.rs`（`select_time_zone`、`get_rows` の displayTime、起動時の TimeZoneContext）、`src/components/TimeZoneToggle.tsx`（新規）・`ConnectionBar.tsx`（上部バーに置く）・`FetchForm.tsx`（ラベル）・`LogTable.tsx`（見出しと displayTime）、`src/hooks/useRowWindow.ts`（timeZone の変化で取り寄せ直す）、`src/api.ts`、`src/App.tsx`、`src/i18n/messages.ts`、`src/format.ts`（`formatUtcMillis` を消す） | BR2.2 の文言、BR3.1〜BR3.3 |

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

この契約の `ordering` をそのまま守る。純粋なロジック（選んだタイムゾーンでの解釈、夏時間の存在しない日時と 2 回現れる日時、瞬間の文字列化、切替での作り直しと解釈し直し、瞬間からの範囲、[Fetch] の理由の並び）はテストを先に書いて失敗を確かめてから実装する。OS のタイムゾーン名の読み取り、AppSession のつなぎ、Tauri のコマンドと画面は先に実装してから、その層のテストを書いて実行する。

## 3. 手順

### Step 1: 骨組みと設定

- [ ] `crates/local-sights-core/Cargo.toml` に `chrono-tz` と `iana-time-zone` を足す。ほかに新しい依存は足さない。テレメトリを送る依存は入れない（project.md Forbidden）。`cargo-deny` の許可するライセンスに収まることを確かめる
- [ ] `src/lib.rs` に `time_zone`・`date_input` のモジュールを宣言する

### Step 2: テストの実行環境と単位を絞ったコマンドの確認

- [ ] U1〜U3 の実行環境（`cargo test`・Vitest）をそのまま使う。`unit-test-instructions.md` の U4 のコマンドが実行できることを確かめる（ファイルができた時点で確かめる）

### Step 3: 純粋なロジックのテストを先に書く（失敗を確かめる）

各テストを書いたら実行し、失敗の出力を記録してから Step 4 に進む。

- [ ] `time_range.rs` のテスト：UTC では U1 と同じ瞬間になる。Asia/Tokyo（夏時間なし）で `2024-03-01 10:00:00` は UTC の `01:00:00`。America/New_York で `2024-03-10 02:30:00` は存在しない日時、`2024-11-03 01:30:00` は早い方（UTC の 05:30:00）。形の誤りは Format。瞬間の文字列化：Local（New_York）で夏時間をまたぐ 2 つの瞬間のオフセットが変わる、`.mmm` 付きの一覧用、4 桁の年に表せない瞬間は入力欄用が None・一覧用が数値（BR1.2、BR3.1、R-05）
- [ ] `date_input.rs` のテスト：空は instant も error も持たない。正しい入力は instant を持つ。切替で instant を持つ入力は文字列だけが変わり instant は同じ（Asia/Tokyo ⇄ UTC の受け入れ条件）。New_York の 2 回現れる時刻の遅い方の瞬間を UTC で入れ、ローカル → UTC と往復しても元の UTC の文字列に戻る。形式の誤りは切替後も誤りで文字列はそのまま。ローカルで存在しない日時は UTC に切り替えると正しい日時になる（R-02）。4 桁の年に表せない瞬間は文字列を残して解釈し直す（R-05）。同じ文字列を受けても解釈し直さず instant を保つ（BR1.5、R-06）
- [ ] `request.rs` のテスト：開始・終了の瞬間から範囲を作り、順序は瞬間で確かめる（BR1.6）。存在しない日時は StartNonexistentLocalTime・EndNonexistentLocalTime で、形式の誤りとは別。誤りがあるときは RangeOrder を出さない（BR2.1、BR2.2）
- [ ] `session.rs` の [Fetch] の条件のテスト：理由が条件の並びの順にすべて出る。確認待ちと取得中は押せない（理由の文言キーは足さない）（BR2.1、R-04）

### Step 4: 純粋なロジックを実装する（テストを通す）→ 整理する

- [ ] `time_range.rs`・`date_input.rs`・`request.rs`・`session.rs`（条件の部分）を実装し、Step 3 のテストを通す。テストが通ったまま整理する。テスト以外で `unwrap()` / `expect()` を使わない。entities の「instant は変わらない」の例外（R-09）を doc コメントに書く

### Step 5: OS のタイムゾーンの読み取りを実装する → テストする（BR1.1、BR3.4）

- [ ] `time_zone.rs`：TimeZoneChoice（Local・Utc、既定は Local）と TimeZoneContext（ローカルとして使う `Tz`）。起動時に `iana-time-zone` で名前を読み、`chrono-tz` で引く。読めない・知らない名前は UTC にし、名前だけを診断ログに出す（認証情報は関係しない）
- [ ] テスト（実装の後）：名前から TimeZoneContext を作る（知っている名前・知らない名前・空）。OS の設定に依存するテストは置かない

### Step 6: AppSession を広げる → テストする（BR1.4、BR1.5、BR2.1、R-08）

- [ ] `session.rs`：SessionState にタイムゾーンと開始・終了の DateTimeInput を持たせる。`select_time_zone` は取得中・確認待ちでも受け付け、入力欄を作り直して条件を確かめ直す（phase は変えない）。入力の書き換えは U1:BR1.4 のとおり取得中は拒む。行の取り寄せで、EventTimeline の各行に TimeRangeModel で文字列化した displayTime を付ける（R-08）。SessionView にタイムゾーン、入力欄の文字列と誤りを出す
- [ ] `session.rs` のテスト（実装の後）：起動時は Local。切替が取得中・確認待ちでも通り phase を変えない。取得中の入力の書き換えは拒まれる。切替で入力欄の文字列が作り直される。取り寄せた行の displayTime がいまのタイムゾーンで、切替後に変わる。取得の範囲はタイムゾーンに依らない。U1〜U3 のテストを DateTimeInput の形に直す

### Step 7: Tauri のつなぎを実装する

- [ ] `src-tauri/src/lib.rs`：起動時に TimeZoneContext を作って AppSession に渡す。コマンド `select_time_zone` を足し、`get_rows` の行に displayTime を入れる。`capabilities/default.json` に新しいコマンドの権限だけを足す
- [ ] 自動テストは置かず、`cargo build -p local-sights` で組み立てを確かめる

### Step 8: 画面を実装する → Vitest でテストする（BR2.2 の文言、BR3.1〜BR3.3）

- [ ] `src/api.ts`（SessionView のタイムゾーンと入力欄、行の displayTime、`selectTimeZone`）、`src/components/TimeZoneToggle.tsx`（ローカル / UTC の標準部品、Tab と矢印キー・スペース、取得中も使える）、`ConnectionBar.tsx`（上部バーに置く）、`FetchForm.tsx`（ラベルと形式の誤りの文言にいまのタイムゾーン）、`LogTable.tsx`（見出しにタイムゾーン、各行は displayTime をそのまま表示）、`src/hooks/useRowWindow.ts`（timeZone の変化で取り寄せ直す、R-08）、`src/App.tsx`、`src/i18n/messages.ts`（U4 の文言キーを英日で足し、「(UTC)」固定の文言を置き換える）、`src/format.ts`（`formatUtcMillis` を消す）。操作できる要素に `data-testid` を付ける
- [ ] 画面のテスト（実装の後）：`TimeZoneToggle.test.tsx`（切替、キーボード、取得中も押せる）、`LogTable.test.tsx`（見出しの切替、displayTime をそのまま出す）、`FetchForm.test.tsx`（ラベル、存在しない日時の文言）、`App.test.tsx`（切替で行を取り寄せ直す）、`messages.test.ts`（英日のそろい）の更新

### Step 9: ビルドと環境の設定

- [ ] `README.md` の手元の確認の項目に U4 の分（ローカルと UTC の切替で入力欄と一覧の時刻が変わる、夏時間のある地域での存在しない日時の表示）を足す
- [ ] `cargo fmt --check`・`cargo clippy --workspace --all-targets`・`npx tsc --noEmit`・`npx prettier --check .`・`npx eslint .`・`npm audit`・`cargo build -p local-sights` が通ることを確かめる

### Step 10: ドキュメントとトレーサビリティ

- [ ] 公開関数に doc コメントを付ける
- [ ] `code-summary.md`・`source-manifest.json`・`traceability.json`（U4 の BR・FR・NFR を実装またはテストのファイルに対応付ける）を書く

## 4. 要件・ルールと手順の対応

| 要件・ルール | 手順 |
|--------------|------|
| FR3.1・FR3.6・BR1.2・BR1.5・BR2.2 | Step 3、Step 4、Step 6、Step 8 |
| FR3.2・BR1.1・BR3.3 | Step 5、Step 6、Step 7、Step 8 |
| FR3.3・BR1.3・BR1.4・BR3.1・BR3.2・BR3.4（R-08、R-09） | Step 3、Step 4、Step 5、Step 6、Step 7、Step 8 |
| FR3.4・FR3.5・BR1.6・BR2.1 | Step 3、Step 4、Step 6、Step 8 |
| BR3.5 | 確認用プログラムは変えない（Step 9 で README の記述を確かめる） |
| NFR12・NFR13 | Step 8 |

## 5. 注意点

- U4 は AWS の API を新しく呼ばない。タイムゾーンの切替は取得し直さない（FR3.3）。
- 自動テストは実際の AWS に接続せず、OS のタイムゾーンの設定にも依存しない（TimeZoneContext を決めた名前で作る）。
- 診断ログに出してよいのはタイムゾーンの名前だけ（project.md Forbidden の対象の値は扱わない）。
- 画面での見た目の確認（切替、夏時間）は開発者本人の手元で行う。
