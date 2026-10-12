# Design System Mapping — ロググループ全体の取得の高速化

使うのは Cloudscape Design System の標準部品だけ（project.md Corrections、business-overview）。新しい見た目の部品は作らない。

| 画面の要素 | Cloudscape の部品 | 使い方 | 今との違い |
|------------|-------------------|--------|------------|
| ［取得］／［中断］ボタン | Button（variant="primary"） | 取得中は文言を「中断」に替える。中断中は `loading` と `loadingText` を使う。キャッシュ保存中は `disabled` | 文言の切り替えと loading の状態を足す |
| 経過 | 状態表示（今の StatusLine）の中の文字 | 今の StatusIndicator の後ろに文字として並べる | 1 項目を足す |
| 取得中の印 | StatusIndicator（type="loading"） | 今と同じ | なし |
| 中断の注意 | Alert（type="warning"） | ログの表の上、今のエラーの表示と同じ場所。`dismissible` は付けない | 新しく出す（部品は既存の Alert） |
| 失敗の件数と詳細 | 今の StatusLine のリンクボタン | 中断のときも同じものを出す | なし |
| 一覧・絞り込み | 今の Table と TextFilter | 中断後も使える | なし |

## 色と記号

- 中断の注意は Cloudscape の warning の色とアイコン。色だけで伝えない（アイコンと文）。
- ダークモードは OS に合わせる今の作りに従う。

## Assumptions & Open Questions

None.
