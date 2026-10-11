## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-10-10T13:34:34Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261010-console-ui-polish/inception/refined-mockups/mockups.md > 冒頭の決定表（時刻が空のとき AS1）、interaction-spec.md > §3 日付欄と時刻欄の振る舞い | Q4 で AS1（時刻が空なら不正）を「自動で補う」に置き換えたが、承認済みの要件 FR1.4 とその受け入れ基準（「時刻が空 → [Fetch] は押せず、開始の時刻欄の直下に理由」）は残ったままで、そのまま矛盾する。さらに補完は「焦点が外れたとき」だけなので、時刻が空のまま日付欄で Enter を押したとき（§3 の「Enter（どの欄でも）」）に、補完されるのか、[Fetch] が有効なのか、どの値で取得するのかが書かれていない。 | 要件側（FR1.4、その受け入れ基準、AS1）を Q4 に合わせて直す対象として明記する。空の時刻で Enter を押す、空のまま Fetch ボタンを押す、補完前の [Fetch] の有効・無効の状態を、欄の補完と同じ値になることを含めて §3 に追記する。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261010-console-ui-polish/inception/refined-mockups/interaction-spec.md > §8「形式の判定」行、mockups.md > 画面 5a の紐づけの文 | validationErrors の欄への割り当てが表になっていない。§8 は「mockups.md 画面 5a の表」を参照するが、5a には表がなく散文だけがある。logGroupRequired、logGroupTooLong、startNonexistentLocalTime、endNonexistentLocalTime、rangeOrder は文章から読み取れるが、startFormat と endFormat は「開始の形式 → 日付欄（形式）または時刻欄」とあいまいで、コアがこの key を返したとき（画面側の判定とコアの判定がずれたとき）にどちらの欄に出すかが決まらない。日付の妥当性を TypeScript とコアの 2 か所で判定する形になり、食い違いの扱いも未記載。また、エラーを表示するタイミング（起動直後の未選択で理由を出すか、[Fetch] を試みた後か）も定まっていない。画面 1 は起動直後の無効な [Fetch] に理由を出さず、画面 2 は「[Fetch] を見ると」理由が出るとしており、無効なボタンは押せないので理由が永久に出ない読み方もできる。 | 7 つの key すべてについて「key → 欄 → 文言キー」の表を作る（startFormat/endFormat は日付欄・時刻欄のどちらに出すか、画面側とコアが食い違ったときの優先も決める）。あわせて、各理由を出すタイミング（起動時から、または欄に触れた後、など）を 1 つに決めて書く。 | New |
| R-03 | Major | aidlc/spaces/default/intents/261010-console-ui-polish/inception/refined-mockups/interaction-spec.md > §4 実装方針（Q3）の手順 2〜4、mockups.md > 画面 4 の最終段落 | Q3 の「Table で作って測り、満たせなければ戻す」は、合格の数値（絞り込み 100 秒以内、反応 1 秒以内）は書かれているが、測る条件が決まっていない。測る環境（WKWebView の Tauri アプリ上か、ブラウザーか）、機種、行の中身（メッセージの長さ）、「スクロール・展開に反応」を測る操作の定義、要件の NFR2 にある「取得中も固まらない」が手順に入っていないこと、測る人と結果の残し方が不明。Cloudscape の Table を試すのに必要な実装量も大きく、どちらの結果でも画面 4 の見た目を維持できるかが見えにくい。 | 手順 2 に、測定環境（Tauri 実機）、データ（100 万行、メッセージ長の目安）、測る操作（絞り込み・スクロール・行展開・取得中の入力）、合否（いずれか 1 つでも超えたら自前＋トークンへ）を書く。測った数値の記録先も 1 か所に決める。 | New |
| R-04 | Major | aidlc/spaces/default/intents/261010-console-ui-polish/inception/refined-mockups/mockups.md > 画面 1〜4 の「MVP の振る舞い」と、interaction-spec.md > §1〜§7 の「MVP U6/U7 の振る舞い」 | 「MVP の振る舞い」「MVP U6・U7」を何十か所も参照して「維持する」と言うが、維持する振る舞いの一覧がこの記録にない（前回ワークフローの文書は今回の記録に含まれない）。Code Generation の計画担当者は、何を壊してはいけないかを推測するしかなく、受け入れの確認もできない。たとえば取得中に絞り込み欄だけ使える、焦点の戻り先、Enter での取得、確認ダイアログを出す条件などは本文にあるが、他は参照だけ。 | FR4.4 の項目ごとに、維持する振る舞いを「条件 → 結果」の表（または箇条書き）で interaction-spec に 1 か所にまとめ、参照先の MVP の U 番号は不要にする。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261010-console-ui-polish/inception/refined-mockups/mockups.md > 画面 3 の図の注記「(all disabled)」と 画面 1 の上部ナビゲーションの説明、interaction-spec.md > §1 States | 取得中に上部ナビゲーションのどれを無効にするかが食い違う。画面 3 の図は全部無効に見え、§1 は「タイムゾーンは使える」、画面 1 は「接続を変えられないとき」の接続のドロップダウンだけと書く。取得中に日付欄が無効なのにタイムゾーンで値が書き換わる点も未整理（FR1.8 と FR1.5 の関係）。設定ボタンを取得中に無効にするかも画面 1 では触れていない。 | 取得中・確認待ち・設定表示中の各状態で、プロファイル／リージョン／タイムゾーン／設定が有効か無効かの表を 1 つ作り、図の注記と合わせる。 | New |
| R-06 | Minor | aidlc/spaces/default/intents/261010-console-ui-polish/inception/refined-mockups/interaction-spec.md > §4 States（empty、filtered）、§5 dismissed | 絞り込みで 0 件になったときの表示が未定義。empty の文言は「取得前、0 件」用で、絞り込みの 0 件にも同じ「ロググループを選び…」が出ると誤解を招く。また ErrorAlert を Escape で閉じるとあるが、Cloudscape の Alert にその操作はなく、Modal やカレンダーの Escape とも衝突しうる。取得ボタンが取得中に無効になるとき、焦点がどこへ行くかも未記載（NFR3）。 | 絞り込み 0 件の専用文言を足す。Alert の Escape は削除するか実現方法を書く。[Fetch] が無効になったときの焦点の扱いを accessibility-checklist に 1 項目足す。 | New |
| R-07 | Minor | aidlc/spaces/default/intents/261010-console-ui-polish/inception/refined-mockups/design-system-mapping.md > 日付・時刻の部品の設定（DatePicker の行） | Cloudscape の DatePicker の入力形式（スラッシュ／ハイフンの受け付け方）と、タイプ入力のときの扱いが実装前に確認されていない。FR2.4（ハイフンは誤り）と FR2.1（`yyyy/mm/dd`）は部品の既定の振る舞いに左右される。 | 実装の最初の確認項目（AS4 と同じ扱い）として、DatePicker が `2024-03-01` を受けた場合の挙動と、不一致時の回避策（自前の判定を先に通す、など）を追記する。 | New |

### Summary

要件 FR1〜FR4 と NFR2/NFR3/NFR7 はおおむね画面と仕様に割り当てられ、末尾の対応表、アクセシビリティ、状態の網羅も良い。ただし Q4 による AS1 の置き換えが要件本文とまだ矛盾していること、validationErrors と表示タイミングの対応が表になっていないこと、Q3 の測定条件が定まっていないこと、「維持する」と言う MVP の振る舞いの一覧がないことが、Code Generation が推測せずに計画するのを妨げているため NOT-READY とする。
