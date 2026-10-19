#!/usr/bin/env bash
# U2 の Q1 の確かめ：AWS の資料に書かれていない Logs Insights の細部を、CLI で確かめる。
# 開発者の Mac で手で実行する（アプリや CI からは実行しない）。読み取りのクエリだけを流す。
# Logs Insights は読んだ量で課金されるため、期間は短くしておく（既定は直近 1 時間）。
#
# 使い方：
#   PROFILE=my-profile REGION=us-east-1 GROUP=/aws/xxx FIELD=level ./console-check.sh
#   GROUP は JSON のメッセージを含むロググループ。FIELD は、一部のイベントにだけある JSON の項目の名前
#   （入れ子なら a.b）。期間は END（UNIX 秒、既定は今）と START（既定は END の 1 時間前）で変えられる。
#   優先順位の確かめ（4a〜4f）で @message に当てる 3 つの言葉は PA・PB・PC で変えられる（既定は e・o・a）。
#   どのイベントにも入っている言葉だと件数が分かれないため、ログに合わせて選ぶ。
# 出力（件数・時刻・項目の名前・エラーの文、FIELD の値は先頭 12 文字まで。認証情報は出ない）を、
# そのまま貼って返してください。ログのメッセージの中身は出さない。
set -u

: "${PROFILE:?PROFILE を指定してください}"
: "${REGION:?REGION を指定してください}"
: "${GROUP:?GROUP を指定してください}"
: "${FIELD:?FIELD を指定してください（一部のイベントにだけある JSON の項目の名前）}"

END=${END:-$(date +%s)}
START=${START:-$((END - 3600))}
PA=${PA:-e}
PB=${PB:-o}
PC=${PC:-a}

# クエリを流して結果の JSON を返す。失敗したら "ERROR: ..." を返す。
query() {
  local q="$1" id status out
  if ! id=$(aws logs start-query --profile "$PROFILE" --region "$REGION" \
      --log-group-name "$GROUP" --start-time "$START" --end-time "$END" \
      --query-string "$q" --query queryId --output text 2>&1); then
    echo "ERROR: $(printf '%s' "$id" | tail -1)"
    return
  fi
  while :; do
    out=$(aws logs get-query-results --profile "$PROFILE" --region "$REGION" \
      --query-id "$id" --output json)
    status=$(printf '%s' "$out" | jq -r .status)
    case "$status" in Scheduled | Running) sleep 1 ;; *) break ;; esac
  done
  printf '%s' "$out"
}

# 件数（stats count(*) の値）だけを出す。
count() {
  local label="$1" out
  out=$(query "$2")
  case "$out" in
    ERROR:*) echo "[$label] $out" ;;
    *) echo "[$label] $(printf '%s' "$out" | jq -r .status) 件数=$(printf '%s' "$out" | jq -r '.results[0][0].value // "0"')" ;;
  esac
}

# 結果の行の、指定した項目の値を先頭 12 文字まで並べる。
values() {
  local label="$1" field="$2" out
  out=$(query "$3")
  case "$out" in
    ERROR:*) echo "[$label] $out" ;;
    *) echo "[$label] $(printf '%s' "$out" | jq -r --arg f "$field" \
        '[.results[] | (map(select(.field == $f))[0].value // "(なし)") | if $f == "@timestamp" then . else .[0:12] end] | join(" | ")')" ;;
  esac
}

# 結果の 1 行目の列の名前を並べる（値は出さない）。
columns() {
  local label="$1" out
  out=$(query "$2")
  case "$out" in
    ERROR:*) echo "[$label] $out" ;;
    *) echo "[$label] 列=$(printf '%s' "$out" | jq -r '.results[0] // [] | map(.field) | join(",")') 行数=$(printf '%s' "$out" | jq -r '.results | length')" ;;
  esac
}

echo "期間：${START} 〜 ${END}（秒）、FIELD=${FIELD}"

echo "--- 並び順（@timestamp の値。U2 の sort の既定）"
values "1a sort の既定の向き" "@timestamp" "fields @timestamp | sort @timestamp | limit 3"
values "1b sort asc" "@timestamp" "fields @timestamp | sort @timestamp asc | limit 3"
values "1c sort desc" "@timestamp" "fields @timestamp | sort @timestamp desc | limit 3"
values "1d sort なし" "@timestamp" "fields @timestamp | limit 3"

# ONLY=sort のときは並び順だけを確かめて終わる（@timestamp はミリ秒まで出す）。
if [ "${ONLY:-}" = "sort" ]; then
  exit 0
fi

echo "--- fields を書かないときの列"
columns "2a 命令なし" "limit 1"
columns "2b filter だけ" "filter @message like /./ | limit 1"
columns "2c fields を 2 回" "fields @timestamp | fields @logStream | limit 1"

echo "--- 言葉の大文字・小文字"
count "3a FILTER と LIKE" "FILTER @message LIKE /./ | stats count(*)"
count "3b AND・OR・NOT" "filter @message like /./ AND NOT @message like /^$/ | stats count(*)"
count "3c IN" "filter @logStream IN [\"x\"] | stats count(*)"
count "3d Fields・Sort・Limit" "Fields @timestamp | Sort @timestamp Desc | Limit 1 | stats count(*)"

echo "--- 優先順位（a=/${PA}/、b=/${PB}/、c=/${PC}/ を @message に当てる）"
count "4a a or b and c" "filter @message like /${PA}/ or @message like /${PB}/ and @message like /${PC}/ | stats count(*)"
count "4b a or (b and c)" "filter @message like /${PA}/ or (@message like /${PB}/ and @message like /${PC}/) | stats count(*)"
count "4c (a or b) and c" "filter (@message like /${PA}/ or @message like /${PB}/) and @message like /${PC}/ | stats count(*)"
count "4d not a and b" "filter not @message like /${PA}/ and @message like /${PB}/ | stats count(*)"
count "4e (not a) and b" "filter (not @message like /${PA}/) and @message like /${PB}/ | stats count(*)"
count "4f not (a and b)" "filter not (@message like /${PA}/ and @message like /${PB}/) | stats count(*)"

echo "--- ない項目（FIELD は一部のイベントにだけある）"
count "5a 全体" "stats count(*)"
count "5b FIELD がある" "filter ispresent(${FIELD}) | stats count(*)"
count "5c FIELD = \"zzz\"" "filter ${FIELD} = \"zzz\" | stats count(*)"
count "5d FIELD != \"zzz\"" "filter ${FIELD} != \"zzz\" | stats count(*)"
count "5e not FIELD = \"zzz\"" "filter not ${FIELD} = \"zzz\" | stats count(*)"
count "5f FIELD not like /zzz/" "filter ${FIELD} not like /zzz/ | stats count(*)"
count "5g FIELD not in [\"zzz\"]" "filter ${FIELD} not in [\"zzz\"] | stats count(*)"
values "5h sort FIELD asc（FIELD の値）" "${FIELD}" "fields @timestamp, ${FIELD} | sort ${FIELD} asc | limit 3"
values "5i sort FIELD desc（FIELD の値）" "${FIELD}" "fields @timestamp, ${FIELD} | sort ${FIELD} desc | limit 3"
columns "5j fields にない項目" "fields @timestamp, zzz_no_such_field | limit 1"

echo "--- 型の違う値（ログに依らない値で比べる）"
count "6a \"10\" > 9" "fields \"10\" as a, 9 as b | filter a > b | stats count(*)"
count "6b \"10\" = 10" "fields \"10\" as a | filter a = 10 | stats count(*)"
count "6c \"abc\" > 5" "fields \"abc\" as a | filter a > 5 | stats count(*)"
count "6d \"abc\" < 5" "fields \"abc\" as a | filter a < 5 | stats count(*)"
count "6e \"abc\" != 5" "fields \"abc\" as a | filter a != 5 | stats count(*)"
count "6f \"10\" > \"9\"" "fields \"10\" as a | filter a > \"9\" | stats count(*)"
count "6g @logStream = 1" "filter @logStream = 1 | stats count(*)"
count "6h @logStream != 1" "filter @logStream != 1 | stats count(*)"

echo "--- limit"
count "7a limit 100000" "fields @timestamp | limit 100000 | stats count(*)"
count "7b limit 100001" "fields @timestamp | limit 100001 | stats count(*)"
count "7c limit 0" "fields @timestamp | limit 0 | stats count(*)"
