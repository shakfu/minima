# Shared by scripts/test_<provider>.sh. Those set PROVIDER, KEY_ENV and DEFAULT_MODEL, then
# source this file. The endpoint and the wire format come from minima's own provider registry, so
# a script names a provider and nothing else.
#
# Runs from a throwaway sandbox, never the repo. minima's file tools have no path jail because its
# bash tool is unrestricted, and a live model decides for itself what to run.

set -u

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
BIN=${BIN:-$ROOT/target/debug/minima}
MODEL=${MODEL:-$DEFAULT_MODEL}
# Only for pointing a provider at a local server or a fixture; it never changes the dialect.
BASE_URL=${BASE_URL:-}
KEY_FILE=${KEY_FILE:-$HOME/.config/minima/$PROVIDER.key}

PASS=0
FAIL=0

die() { echo "error: $*" >&2; exit 1; }

[ -x "$BIN" ] || die "no binary at $BIN -- run 'make build' first"

# Environment first, then the key file. The key is never echoed, and goes to minima through the
# environment rather than a flag so it stays out of the process table.
resolve_key() {
    local from_env="${!KEY_ENV:-}"
    if [ -n "$from_env" ]; then
        MINIMA_API_KEY="$from_env"
        KEY_SOURCE="\$$KEY_ENV"
    elif [ -r "$KEY_FILE" ]; then
        MINIMA_API_KEY=$(tr -d '\n\r' < "$KEY_FILE")
        KEY_SOURCE="$KEY_FILE"
    else
        die "no key: set \$$KEY_ENV, or write one to $KEY_FILE (chmod 600)"
    fi
    [ -n "$MINIMA_API_KEY" ] || die "the key from $KEY_SOURCE is empty"
    export MINIMA_API_KEY
}

setup_sandbox() {
    SANDBOX=$(mktemp -d "${TMPDIR:-/tmp}/minima-live-XXXXXX") || die "could not make a sandbox"
    trap 'rm -rf "$SANDBOX"' EXIT
    printf 'alpha: the first line\nbeta: the second line\ngamma: the third line\n' \
        > "$SANDBOX/notes.txt"
    printf 'one\n' > "$SANDBOX/a.txt"
    printf 'two\n' > "$SANDBOX/b.txt"
    cd "$SANDBOX" || die "could not enter the sandbox"
}

# scenario <name> <prompt> <extended-regex the combined output must match>
scenario() {
    local name=$1 prompt=$2 expect=$3 out status
    echo "=============================================================="
    echo "## $name"
    echo "--------------------------------------------------------------"
    out=$("$BIN" -p "$prompt" 2>&1)
    status=$?
    echo "$out"
    if [ $status -ne 0 ]; then
        echo ">> FAIL (exit $status)"
        FAIL=$((FAIL + 1))
    elif echo "$out" | grep -qiE "$expect"; then
        echo ">> pass"
        PASS=$((PASS + 1))
    else
        echo ">> FAIL (no match for /$expect/)"
        FAIL=$((FAIL + 1))
    fi
    echo
}

run_suite() {
    resolve_key
    setup_sandbox

    export MINIMA_PROVIDER="$PROVIDER"
    export MINIMA_MODEL="$MODEL"
    [ -n "$BASE_URL" ] && export MINIMA_BASE_URL="$BASE_URL"

    echo "provider : $PROVIDER"
    echo "endpoint : ${BASE_URL:-(from the registry)}"
    echo "model    : $MODEL"
    echo "key from : $KEY_SOURCE"
    echo "sandbox  : $SANDBOX"
    echo

    scenario "text only, no tools" \
        "Reply with exactly the word: pong" \
        'pong'

    scenario "read tool" \
        "Read the file notes.txt and tell me what the second line says." \
        'beta|second line'

    # Before the write scenario, so the count is a known 3.
    scenario "bash tool" \
        "Use bash to count how many .txt files are in the current directory. Report the number." \
        '(^|[^0-9])3([^0-9]|$)'

    scenario "write, then read back" \
        "Write a file called greeting.txt containing the single word hello, then read it back and tell me what it contains." \
        'hello'

    echo "=============================================================="
    if [ -f "$SANDBOX/greeting.txt" ]; then
        echo "greeting.txt on disk: $(cat "$SANDBOX/greeting.txt")"
    else
        echo "greeting.txt was never written"
        FAIL=$((FAIL + 1))
    fi
    echo "$PASS passed, $FAIL failed"
    [ "$FAIL" -eq 0 ]
}
