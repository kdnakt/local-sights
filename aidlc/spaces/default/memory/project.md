# Project-Level Rules

> Project-specific specialisation and corrections. Loaded after `org.md` and
> `team.md` as strict-additive guidance; contradictions with broader policy
> are rejected. Populated by practices-discovery and the self-learning loop.
>
> Use sparingly: most teams don't need a project layer. Reach for it
> only when this specific project needs stable, durable guidance beyond the
> team practice (for example, package-specific release checks or an additional
> regression suite for a legacy component).

## Way of Working

<!-- Project-specific specialisation. Example: -->
<!-- This monorepo requires package-scoped branch names and a package owner -->
<!-- review in addition to the team's normal merge policy. -->

## Walking Skeleton

<!-- Project-specific specialisation. Example: -->
<!-- The walking skeleton must exercise the legacy service adapter as well -->
<!-- as the new service boundary. -->

## Testing Posture

<!-- Project-specific specialisation. -->

## Guard Policy

<!-- Project-specific. Mode: strict, relaxed, or off. Strict here holds for every intent and cannot be changed from chat. A section under the retired Change Control heading, written by an earlier release, is still read. -->

## Deployment

<!-- Project-specific specialisation. -->

## Code Style

<!-- Project-specific specialisation. -->

## Tech Stack

<!-- Technology choices locked for this project. -->

## Decided

<!-- Decisions made in earlier stages that should not be re-asked. -->
<!-- Format: DECIDED: [decision] (Stage [slug], [date]) -->

## Scope Overrides

<!-- Custom scope rules for this project. -->

## Forbidden

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: NEVER [behavior] (affirmed [date]) -->
<!-- Example: NEVER throw exceptions across service layer boundaries (affirmed 2026-05-17) -->

- NEVER 秘密の認証情報（シークレットアクセスキー・セッショントークン・SSO のトークン）とアクセスキー ID を、リポジトリ・アプリのログ・画面のエラー表示・キャッシュファイルに書き込む（プロファイル名・ロール名／ロール ARN・アカウント ID はログやエラー表示に出してよいが、リポジトリには置かない） (affirmed 2026-10-03)

- NEVER CloudWatch Logs の読み取り API（DescribeLogGroups / DescribeLogStreams / GetLogEvents）以外の API を呼ぶ (affirmed 2026-10-03)

- NEVER 自動テストや CI から実際の AWS に接続する、または CI に AWS の認証情報を置く (affirmed 2026-10-03)

- NEVER テレメトリやクラッシュレポートを外部に送る (affirmed 2026-10-03)

## Mandated

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: ALWAYS [behavior] (affirmed [date]) -->
<!-- Example: ALWAYS use Result<T,E> for fallible operations in service layer (affirmed 2026-05-17) -->

None. (affirmed 2026-10-03)

## Corrections

<!-- Project-specific corrections from human feedback. -->
<!-- Format: NEVER/ALWAYS [behavior] (learned [date]) -->
- 「Rust製」「GetLogEvents API」は利用者自身が指定した制約として扱い、ideation 成果物でも実装詳細ではなく前提条件として残す (learned 2026-10-02) <!-- cid:261002-cwlogs-viewer:intent-capture:74d4398b24a4f112f577207863f10250ccf36e3bb70cba67a834eb98c33dc7ee -->
- ALWAYS 質問は一問ずつ構造化質問で提示する（利用者が Chat モードを選んでも、論点をまとめて投げず一問ずつ聞く） (learned 2026-10-03) <!-- cid:261002-cwlogs-viewer:feasibility:05a9352b368ecb7c5eb634bb983900899de8d8f71c8fdc06ff5f6fe29b4b0e75 -->
- ALWAYS 進捗表示を対象外にした場合でも、処理中であることを示す最低限の状態表示（例：Fetching 表示とボタン無効化）は画面に含める (learned 2026-10-03) <!-- cid:261002-cwlogs-viewer:rough-mockups:a49c7eff6e1d6606f95c463f6bdc84b675b0e15c6d554db57e55516124d026b7 -->
- デスクトップ GUI の MVP は、見た目を標準部品のみ・作り込みなしとして期限を優先する (learned 2026-10-03) <!-- cid:261002-cwlogs-viewer:rough-mockups:b9a3c28dbc3b57f7f1b6aa4f5d8e57dbea355bb53f4f21dd2e76aa55c5430c24 -->
- ALWAYS 部品（コンポーネント）は書くコードだけにし、AWS SDK・外部 API・ファイルシステム・GUI フレームワークは外部依存として扱い、それらに触れる境界のコードを部品にする (learned 2026-10-03) <!-- cid:261002-cwlogs-viewer:domain-design:735533c17cfc1b86b464424e28676424e2c8c9a75125564547127189b4c88c20 -->
