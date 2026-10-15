# Discovered Rules

人間が明言した、例外のないハードな制約だけを記録する。確定済みの 4 つの `NEVER`（秘密の認証情報、CloudWatch Logs の読み取り API 以外、テストや CI から実際の AWS、テレメトリ）はすでに `project.md` に入っているため、ここには重ねて書かない。

intent-capture の回答（Q5・Q6・Q8）で決まった「Windows 11（x64）だけ」「Windows のコード署名・自動更新・Intel の Mac と Linux・Microsoft Store は含めない」は、今回の作業の範囲の区切りで、人間が今後もずっと守る制約として述べたものではない。そのため `ALWAYS` / `NEVER` の決まりにはせず、`team-practices.md` の Deployment の記述（配布の対象と形、署名）として扱う。面談（Q1〜Q8）の答えも、インストーラの形・CI・確認の OS・リリースの作り方・次の版の番号・対象の版・手順書・改行コードという慣行の選択で、例外のない制約として述べたものはない。「一度公開したタグは付け替えない」も Deployment の慣行として `team-practices.md` に書く。

## Mandated

None.

## Forbidden

None.
