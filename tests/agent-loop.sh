#!/usr/bin/env bash
# Cross-agent smoke test against an INSTALLED redline: a headless agent session registers an artifact
# over its redline MCP server, waits for comments, gets one submitted the way the browser does (with
# the artifact token), and replies over MCP. Passes once the reply is in the thread.
#
#   tests/agent-loop.sh claude                 # one agent
#   tests/agent-loop.sh claude opencode agy    # several at once, sharing one daemon
#
# Agents: claude, codex, opencode, agy, cursor-agent, gemini.
# Env: REDLINE (CLI, default `redline` on PATH), LOOP_TIMEOUT (s, default 300),
#      CODEX_MODEL (codex -m), CLAUDE_CONFIG_DIR (a second Claude profile), KEEP_LOGS=1.
# Needs: jq, curl. Read-only for configs: it only starts agent sessions and talks to the daemon.
set -uo pipefail

if (($# == 0)); then
  sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
  exit 2
fi

if (($# > 1)); then
  pids=() fails=0
  for a in "$@"; do "$0" "$a" & pids+=($!); sleep 1; done
  for p in "${pids[@]}"; do wait "$p" || fails=$((fails + 1)); done
  echo "== $(($# - fails))/$# passed"
  exit $((fails > 0))
fi

agent=$1
R=${REDLINE:-redline}
limit=${LOOP_TIMEOUT:-300}
tag="loop-$agent-$(date +%s)-$$"
work="$(mktemp -d)"
out="$work/$agent.log"
command -v "$R" >/dev/null || { echo "FAIL $agent: redline CLI not found (set REDLINE=<clone>/bin/redline)"; exit 1; }

prompt="This is an automated test of the redline MCP server. Use the redline MCP tools directly (not the shell, not the redline CLI).
1. Call register_artifact with title \"$tag\" and uri \"file://$work/$tag.html\".
2. Call wait_for_comments with that artifact_id and timeout_s 240. If it returns no events, call it once more.
3. For the first user comment you receive, call reply on its thread_id with body \"ack-$agent: \" followed by the comment text.
4. Then stop and print the artifact_id. Do not create or edit any files."

# opencode run starts a persistent `opencode serve`; remember which ones were already running.
before_serve="$(pgrep -f 'opencode serve' 2>/dev/null | sort)"

cd "$work" || exit 1
case $agent in
  claude) claude -p "$prompt" --allowedTools "mcp__redline" >"$out" 2>&1 & ;;
  codex) codex exec --skip-git-repo-check ${CODEX_MODEL:+-m "$CODEX_MODEL"} \
    -c 'mcp_servers.redline.default_tools_approval_mode="approve"' "$prompt" >"$out" 2>&1 & ;;
  opencode) opencode run --auto "$prompt" >"$out" 2>&1 & ;;
  agy) agy --dangerously-skip-permissions -p "$prompt" >"$out" 2>&1 & ;; # flags before -p
  cursor-agent) cursor-agent -p --force "$prompt" >"$out" 2>&1 & ;;
  gemini) gemini -y -p "$prompt" >"$out" 2>&1 & ;;
  *) echo "FAIL $agent: unknown agent (claude, codex, opencode, agy, cursor-agent, gemini)"; exit 2 ;;
esac
pid=$!

finish() { # code message
  kill "$pid" 2>/dev/null
  if [[ $agent == opencode ]]; then # only this test may: in a parallel run the others' serve is opencode's
    for p in $(comm -13 <(echo "$before_serve") <(pgrep -f 'opencode serve' 2>/dev/null | sort)); do kill "$p" 2>/dev/null; done
  fi
  echo "$2"
  if (($1 != 0)); then echo "--- last lines of $out"; tail -25 "$out"; fi
  [[ "${KEEP_LOGS:-}" == 1 || $1 != 0 ]] || rm -rf "$work"
  exit "$1"
}
alive() { kill -0 "$pid" 2>/dev/null; }
deadline=$((SECONDS + limit))

id="" session=""
while ((SECONDS < deadline)) && [[ -z $id ]]; do
  row="$("$R" list 2>/dev/null | jq -c --arg t "$tag" 'first(.[] | select(.title == $t))' 2>/dev/null)"
  id="$(jq -r '.id // empty' <<<"$row" 2>/dev/null)"; session="$(jq -r '.session // empty' <<<"$row" 2>/dev/null)"
  [[ -n $id ]] || { alive || finish 1 "FAIL $agent: exited before register_artifact"; sleep 2; }
done
[[ -n $id ]] || finish 1 "FAIL $agent: never called register_artifact"

# Wait until the agent is really listening (status lists its long-poll), so the comment isn't early.
listening=""
for _ in $(seq 60); do
  if "$R" status | jq -e --arg a "artifact:$id" --arg s "session:$session" '.waiting | has($a) or has($s)' >/dev/null 2>&1; then
    listening=1; break
  fi
  alive || finish 1 "FAIL $agent: exited before wait_for_comments"
  sleep 1
done
[[ -n $listening ]] || echo "note $agent: no wait_for_comments seen after 60s; submitting anyway"

port="$("$R" status | jq -r .port)"
tok="$("$R" snippet "$id" | grep -o 'data-redline-token="[^"]*"' | cut -d'"' -f2)"
curl -fsS -X POST "http://127.0.0.1:$port/api/artifacts/$id/submit" -H "Authorization: Bearer $tok" \
  -H 'Content-Type: application/json' \
  -d '{"threads":[{"anchor":{"kind":"text","quote":"hello"},"body":"ping from harness"}]}' >/dev/null \
  || finish 1 "FAIL $agent: submit rejected"

while ((SECONDS < deadline)); do
  got="$("$R" list "$id" 2>/dev/null | jq -r 'first(.[0].messages[] | select(.author == "agent") | .body) // empty')"
  [[ -n $got ]] && finish 0 "PASS $agent: $got"
  if ! alive; then # the reply can land just as the agent exits: look once more
    sleep 3
    got="$("$R" list "$id" 2>/dev/null | jq -r 'first(.[0].messages[] | select(.author == "agent") | .body) // empty')"
    [[ -n $got ]] && finish 0 "PASS $agent: $got"
    finish 1 "FAIL $agent: exited without replying"
  fi
  sleep 2
done
finish 1 "FAIL $agent: no reply within ${limit}s"
