# 見つかった強い制約（261020-carryover-fixes）

> 人が述べた強い制約だけを `ALWAYS …` / `NEVER …` の形で挙げる。この作業の依頼文と証拠（CI とリリースのワークフロー、`docs/`、コードの知識ベース、git の履歴）には、新しく加えるべき強い制約は見当たらず、面談（Q6 の答え A）でも足すものはないと確かめた。`aidlc/spaces/default/memory/project.md` に affirmed 済みの Forbidden 4 件（秘密の認証情報を書き込まない、読み取り 3 API 以外を呼ばない、テストと CI から AWS に接続しない、テレメトリを送らない）はすでに効いているため、重ねて書かない。「既存のテストはすべて緑のまま」は依頼文の前提だが、team.md の Testing Posture と org.md の `bugfix` スコープの決まりがすでに求めているもので、新しい制約ではない。

## Mandated

None.

## Forbidden

None.
