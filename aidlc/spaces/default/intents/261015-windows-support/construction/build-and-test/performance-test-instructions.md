# Performance Test Instructions — 261015-windows-support

この作業（Windows 対応）には、速さの要件はない。`requirements.md` の NFR1〜NFR6 に速さの目標はなく、`code-generation-plan.md` の Testing Contract にも速さの項目はない。Test Strategy は `Standard` で、速さの指示書は既定では作らないが、`produces` に含まれるため「対象なし」を明記するためにこのファイルを置く。

## 確かめること

- **新しい速さの目標：なし。** Target Verification Matrix には速さの行を置かない。
- **既存の速度測定が今の基準で通ること（NFR1 の回帰の一部）**：100 万件の一覧の速度測定 5 件（`timeline.rs` 2 件、`log_view.rs` 3 件）は、この作業で触っていない。Windows 対応で変えたのは OS の境界の層と CI・ワークフロー・手順書だけで、取得と一覧の経路は変えていない。

```sh
cargo test -p local-sights-core --release --lib -- --ignored
```

通常の `cargo test` では `#[ignore]` で飛ばされる。CI でも走らせない（前の作業と同じ）。このステージでは、変えていない経路の回帰として任意に 1 回走らせる。

## Windows での体感の確認（目標なし）

数値の目標はないが、手元の確認（`integration-test-instructions.md` の「手元の確認」）のついでに、Windows でも取得と一覧のスクロールが macOS と同じように滑らかであることを目で見る。数値は記録しない。
