# Practices Discovery の面談（261020-carryover-fixes）

再実行の面談です。チームの進め方の 5 節（Way of Working・Walking Skeleton・Testing Posture・Deployment・Code Style）のうち、証拠と食い違ったのは Deployment 節の「次の版は `v0.2.0`」の記述だけでした（2026-10-20 に v0.2.0 を公開済み）。ほかの 4 節は今の team.md の文面のままを案にしています。下書きは `team-practices.md`、根拠は `evidence.md` にあります。

## Q1. 次の版の番号

v0.2.0 のあとに配る次の版のタグをどうしますか。今回の作業で利用者に見える変更は、接続の画面のリージョンの選択欄に名前を出して検索できるようにする点だけで、保存形式や API は変えません。

A. `v0.2.1` とする（パッチの版。下書きの案）
B. `v0.3.0` とする（マイナーの版）
C. この作業では番号を決めず、タグを打つ直前に決める（team.md には「次の版はこの作業が終わったあとに決める」と書く）
X. Other (please specify)

[Answer]: C

## Q2. タグを打つ時期

次の版のタグはいつ打ちますか。

A. この作業が `main` にマージされたら、手順書どおり（版を上げる PR → Release dry run → 手元の確認 → タグ）すぐ打つ
B. 次の機能（IB-10 や IB-09 など）が終わるのを待って、合わせて配る
C. この作業では決めない
X. Other (please specify)

[Answer]: B

## Q3. Deployment 節の細部 (select all that apply)

Deployment 節の「次の版」の箇条書きを書き換えるにあたり、次のうち入れたいものを選んでください。

A. v0.2.0 のあとに `main` に入った PR #35（IB-11 の記録の更新）も次の版に含まれる、と書く
B. 「GitHub Actions がテストを流してから」を `release.yml` の実際（ライブラリと画面のテストを Linux のランナーで）に合わせて細かく書く
C. 公開済みの版の一覧（`v0.1.0`・`v0.1.1`・`v0.2.0`）を独立した箇条書きにし、「一度公開したタグは付け替えも使い回しもしない」をそこに添える
D. どれも入れない（次の版の箇条書きの書き換えだけにする）
X. Other (please specify)

[Answer]: C

## Q4. ほかの 4 節

Way of Working・Walking Skeleton・Testing Posture・Code Style の 4 節は、証拠（CI のジョブ名、80% のカバレッジ、cargo-deny、`.gitattributes` の LF、インストーラの形など）と一致していました。文面は変更なしでよいですか。

A. 4 節とも変更なし
B. 変えたい節がある（X で節と内容を書いてください）
X. Other (please specify)

[Answer]: A

## Q5. Windows 11 の手元の確認を飛ばす扱い

Walking Skeleton の節は「両方の OS に効く変更では、macOS と Windows の両方で手元の確認を行う」と決めています。今回の作業は Windows の手元の確認を飛ばします。この扱いをどう残しますか。

A. この作業だけの例外として、作業の記録（要件と Build and Test）に残す。節の文面は変えない
B. 節の文面に「利用者に見える変更が小さい作業では、開発者の判断で Windows の手元の確認を省いてよい」のような条件を足す
X. Other (please specify)

[Answer]: A

## Q6. 新しい強い制約

今回、新しい強い制約（ALWAYS / NEVER で書くもの）は見当たりませんでした。Mandated・Forbidden はどちらも `None.` でよいですか。既存の 4 つの Forbidden（認証情報を書かない、読み取り 3 API 以外を呼ばない、テストや CI から AWS につながない、テレメトリを送らない）はそのまま残ります。

A. どちらも `None.` でよい
B. 足したいものがある（X で書いてください）
X. Other (please specify)

[Answer]: A
