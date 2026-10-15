# Build and Test Questions — 261012-parallel-fetch

## Q1 手元の確認

自動のビルド・テスト・検査はすべて成功しました。目標 89 件のうち 5 件（実際の AWS での速さ比べ U1-NFR2.1、GUI の経過 U4-NFR2.3、スリープをまたいだ経過 U4-NFR6.7、VoiceOver U4-NFR9.2・U5-NFR9.4）は、GUI も実際の AWS も使えないコンテナでは確かめられません。どう進めますか？

A. 手元の Mac で確認して、結果を伝える (Recommended)
B. 今は確認しない（5 件は Unverified のまま。このステージは失敗の扱いになり、次の手を相談する）
X. Other (please specify)

[Answer]: A

## Q2 VoiceOver の確認

VoiceOver は macOS の読み上げ機能です。要件（requirements.md の NFR9・NFR10）には読み上げの条件はなく、U4 の NFR9.2 と U5 の NFR9.4 の「確かめ方」に「GUI の確認で VoiceOver を一度使う」と書いた部分だけが残っています。読み上げのための属性（aria-hidden、role=status）は、画面のテストですでに確かめて通っています。どうしますか？

A. VoiceOver の確認はしない。2 件は画面のテストの結果で Met とし、確かめ方から VoiceOver を外したことを記録に残す (Recommended)
B. 一度だけ VoiceOver で確かめる（数分）
X. Other (please specify)

[Answer]: A
