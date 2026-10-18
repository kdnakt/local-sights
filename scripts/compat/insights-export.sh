#!/usr/bin/env bash
# 261017-insights-query U4: exports the console's Logs Insights results of the
# query set, for compat_check to compare with the app (BR1.1-BR1.4).
#
# Run it by hand on your own machine with your own credentials (the app and CI
# never run it). It calls only `aws logs start-query` and
# `aws logs get-query-results`, one query at a time, as the U2 console check did.
# Logs Insights is charged by the amount of log data it reads, for every query:
# keep the period short.
#
# Usage:
#   PROFILE=my-profile REGION=us-east-1 GROUP=CloudTrail START=<epoch s> END=<epoch s> \
#     OUT=~/insights-export ./scripts/compat/insights-export.sh
#   cargo run -p local-sights-core --example compat_check -- --export-dir ~/insights-export
#
#   PROFILE  optional; without it the AWS CLI's default settings are used.
#   START, END  the period in epoch seconds, given to start-query as they are.
#            END must be at least 30 minutes ago, so late logs do not make the
#            two sides see different events (BR1.3). Choose START and END
#            seconds with few events: how Logs Insights takes the end second
#            has not been checked, and compat_check marks a query whose result
#            has events in either boundary second (review R-03).
#   OUT      a folder OUTSIDE the repository (created when missing). It gets
#            settings.json and one <id>.json for every query; they hold log
#            contents, so never put them in the repository (BR1.2, NFR4).
#            Every file of one run carries the same exportedAtSeconds, and
#            compat_check does not compare an <id>.json whose value differs
#            from settings.json, so files left by an earlier run in the same
#            folder are never taken for this run's (code review R-02).
#   QUERIES  optional; the query set (default
#            crates/local-sights-core/examples/compat_queries.txt). Give
#            compat_check the same file with --queries.
#   WAIT_SECONDS  optional; how long to wait for one query (default 600).
#
# Every query is started with --limit 10000: compat_check assumes that cap
# for a query without its own `limit`, the same as the app (code review R-03).
#
# Output: one line a query with its status, then the counts. No credentials and
# no log contents are printed. Exit codes: 0 every query ended Complete or was
# refused by start-query as a malformed query (Error, compared by
# compat_check), 1 some query ended otherwise (start-query failed for another
# reason such as expired credentials, or Failed, Cancelled, Timeout, Unknown,
# still Running; compat_check does not compare those), 2 wrong settings
# (nothing is run).
set -u
export LC_ALL=C

fail() {
  echo "error: $*" >&2
  exit 2
}

PROFILE=${PROFILE:-}
REGION=${REGION:-}
GROUP=${GROUP:-}
START=${START:-}
END=${END:-}
OUT=${OUT:-}
WAIT_SECONDS=${WAIT_SECONDS:-600}

command -v aws >/dev/null 2>&1 || fail "the AWS CLI (aws) is not found"
command -v jq >/dev/null 2>&1 || fail "jq is not found"

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd -P) || fail "the script's folder cannot be resolved"
REPO=$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null) ||
  fail "the repository's top folder cannot be found (git rev-parse)"
QUERIES=${QUERIES:-$REPO/crates/local-sights-core/examples/compat_queries.txt}

case "$REGION" in '' | *[!a-z0-9-]*) fail "REGION must be a region code such as us-east-1" ;; esac
[ -n "$GROUP" ] || fail "GROUP is required"
case "$START" in '' | *[!0-9]*) fail "START must be epoch seconds" ;; esac
case "$END" in '' | *[!0-9]*) fail "END must be epoch seconds" ;; esac
case "$WAIT_SECONDS" in '' | *[!0-9]*) fail "WAIT_SECONDS must be a number of seconds" ;; esac
[ "$START" -lt "$END" ] || fail "START must be before END"
NOW=$(date +%s)
# BR1.3: the period ended at least 30 minutes ago.
[ "$END" -le $((NOW - 1800)) ] || fail "END must be at least 30 minutes ago (now $NOW)"
[ -n "$OUT" ] || fail "OUT is required (a folder outside the repository)"
[ -f "$QUERIES" ] || fail "the query set $QUERIES is not found"

# BR1.2, review R-05: the real place of OUT (symbolic links resolved) must not
# be the repository's top folder or inside it. A missing OUT is resolved
# through its parent folder, before anything is created.
resolve_folder() {
  if [ -d "$1" ]; then
    (cd "$1" && pwd -P)
  else
    local parent
    parent=$(dirname "$1")
    [ -d "$parent" ] || return 1
    printf '%s/%s\n' "$(cd "$parent" && pwd -P)" "$(basename "$1")"
  fi
}
OUT_REAL=$(resolve_folder "$OUT") || fail "the parent folder of OUT does not exist"
REPO_REAL=$(cd "$REPO" && pwd -P) || fail "the repository's folder cannot be resolved"
out_seen=$OUT_REAL
repo_seen=$REPO_REAL
case "$(uname -s)" in
  # File names ignore case there by default.
  Darwin | MINGW* | MSYS* | CYGWIN*)
    out_seen=$(printf '%s' "$OUT_REAL" | tr '[:upper:]' '[:lower:]')
    repo_seen=$(printf '%s' "$REPO_REAL" | tr '[:upper:]' '[:lower:]')
    ;;
esac
case "$out_seen/" in
  "$repo_seen/"*) fail "OUT ($OUT_REAL) is inside the repository; use a folder outside it" ;;
esac
mkdir -p "$OUT_REAL" || fail "OUT cannot be created"

# start-query's standard error, kept apart from its standard output (the
# query ID) and read only when it fails (code review R-01, R-04).
ERR_FILE=$(mktemp "${TMPDIR:-/tmp}/insights-export.XXXXXX") ||
  fail "a temporary file cannot be made"
trap 'rm -f "$ERR_FILE"' EXIT

PROFILE_ARGS=()
if [ -n "$PROFILE" ]; then
  PROFILE_ARGS=(--profile "$PROFILE")
fi
# `${a[@]+"${a[@]}"}`: an empty array under `set -u` on bash 3.2 (macOS).
aws_logs() {
  aws logs "$@" ${PROFILE_ARGS[@]+"${PROFILE_ARGS[@]}"} --region "$REGION"
}

# BR1.4: the conditions, for compat_check to fetch the same period.
# `$end` is not a valid name on jq 1.6 (`end` is a keyword there): use
# start_seconds and end_seconds.
jq -n --arg profile "$PROFILE" --arg region "$REGION" --arg group "$GROUP" \
  --argjson start_seconds "$START" --argjson end_seconds "$END" --argjson at "$NOW" \
  '{profile: (if $profile == "" then null else $profile end), region: $region,
    logGroup: $group, startSeconds: $start_seconds, endSeconds: $end_seconds,
    exportedAtSeconds: $at}' \
  >"$OUT_REAL/settings.json" || fail "settings.json cannot be written"

# The last line of the CLI's error without its request ID (entities.md
# InsightsExport.message).
refusal_message() {
  tail -n 1 | sed -E 's/[[:space:]]*\(?[Rr]equest[ _-]?[Ii][Dd][:=]?[[:space:]]*[0-9A-Fa-f-]{8,}\)?//g'
}

TAB=$(printf '\t')
CR=$(printf '\r')
ran=0
complete=0
refused=0
other=0
line_number=0
seen_ids=" "
while IFS= read -r line || [ -n "$line" ]; do
  line_number=$((line_number + 1))
  line=${line%"$CR"}
  trimmed=${line#"${line%%[![:space:]]*}"}
  case "$trimmed" in '' | '#'*) continue ;; esac
  # id<TAB>order|set<TAB>purpose<TAB>query, as compat_check reads it.
  id=${line%%"$TAB"*}
  rest=${line#*"$TAB"}
  mode=${rest%%"$TAB"*}
  rest=${rest#*"$TAB"}
  rest=${rest#*"$TAB"}
  query=$rest
  case "$id" in '' | *[!A-Za-z0-9-]*) fail "$QUERIES line $line_number: bad id" ;; esac
  case "$mode" in order | set) ;; *) fail "$QUERIES line $line_number: bad mode" ;; esac
  case "$seen_ids" in *" $id "*) fail "$QUERIES line $line_number: the id $id is used twice" ;; esac
  seen_ids="$seen_ids$id "
  [ -n "$query" ] || fail "$QUERIES line $line_number: empty query"

  ran=$((ran + 1))
  file="$OUT_REAL/$id.json"
  # 261017-insights-query U4 code review R-01: only a query Logs Insights
  # cannot compile is its refusal (Error, compared by BR2.4). Any other
  # failure of start-query (expired or missing credentials, access denied,
  # throttling, no such log group, the network) means the query was not run:
  # it is written as Unknown and counted as other, so compat_check does not
  # compare it.
  # Code review R-03: --limit 10000, the row cap compat_check takes for a
  # query without `limit` (U2:BR5.1), instead of StartQuery's own default.
  # Code review R-04: standard error goes to ERR_FILE, so a warning of the
  # CLI never becomes part of the query ID.
  if ! query_id=$(aws_logs start-query --log-group-name "$GROUP" --start-time "$START" \
    --end-time "$END" --limit 10000 --query-string "$query" --query queryId --output text \
    2>"$ERR_FILE"); then
    message=$(refusal_message <"$ERR_FILE")
    if grep -q 'MalformedQueryException' "$ERR_FILE"; then
      jq -n --arg message "$message" --argjson at "$NOW" \
        '{status: "Error", message: $message, exportedAtSeconds: $at}' >"$file"
      refused=$((refused + 1))
      echo "[$id] Error"
    else
      jq -n --arg message "$message" --argjson at "$NOW" \
        '{status: "Unknown", message: $message, results: [], exportedAtSeconds: $at}' >"$file"
      other=$((other + 1))
      echo "[$id] Unknown (start-query failed)"
    fi
    continue
  fi
  waited=0
  result=""
  status=Unknown
  while :; do
    if ! result=$(aws_logs get-query-results --query-id "$query_id" --output json 2>/dev/null); then
      result=""
      status=Unknown
      break
    fi
    status=$(printf '%s' "$result" | jq -r '.status // "Unknown"')
    case "$status" in Scheduled | Running) ;; *) break ;; esac
    # Still running when the wait ends: written as it is, not compared.
    [ "$waited" -lt "$WAIT_SECONDS" ] || break
    sleep 1
    waited=$((waited + 1))
  done
  if [ -n "$result" ]; then
    printf '%s' "$result" | jq --argjson at "$NOW" \
      '{status: (.status // "Unknown"), results: (.results // []), exportedAtSeconds: $at}' >"$file"
  else
    jq -n --argjson at "$NOW" '{status: "Unknown", results: [], exportedAtSeconds: $at}' >"$file"
  fi
  if [ "$status" = Complete ]; then
    complete=$((complete + 1))
  else
    other=$((other + 1))
  fi
  echo "[$id] $status rows=$(jq '.results | length' "$file")"
done <"$QUERIES"

echo "queries: $ran  Complete: $complete  Error: $refused  other: $other  (folder: $OUT_REAL)"
[ "$other" -eq 0 ] || exit 1
exit 0
