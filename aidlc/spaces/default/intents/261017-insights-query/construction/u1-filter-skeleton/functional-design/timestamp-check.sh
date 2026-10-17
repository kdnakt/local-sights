#!/usr/bin/env bash
# Q2 の確かめ：Logs Insights が filter の中の @timestamp をどう比べるかを、CLI で確かめる。
# 開発者の Mac で手で実行する（アプリや CI からは実行しない）。読み取りのクエリだけを流す。
# Logs Insights は読んだ量で課金されるため、期間は短くしておく（既定は直近 1 時間）。
#
# 使い方：
#   PROFILE=my-profile REGION=ap-northeast-1 GROUP=/aws/lambda/my-func ./timestamp-check.sh
#   期間は END（UNIX 秒、既定は今）と START（既定は END の 1 時間前）で変えられる。
# 出力（件数と状態だけ。認証情報は出ない）を、そのまま貼って返してください。
set -u

: "${PROFILE:?PROFILE を指定してください}"
: "${REGION:?REGION を指定してください}"
: "${GROUP:?GROUP を指定してください}"

END=${END:-$(date +%s)}
START=${START:-$((END - 3600))}
MID_S=$(((START + END) / 2))
MID_MS=$((MID_S * 1000))
MID_UTC=$(date -u -r "$MID_S" '+%Y-%m-%d %H:%M:%S.000')

run() {
  local label="$1" q="$2" id status out
  if ! id=$(aws logs start-query --profile "$PROFILE" --region "$REGION" \
      --log-group-name "$GROUP" --start-time "$START" --end-time "$END" \
      --query-string "$q" --query queryId --output text 2>&1); then
    echo "[$label] start-query がエラー: $id"
    return
  fi
  while :; do
    out=$(aws logs get-query-results --profile "$PROFILE" --region "$REGION" \
      --query-id "$id" --output json)
    status=$(printf '%s' "$out" | jq -r .status)
    case "$status" in Scheduled | Running) sleep 1 ;; *) break ;; esac
  done
  echo "[$label] $status 件数=$(printf '%s' "$out" | jq -r '.results[0][0].value // "0"')"
}

echo "期間：${START} 〜 ${END}（秒）、境目：${MID_MS} ms = ${MID_UTC} UTC"
run "1 全体" "stats count(*)"
run "2 toMillis と数値" "filter toMillis(@timestamp) > $MID_MS | stats count(*)"
run "3 そのまま数値" "filter @timestamp > $MID_MS | stats count(*)"
run "4 日時の文字列" "filter @timestamp > \"$MID_UTC\" | stats count(*)"
run "5 ISO の文字列" "filter @timestamp > \"$(date -u -r "$MID_S" '+%Y-%m-%dT%H:%M:%S.000Z')\" | stats count(*)"
