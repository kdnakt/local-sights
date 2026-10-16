# Intent Capture Questions — Windows 対応（IB-16）

## Sources

- [desc] Initial description: "Windows 対応をしたい（IB-16）。今は macOS（Apple Silicon）だけに配っているが、Windows でも動かして配れるようにしたい。終わったら次は Logs Insights 互換のクエリ（IB-11）をやる予定。"
- [scope] Workflow-selected scope: `bugfix`.
- [memory:M1] `aidlc/spaces/default/memory/team.md#Deployment`: "配布の対象：macOS（Apple Silicon）だけとする。Intel の Mac と Windows には配らない。"
- [memory:M2] `aidlc/spaces/default/memory/team.md#Deployment`: "署名と公証：今は Apple の署名・公証をしない。初回起動時に出る Gatekeeper の警告（「開発元を確認できません」）を開く方法を手順書に書く。会社のルールで未署名のアプリが禁止されていると分かったら見直す。"

## Q1. Windows で動かしたい理由は何ですか？

背景：何のための Windows 対応なのかを記録します。理由によって、どこまで作り込むか（自分が動かせれば足りるのか、ほかの人に配れる形まで要るのか）が変わります。

A. 会社の PC に Windows のものがあり、そこでも使いたい
B. 同僚に Windows を使っている人がいて、その人たちにも使ってほしい
C. OSS として公開しているので、Windows の利用者にも使えるようにしたい
D. 上のいくつか（具体的に書いてください）
E. Not identified
X. Other (please specify)

[Answer]: A

## Q2. Windows 版を使うのは誰ですか？

背景：これまでの利用者は、開発者本人・同じ会社の同僚・OSS の利用者でした。Windows 版で想定する利用者を確かめます。

A. 開発者本人だけ
B. 開発者本人と、同じ会社で Windows を使う同僚
C. 開発者本人と、OSS として使う不特定の Windows 利用者
D. 上の全員
E. Not identified
X. Other (please specify)

[Answer]: D

## Q3. 「Windows に対応できた」と言える目安は何ですか？

背景：終わったと判断する基準を、確かめられる形で決めるためです。

A. GitHub Releases から Windows 用のインストーラを取得して入れ、ログを取得・表示できる
B. A に加えて、AWS の SSO ログインのボタンも Windows で動く
C. B に加えて、macOS 版とできることがすべて同じ（キャッシュ・中断・経過時間の表示なども含む）
D. ソースからビルドして Windows で動けばよい（配布物はまだ要らない）
E. Not yet defined
X. Other (please specify)

[Answer]: C

## Q4. Windows で実際に動かして確かめられる環境はありますか？

背景：開発は macOS で進めており、自動テストと CI は実際の AWS に接続しません。チームの決まりでは、薄い一本の確認として、本人の AWS 認証情報で取得を行い、GUI で結果を目で確かめることになっています。Windows でこれができるかどうかで、確かめ方と進め方が変わります。

A. 手元に Windows の PC がある（AWS の認証情報も使える）
B. 会社の Windows の PC で確かめられる（インストールはできる）
C. 仮想マシンやクラウドの Windows で確かめられる
D. 実機で確かめる手段はない（CI のビルドとテストだけで判断する）
E. Not yet defined
X. Other (please specify)

[Answer]: A

## Q5. 対象にする Windows の種類はどれですか？

背景：配布物の作り方と確かめる範囲を決めるためです。いまの macOS 版は Apple Silicon だけに配っています [memory:M1]。

A. Windows 11（x64）だけ
B. Windows 10 と 11（x64）
C. Windows 11 の x64 と arm64
D. Windows 10 と 11 の x64 と arm64
E. Not yet defined
X. Other (please specify)

[Answer]: A

## Q6. 今回の Windows 版に含めないものはどれですか？（select all that apply）

背景：作業の範囲を区切るためです。いまの macOS 版は Apple の署名・公証をせず、警告の開き方を手順書に書いています [memory:M2]。

A. Windows のコード署名（SmartScreen の警告が出る前提で、開き方を手順書に書く）
B. 自動更新の仕組み
C. Intel の Mac と Linux への配布
D. Windows ストア（Microsoft Store）での配布
E. None（どれも含める）
X. Other (please specify)

[Answer]: A, B, C, D

## Q7. この作業の進め方を決める人と、進み具合の共有はどうしますか？

背景：これまでは開発者本人が決め、進み具合の共有は GitHub の PR と README 程度で足りていました。今回も同じかを確かめます。

A. これまでと同じ（本人が決め、共有は PR と README で足りる）
B. 本人が決めるが、Windows を使う同僚に途中で試してもらいたい
C. 会社の誰かの了解（例：情報システム部門のインストールの許可）が要る
D. B と C の両方
E. Not identified
X. Other (please specify)

[Answer]: A

## Q8. 会社の Windows の PC に、署名のないアプリを入れてもよいですか？

背景：Windows で使いたい理由は会社の PC で使うためで（Q1）、今回はコード署名をしません（Q6）。macOS 版では、会社のルールで未署名のアプリが禁止されていると分かったら見直す、と決めています [memory:M2]。Windows の会社の PC でも同じ前提でよいかを確かめます。

A. 入れてよい（SmartScreen の警告を開けば使える）
B. 入れてよいかは分からないが、まずは未署名で作り、禁止と分かったら見直す
C. 会社の PC では未署名のアプリを入れられない（署名が要る）
D. 会社の PC には管理者の許可がないとアプリを入れられない
E. Not yet defined
X. Other (please specify)

[Answer]: A
