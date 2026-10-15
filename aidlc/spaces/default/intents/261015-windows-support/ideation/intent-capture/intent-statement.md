# Intent Statement — Windows 対応（IB-16）

## Problem Statement

このアプリは今、macOS（Apple Silicon）だけに配っている。Windows では動かせず、配る手段もない。 [desc] [memory:M1]

開発者は会社の PC のうち Windows のものでもこのアプリを使いたい。 [Q1]

この作業は、Windows でもアプリを動かし、配れるようにすることをねらう。 [desc]

## Target Customer

| 利用者 | 得られること | Source |
|--------|--------------|--------|
| 開発者本人 | 会社の Windows の PC でも、macOS と同じようにログを見られる | [Q1] [Q2] |
| 同じ会社で Windows を使う同僚 | Windows の PC でこのアプリを使える | [Q2] |
| OSS としてこのアプリを使う不特定の Windows 利用者 | Windows でこのアプリを使える | [Q2] |

## Success Metrics

| 指標 | 目安 | Source |
|------|------|--------|
| 配布物の取得と導入 | GitHub Releases から Windows 用のインストーラを取得して入れられる | [Q3] |
| 取得と表示 | Windows で、ログを取得して表示できる | [Q3] |
| SSO ログイン | AWS の SSO ログインのボタンが Windows でも動く | [Q3] |
| macOS 版とそろうこと | キャッシュ・中断・経過時間の表示なども含め、macOS 版でできることが Windows 版でもすべてできる | [Q3] |
| 確かめ方 | 開発者の手元の Windows の PC で、本人の AWS の認証情報を使って実際に動かして確かめる | [Q4] |

## Initiative Trigger

会社の Windows の PC でもこのアプリを使いたいことがきっかけである。 [Q1]

次に予定している Logs Insights 互換のクエリ（IB-11）より先に、この作業を終える。 [desc]

## Initial Scope Signal

### Workflow-selected scope

作業の進め方は `bugfix` をもとにした計画が選ばれている（workflow-selected）。 [scope]

### User-confirmed product boundary

| 区分 | 項目 | Source |
|------|------|--------|
| 含める | Windows でアプリを動かし、GitHub Releases で配れるようにする | [desc] [Q3] |
| 対象 | Windows 11（x64）だけ | [Q5] |
| 含めない | Windows のコード署名。SmartScreen の警告が出る前提で、開き方を手順書に書く | [Q6] |
| 含めない | 自動更新の仕組み | [Q6] |
| 含めない | Intel の Mac と Linux への配布 | [Q6] |
| 含めない | Microsoft Store での配布 | [Q6] |
| 前提 | 会社の Windows の PC に署名のないアプリを入れてよい。SmartScreen の警告を開けば使える | [Q8] |

## Assumptions & Open Questions

None.
