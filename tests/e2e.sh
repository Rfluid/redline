#!/usr/bin/env bash
# End-to-end: daemon + CLI (agent side) + curl (artifact side) + MCP stdio. Temp state, random port.
# Only cross-process behaviour lives here; HTTP semantics (auth matrix, submit rules) are unit-tested in src/daemon/tests.rs.
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
R="$repo/bin/redline"
# What agents' MCP configs run: bin/redline, or on Windows (Git Bash) the exe bin/redline.ps1 keeps current.
E="$R" EXE=""
case "$(uname -s)" in MINGW* | MSYS* | CYGWIN*)
  win=1 E="$repo/target/bin/redline.exe" EXE=".exe"
  jq() { command jq -b "$@"; } # native jq writes CRLF otherwise
  ;;
esac
# Daemon pids come from daemon.json: Windows pids on Windows, which bash's kill doesn't know.
alive() { if [[ -n "${win:-}" ]]; then tasklist //FI "PID eq $1" //NH | grep -qw "$1"; else kill -0 "$1" 2>/dev/null; fi; }
kill_pid() { if [[ -n "${win:-}" ]]; then taskkill //F //PID "$1" >/dev/null 2>&1; else kill "$1" 2>/dev/null; fi; }
export REDLINE_HOME="$(mktemp -d)" REDLINE_PORT="$((20000 + RANDOM % 20000))" RUST_LOG="redline=debug,tower_http=debug"
# Own config.toml; no background release check (the update steps use a local fake remote).
export REDLINE_CONFIG="$REDLINE_HOME/config.toml" REDLINE_NO_UPDATE_CHECK=1
trap 'kill_pid "$(jq -r .pid "$REDLINE_HOME/daemon.json" 2>/dev/null)" || true; sleep 0.5; rm -rf "$REDLINE_HOME" 2>/dev/null || true' EXIT
ok() { echo "ok - $*"; }
fail() { echo "FAIL - $*" >&2; exit 1; }

reg="$("$R" register "Demo page" --session ses_test)"
art="$(jq -r .artifact_id <<<"$reg")"
tok="$(jq -r .embed <<<"$reg" | grep -o 'data-redline-token="[^"]*"' | cut -d'"' -f2)"
base="http://127.0.0.1:$REDLINE_PORT"
[[ -n "$art" && -n "$tok" ]] || fail "register"
ok "register $art (daemon auto-started)"

[[ "$(curl -fsS "$base/client.js")" == *window.Redline* ]] || fail "client.js served"
ok "client.js served"

# Agent waits in the background; the user submits two comments in one batch.
"$R" wait --session ses_test --timeout 20 > "$REDLINE_HOME/wait.json" &
waiter=$!
sleep 0.5
curl -fsS "$base/api/artifacts/$art/submit" -H "authorization: Bearer $tok" -H 'content-type: application/json' \
  -d '{"threads":[{"anchor":{"kind":"text","quote":"Hello"},"body":"make this bigger"},{"anchor":{"kind":"element","selector":"#nav"},"body":"remove"}]}' >/dev/null
wait $waiter
n="$(jq '.events | length' "$REDLINE_HOME/wait.json")"
[[ "$n" == 2 ]] || fail "wait got $n events"
ok "batch submit delivered to waiting agent"

thr="$(jq -r '.events[0].thread.id' "$REDLINE_HOME/wait.json")"
next="$(jq -r .next "$REDLINE_HOME/wait.json")"
"$R" status "$thr" working >/dev/null
"$R" reply "$thr" "Bumped to 2rem." >/dev/null
st="$(curl -fsS "$base/api/artifacts/$art/threads" -H "authorization: Bearer $tok" | jq -r ".[] | select(.id==\"$thr\") | \"\(.status) \(.messages[-1].author)\"")"
[[ "$st" == "answered agent" ]] || fail "reply state: $st"
ok "agent reply visible to artifact"

out="$("$R" wait --session ses_test --since "$next" --timeout 1)"
[[ "$(jq '.events | length' <<<"$out")" == 0 ]] || fail "agent's own events should not wake the agent"
ok "agent events filtered from agent wait"

mcp_out="$(printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"e2e","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"list_threads\",\"arguments\":{\"artifact_id\":\"$art\"}}}" \
  | "$E" mcp)"
[[ "$(jq -s 'length' <<<"$mcp_out")" == 3 ]] || fail "mcp responses: $mcp_out"
jq -se '.[1].result.tools | map(.name) | index("wait_for_comments") != null' <<<"$mcp_out" >/dev/null || fail "tools/list"
jq -se '.[2].result.content[0].text | fromjson | length == 2' <<<"$mcp_out" >/dev/null || fail "tools/call list_threads"
ok "mcp initialize, tools/list, tools/call"

# Concurrency: two MCP sessions on one daemon, each waiting on its own artifact.
mcp_start() { # name fd: an MCP server reading what we write to fd
  eval "exec $2> >(\"\$E\" mcp >\"\$REDLINE_HOME/$1.out\" 2>/dev/null)"
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"e2e","version":"0"}}}' \
    '{"jsonrpc":"2.0","method":"notifications/initialized"}' >&"$2"
}
mcp_call() { # fd id tool args-json
  printf '{"jsonrpc":"2.0","id":%s,"method":"tools/call","params":{"name":"%s","arguments":%s}}\n' "$2" "$3" "$4" >&"$1"
}
mcp_result() { # name id [tries]: the tool result's JSON text, once it arrives
  for _ in $(seq "${3:-100}"); do
    r="$(jq -cR "fromjson? | select(.id==$2) | .result.content[0].text | fromjson" "$REDLINE_HOME/$1.out")"
    [[ -n "$r" ]] && { printf '%s' "$r"; return 0; }
    sleep 0.1
  done
  return 1
}
mcp_start a 7
mcp_start b 8
mcp_call 7 2 register_artifact '{"title":"A"}'
mcp_call 8 2 register_artifact '{"title":"B"}'
art_a="$(mcp_result a 2 | jq -r .artifact_id)"; art_b="$(mcp_result b 2 | jq -r .artifact_id)"
tok_a="$(mcp_result a 2 | jq -r .embed | grep -o 'data-redline-token="[^"]*"' | cut -d'"' -f2)"
tok_b="$(mcp_result b 2 | jq -r .embed | grep -o 'data-redline-token="[^"]*"' | cut -d'"' -f2)"
[[ "$art_a" == art_* && "$art_b" == art_* && "$art_a" != "$art_b" ]] || fail "concurrent register: $art_a $art_b"
mcp_call 7 3 wait_for_comments '{"timeout_s":20}'
mcp_call 8 3 wait_for_comments '{"timeout_s":20}'
sleep 0.5
submit() { curl -fsS "$base/api/artifacts/$1/submit" -H "authorization: Bearer $2" -H 'content-type: application/json' \
  -d "{\"threads\":[{\"anchor\":{},\"body\":\"$3\"}]}" >/dev/null; }
submit "$art_a" "$tok_a" "for A"
got_a="$(mcp_result a 3)" || fail "session A never woke"
[[ "$(jq -r '[.events[].artifact_id] | unique | join(",")' <<<"$got_a")" == "$art_a" ]] || fail "A got: $got_a"
! mcp_result b 3 10 >/dev/null || fail "session B woke on A's comment"
submit "$art_b" "$tok_b" "for B"
got_b="$(mcp_result b 3)" || fail "session B never woke"
[[ "$(jq -r '[.events[] | .thread.messages[0].body] | join(",")' <<<"$got_b")" == "for B" ]] || fail "B got: $got_b"
mcp_call 7 4 wait_for_comments '{"timeout_s":1}'
mcp_call 8 4 wait_for_comments '{"timeout_s":1}'
[[ "$(mcp_result a 4 | jq -r .timed_out)" == true ]] || fail "A's cursor re-delivered or saw B"
[[ "$(mcp_result b 4 | jq -r .timed_out)" == true ]] || fail "B's cursor re-delivered or saw A"
exec 7>&- 8>&-
ok "two MCP sessions: no cross-delivery, independent cursors"

# Channel push: `mcp --channel` advertises claude/channel and pushes new user comments.
exec 9> >(REDLINE_SESSION=ses_chan "$E" mcp --channel >"$REDLINE_HOME/c.out" 2>/dev/null)
cpid=$!
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"e2e","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' >&9
reg_c="$("$R" register "chan" --session ses_chan)"
art_c="$(jq -r .artifact_id <<<"$reg_c")"; tok_c="$(jq -r .embed <<<"$reg_c" | grep -o 'data-redline-token="[^"]*"' | cut -d'"' -f2)"
sleep 1
submit "$art_c" "$tok_c" "pushed"
note=""
for _ in $(seq 100); do
  note="$(jq -cR 'fromjson? | select(.method=="notifications/claude/channel") | .params' "$REDLINE_HOME/c.out")"
  [[ -n "$note" ]] && break; sleep 0.1
done
jq -e '.content == "New redline comment: pushed" and (.meta | (.artifact_id | startswith("art_")) and (.seq | type == "string"))' <<<"$note" >/dev/null \
  || fail "channel notification: $note"
jq -seR '[splits("\n") | fromjson? | select(.id==1)][0].result.capabilities.experimental["claude/channel"] == {}' "$REDLINE_HOME/c.out" >/dev/null \
  || fail "claude/channel capability missing"
kill -0 "$cpid" 2>/dev/null || fail "mcp --channel not running before its client hung up"
exec 9>&-
for _ in $(seq 40); do kill -0 "$cpid" 2>/dev/null || break; sleep 0.25; done
! kill -0 "$cpid" 2>/dev/null || fail "mcp --channel kept running after its client hung up"
ok "mcp --channel pushes notifications/claude/channel, and exits with its client"

# Region screenshots: stored with the thread, fetched by the agent via CLI and MCP.
png="iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="
att="$(curl -fsS "$base/api/artifacts/$art/submit" -H "authorization: Bearer $tok" -H 'content-type: application/json' \
  -d "{\"threads\":[{\"anchor\":{\"kind\":\"region\",\"x\":0.5,\"y\":0.5},\"body\":\"here\",\"image\":{\"mime\":\"image/png\",\"data\":\"$png\"}}]}" \
  | jq -r '.[0].thread.anchor.image')"
[[ "$att" == att_* ]] || fail "submit with image: $att"
"$R" image "$att" --out "$REDLINE_HOME/shot.png" | jq -e '.mime == "image/png"' >/dev/null || fail "redline image"
[[ "$(base64 -w0 <"$REDLINE_HOME/shot.png" 2>/dev/null || base64 <"$REDLINE_HOME/shot.png")" == "$png" ]] || fail "image bytes differ"
mcp_img="$(printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"e2e","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"get_image\",\"arguments\":{\"attachment_id\":\"$att\"}}}" \
  | "$E" mcp | jq -c 'select(.id==2) | .result.content[0]')"
jq -e --arg png "$png" '.type == "image" and .mimeType == "image/png" and .data == $png' <<<"$mcp_img" >/dev/null || fail "mcp get_image: $mcp_img"
ok "region screenshot: stored, redline image, mcp get_image"

# Snippet hygiene: strip removes the embed; --check fails while one is left.
page="$REDLINE_HOME/site/index.html"; mkdir -p "$REDLINE_HOME/site"
printf '<html><body>\n<p>Hello</p>\n%s\n</body></html>\n' "$(jq -r .embed <<<"$reg")" >"$page"
if "$R" strip --check "$REDLINE_HOME/site" >/dev/null; then fail "strip --check passed with a snippet in place"; fi
"$R" strip "$REDLINE_HOME/site" | jq -e '.stripped[0].removed == 1' >/dev/null || fail "strip"
! grep -q "redline" "$page" || fail "snippet left: $(cat "$page")"
[[ "$(cat "$page")" == $'<html><body>\n<p>Hello</p>\n</body></html>' ]] || fail "strip changed more than the snippet"
"$R" strip --check "$REDLINE_HOME/site" >/dev/null || fail "strip --check after strip"
ok "strip removes the embed; --check gates on it"

# Lifecycle: status → stop → auto-restart.
st="$("$R" status)"
[[ "$(jq -r .running <<<"$st")" == true && "$(jq -r .port <<<"$st")" == "$REDLINE_PORT" ]] || fail "status: $st"
[[ "$(jq -r .artifacts <<<"$st")" == "$("$R" list | jq length)" && "$(jq -r .by_status.answered <<<"$st")" == 1 ]] || fail "status counts: $st"
pid1="$(jq -r .pid <<<"$st")"
"$R" stop | jq -e ".stopped == $pid1" >/dev/null || fail "stop"
[[ ! -e "$REDLINE_HOME/daemon.json" ]] || fail "daemon.json left behind after stop"
"$R" status | jq -e '.running == false' >/dev/null || fail "status after stop"
"$R" stop | jq -e '.stopped == null' >/dev/null || fail "second stop"
"$R" list >/dev/null
pid2="$(jq -r .pid "$REDLINE_HOME/daemon.json")"
[[ "$pid2" != "$pid1" ]] || fail "no auto-restart"
ok "status, stop, auto-restart"

# A wait rides through a daemon restart (an update replaces the daemon mid-wait).
"$R" wait --artifact "$art" --since "$("$R" status | jq .head)" --timeout 30 >"$REDLINE_HOME/rw.json" &
rw=$!
sleep 1
"$R" stop >/dev/null
sleep 2 # the waiter reconnects, which starts a fresh daemon
submit "$art" "$tok" "after restart"
wait $rw || fail "wait failed across the restart"
[[ "$(jq -r '.events[0].thread.messages[0].body' "$REDLINE_HOME/rw.json")" == "after restart" ]] || fail "wait across restart: $(cat "$REDLINE_HOME/rw.json")"
ok "a wait rides through a daemon restart"

# Version handshake: a binary rebuilt after the daemon started replaces it on the next call.
sleep 1.1
touch "$repo/target/release/redline$EXE"
"$R" list "$art" | jq -e 'length == 4' >/dev/null || fail "threads survive restart"
pid3="$(jq -r .pid "$REDLINE_HOME/daemon.json")"
[[ "$pid3" != "$pid2" ]] || fail "stale daemon not replaced after rebuild"
"$R" list >/dev/null
[[ "$(jq -r .pid "$REDLINE_HOME/daemon.json")" == "$pid3" ]] || fail "fresh daemon restarted again"
ok "rebuilt binary replaces the stale daemon"

# A daemon whose state dir is deleted stops by itself (no orphans after cleanup).
orphan_home="$(mktemp -d)"
REDLINE_HOME="$orphan_home" REDLINE_PORT="$((REDLINE_PORT + 1))" "$R" list >/dev/null
opid="$(jq -r .pid "$orphan_home/daemon.json")"
rm -rf "$orphan_home" 2>/dev/null || true # Windows: the open database stays until the daemon exits
for _ in $(seq 30); do alive "$opid" || break; sleep 1; done
! alive "$opid" || { kill_pid "$opid"; fail "daemon outlived its state dir"; }
rm -rf "$orphan_home"
ok "daemon stops when its state dir is deleted"

# Updates: the release check, notices for people and agents, dismissing them, and `redline update`.
# A local "remote" stands in for GitHub: releases v0.0.1 and v99.0.0, plus a prerelease that must be ignored.
gitq() { git -c user.name=e2e -c user.email=e2e@example.com -c init.defaultBranch=main "$@" >/dev/null 2>&1; }
up="$REDLINE_HOME/up"
mkdir -p "$up/src/bin" "$up/src/scripts"
gitq init --bare "$up/remote.git"
gitq -C "$up/src" init
stub() { # version: bin/redline and its Windows twin print it
  printf '#!/usr/bin/env bash\necho "redline %s (stub)"\n' "$1" >"$up/src/bin/redline"; chmod +x "$up/src/bin/redline"
  printf 'Write-Output "redline %s (stub)"\n' "$1" >"$up/src/bin/redline.ps1"
}
stub 0.0.1
cp "$repo/scripts/checkout-release.sh" "$repo/scripts/checkout-release.ps1" "$up/src/scripts/"
gitq -C "$up/src" add -A; gitq -C "$up/src" commit -m one; gitq -C "$up/src" tag v0.0.1
stub 99.0.0
gitq -C "$up/src" commit -am two; gitq -C "$up/src" tag v99.0.0
gitq -C "$up/src" commit --allow-empty -m three; gitq -C "$up/src" tag v100.0.0-rc.1
gitq -C "$up/src" push --tags "$up/remote.git" HEAD:main

u="$(REDLINE_UPDATE_REMOTE="$up/remote.git" "$R" update check)"
[[ "$(jq -r '"\(.latest) \(.available) \(.notify)"' <<<"$u")" == "99.0.0 true true" ]] || fail "update check: $u"
[[ "$(jq -r .update.latest <<<"$("$R" register "Update notice")")" == 99.0.0 ]] || fail "register should carry update"
mcp_reg="$(printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"e2e","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"register_artifact","arguments":{"title":"U"}}}' | "$E" mcp)"
jq -se '.[1].result.content[0].text | fromjson | .update.latest == "99.0.0" and (.update_hint | test("redline update"))' <<<"$mcp_reg" >/dev/null \
  || fail "register_artifact should carry update: $mcp_reg"
[[ "$(curl -fsS "$base/api/artifacts/$art/update" -H "authorization: Bearer $tok" | jq -r .notify)" == true ]] || fail "overlay update route"
ok "release check finds v99.0.0 (prerelease ignored); register, register_artifact and the overlay route report it"

[[ "$("$R" update skip | jq -r '"\(.skipped) \(.notify)"')" == "true false" ]] || fail "update skip"
grep -q 'skip = \["99.0.0"\]' "$REDLINE_CONFIG" || fail "skip not in config.toml"
[[ "$(jq 'has("update")' <<<"$("$R" register "Skipped")")" == false ]] || fail "skipped release still reported"
[[ "$("$R" update off | jq -r .check)" == false ]] && grep -q '^check = false' "$REDLINE_CONFIG" || fail "update off"
[[ "$("$R" update on | jq -r .check)" == true ]] || fail "update on"
! "$R" update skip 1.2 >/dev/null 2>&1 || fail "update skip accepted a non-release version"
ok "update skip / off / on write config.toml; a skipped release is no longer reported"

# `redline update` on a clone at v0.0.1: checkout-release.sh moves it to v99.0.0, then bin/redline runs.
exe="$repo/target/release/redline$EXE"
gitq clone "$up/remote.git" "$up/clone"; gitq -C "$up/clone" checkout --detach v0.0.1
out="$(REDLINE_REPO="$up/clone" "$exe" update 2>/dev/null)" || fail "redline update"
[[ "$(jq -r '"\(.updated) \(.to) \(.version)"' <<<"$out")" == "true 99.0.0 redline 99.0.0 (stub)" ]] || fail "update result: $out"
[[ "$(git -C "$up/clone" describe --tags)" == v99.0.0 ]] || fail "clone not on v99.0.0"
[[ "$(REDLINE_REPO="$up/clone" "$exe" update 2>/dev/null | jq -r .updated)" == false ]] || fail "second update should be a no-op"
# Clones from before the script existed: the same steps run inline. Local edits stop it.
gitq clone "$up/remote.git" "$up/old"; gitq -C "$up/old" checkout --detach v0.0.1
gitq -C "$up/old" rm -q scripts/checkout-release.sh scripts/checkout-release.ps1; gitq -C "$up/old" commit -m "no script"
echo edit >>"$up/old/bin/redline"
! REDLINE_REPO="$up/old" "$exe" update >/dev/null 2>&1 || fail "update ran over local edits"
gitq -C "$up/old" checkout -- bin/redline
REDLINE_REPO="$up/old" "$exe" update >/dev/null 2>&1 || fail "update without checkout-release.sh"
[[ "$(git -C "$up/old" describe --tags)" == v99.0.0 ]] || fail "fallback update not on v99.0.0"
ok "redline update moves the clone to the latest release (script or inline), and refuses over local edits"

# Logging: requests are logged, tokens never are.
log="$REDLINE_HOME/daemon.log"
grep -q 'path=/api/artifacts' "$log" || fail "requests not logged"
grep -q 'listening on 127.0.0.1' "$log" || fail "startup not logged"
for secret in "$(cat "$REDLINE_HOME/admin.token")" "$tok"; do
  ! grep -qF "$secret" "$log" || fail "a token leaked into daemon.log"
done
ok "daemon.log has requests and no tokens"

echo "all passed"
