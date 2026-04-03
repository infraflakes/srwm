#!/usr/bin/env bash
#
# Torture test for driftwm — hammers the compositor to surface resource leaks.
# Run this INSIDE a driftwm session. Monitors fd count and memory throughout.
#
# Usage:
#   ./extras/scripts/torture-test.sh [--rounds N] [--terminal TERM]
#
# Requirements: foot (or another terminal), grim, wl-copy, wlr-randr (optional)

set -euo pipefail

ROUNDS=50
SOAK=1
TERMINAL="${TERMINAL:-alacritty}"
SLEEP_SHORT=0.3
SLEEP_LONG=1
PID=""

# --- Argument parsing ---
while [[ $# -gt 0 ]]; do
    case "$1" in
        --rounds)  ROUNDS="$2"; shift 2 ;;
        --soak)    SOAK="$2"; shift 2 ;;
        --terminal) TERMINAL="$2"; shift 2 ;;
        *) echo "Unknown arg: $1"; exit 1 ;;
    esac
done

# --- Helpers ---
compositor_pid() {
    if [[ -z "$PID" ]]; then
        PID=$(pidof driftwm 2>/dev/null || true)
        if [[ -z "$PID" ]]; then
            echo "ERROR: driftwm not running" >&2
            exit 1
        fi
    fi
    echo "$PID"
}

fd_count() {
    ls "/proc/$(compositor_pid)/fd" 2>/dev/null | wc -l
}

mem_rss_kb() {
    awk '/^VmRSS:/ { print $2 }' "/proc/$(compositor_pid)/status" 2>/dev/null || echo "?"
}

snapshot() {
    local label="$1"
    local fds mem
    fds=$(fd_count)
    mem=$(mem_rss_kb)
    printf "  %-30s  fds=%-6s  rss=%-10s KB\n" "$label" "$fds" "$mem"
}

separator() {
    echo ""
    echo "━━━ $1 ━━━"
}

wait_settle() {
    sleep "$SLEEP_LONG"
}

# --- Checks ---
echo "driftwm torture test"
echo "  compositor pid: $(compositor_pid)"
echo "  terminal:       $TERMINAL"
echo "  rounds:         $ROUNDS"
echo "  soak iterations: $SOAK"
echo ""

snapshot "BASELINE (before tests)"
BASELINE_FDS=$(fd_count)
BASELINE_MEM=$(mem_rss_kb)

# Track per-soak stats for trend detection
declare -a SOAK_FDS
declare -a SOAK_MEM

run_tests() {
    local prefix="$1"

    # ============================================================
    # Test 1: Rapid window open/close
    # ============================================================
    separator "${prefix}Test 1: Open/close $ROUNDS windows"

    for i in $(seq 1 "$ROUNDS"); do
        $TERMINAL -e sh -c "exit 0" &
    done
    wait
    wait_settle
    snapshot "after $ROUNDS open/close cycles"

    # ============================================================
    # Test 2: Overlapping windows (concurrent)
    # ============================================================
    separator "${prefix}Test 2: $ROUNDS concurrent windows"

    pids=()
    for i in $(seq 1 "$ROUNDS"); do
        $TERMINAL -e sh -c "sleep 2" &
        pids+=($!)
    done
    sleep 1
    snapshot "with $ROUNDS windows open"

    for p in "${pids[@]}"; do
        wait "$p" 2>/dev/null || true
    done
    wait_settle
    snapshot "after all closed"

    # ============================================================
    # Test 3: Screenshot stress (screencopy protocol)
    # ============================================================
    if command -v grim &>/dev/null; then
        separator "${prefix}Test 3: $ROUNDS screenshots (screencopy)"

        for i in $(seq 1 "$ROUNDS"); do
            grim - > /dev/null 2>&1 || true
        done
        wait_settle
        snapshot "after $ROUNDS screenshots"
    else
        separator "${prefix}Test 3: SKIPPED (grim not found)"
    fi

    # ============================================================
    # Test 4: Clipboard cycling
    # ============================================================
    if command -v wl-copy &>/dev/null && command -v wl-paste &>/dev/null; then
        separator "${prefix}Test 4: $ROUNDS clipboard copies"

        for i in $(seq 1 "$ROUNDS"); do
            echo "torture-test-payload-$i" | wl-copy 2>/dev/null || true
        done
        last=$(wl-paste 2>/dev/null || echo "FAIL")
        wait_settle
        snapshot "after $ROUNDS clipboard copies (last=$last)"
    else
        separator "${prefix}Test 4: SKIPPED (wl-copy/wl-paste not found)"
    fi

    # ============================================================
    # Test 5: Window open + screenshot interleaved
    # ============================================================
    separator "${prefix}Test 5: Interleaved window + screenshot ($(( ROUNDS / 2 )) rounds)"

    half=$(( ROUNDS / 2 ))
    for i in $(seq 1 "$half"); do
        $TERMINAL -e sh -c "sleep 0.5" &
        if command -v grim &>/dev/null; then
            grim - > /dev/null 2>&1 || true
        fi
    done
    wait
    wait_settle
    snapshot "after interleaved test"

    # ============================================================
    # Test 6: Layer shell stress (if fuzzel/bemenu available)
    # ============================================================
    if command -v fuzzel &>/dev/null; then
        separator "${prefix}Test 6: Layer shell open/close (10 rounds)"

        for i in $(seq 1 10); do
            fuzzel &>/dev/null &
            fz_pid=$!
            sleep "$SLEEP_SHORT"
            kill "$fz_pid" 2>/dev/null || true
            wait "$fz_pid" 2>/dev/null || true
        done
        wait_settle
        snapshot "after 10 layer-shell open/close"
    else
        separator "${prefix}Test 6: SKIPPED (fuzzel not found)"
    fi

    # ============================================================
    # Test 7: Rapid focus cycling (open N windows, alt-tab between)
    # ============================================================
    separator "${prefix}Test 7: Rapid focus cycling"

    focus_pids=()
    for i in $(seq 1 10); do
        $TERMINAL -e sh -c "sleep 5" &
        focus_pids+=($!)
    done
    sleep 1
    # Hammer the compositor with rapid window activations via foreign-toplevel
    # (simulated by opening/closing more windows while 10 are alive)
    for i in $(seq 1 20); do
        $TERMINAL -e sh -c "exit 0" &
    done
    wait
    for p in "${focus_pids[@]}"; do
        wait "$p" 2>/dev/null || true
    done
    wait_settle
    snapshot "after focus cycling"

    # ============================================================
    # Test 8: Screenshot while windows open (combined protocol stress)
    # ============================================================
    if command -v grim &>/dev/null; then
        separator "${prefix}Test 8: Screenshots with live windows ($ROUNDS rounds)"

        bg_pids=()
        for i in $(seq 1 10); do
            $TERMINAL -e sh -c "sleep 10" &
            bg_pids+=($!)
        done
        sleep 1

        for i in $(seq 1 "$ROUNDS"); do
            grim - > /dev/null 2>&1 || true
        done

        for p in "${bg_pids[@]}"; do
            kill "$p" 2>/dev/null || true
            wait "$p" 2>/dev/null || true
        done
        wait_settle
        snapshot "after screenshots with live windows"
    else
        separator "${prefix}Test 8: SKIPPED (grim not found)"
    fi

    # ============================================================
    # Test 9: Output reconfiguration (if wlr-randr available)
    # ============================================================
    if command -v wlr-randr &>/dev/null; then
        separator "${prefix}Test 9: Output toggle (5 rounds)"

        output_name=$(wlr-randr --json 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin)[0]['name'])" 2>/dev/null || true)

        if [[ -n "$output_name" ]] && [[ $(wlr-randr --json 2>/dev/null | python3 -c "import sys,json; print(len(json.load(sys.stdin)))" 2>/dev/null) -gt 1 ]]; then
            for i in $(seq 1 5); do
                wlr-randr --output "$output_name" --off 2>/dev/null || true
                sleep "$SLEEP_SHORT"
                wlr-randr --output "$output_name" --on 2>/dev/null || true
                sleep "$SLEEP_SHORT"
            done
            wait_settle
            snapshot "after 5 output toggle cycles"
        else
            echo "  (skipped — need 2+ outputs for safe toggle)"
        fi
    else
        separator "${prefix}Test 9: SKIPPED (wlr-randr not found)"
    fi
}

# ============================================================
# Run all tests in a soak loop
# ============================================================
for soak_i in $(seq 1 "$SOAK"); do
    if (( SOAK > 1 )); then
        separator "SOAK ITERATION $soak_i / $SOAK"
        snapshot "start of iteration $soak_i"
    fi

    run_tests "[$soak_i] "

    iter_fds=$(fd_count)
    iter_mem=$(mem_rss_kb)
    SOAK_FDS+=("$iter_fds")
    SOAK_MEM+=("$iter_mem")

    if (( SOAK > 1 )); then
        snapshot "end of iteration $soak_i"
    fi
done

# ============================================================
# Summary
# ============================================================
separator "RESULTS"

FINAL_FDS=$(fd_count)
FINAL_MEM=$(mem_rss_kb)

snapshot "FINAL"
echo ""
echo "  Baseline:  fds=$BASELINE_FDS  rss=${BASELINE_MEM} KB"
echo "  Final:     fds=$FINAL_FDS  rss=${FINAL_MEM} KB"
echo "  Delta:     fds=$(( FINAL_FDS - BASELINE_FDS ))  rss=$(( FINAL_MEM - BASELINE_MEM )) KB"

# Show per-iteration trend if soak > 1
if (( SOAK > 1 )); then
    echo ""
    echo "  Per-iteration trend:"
    for i in $(seq 0 $(( SOAK - 1 ))); do
        printf "    iter %-3d  fds=%-6s  rss=%-10s KB\n" "$(( i + 1 ))" "${SOAK_FDS[$i]}" "${SOAK_MEM[$i]}"
    done

    # Check for monotonic fd growth (leak signal)
    growing=true
    for i in $(seq 1 $(( SOAK - 1 ))); do
        if (( SOAK_FDS[i] <= SOAK_FDS[i-1] )); then
            growing=false
            break
        fi
    done
    if $growing && (( SOAK > 2 )); then
        echo ""
        echo "  ⚠  fd count grew monotonically across all iterations — likely fd leak"
    fi
fi

echo ""

if (( FINAL_FDS > BASELINE_FDS + 5 )); then
    echo "  ⚠  fd count grew by $(( FINAL_FDS - BASELINE_FDS )) — possible fd leak"
    echo "     Inspect with: ls -la /proc/$(compositor_pid)/fd/ | sort"
else
    echo "  ✓  fd count stable"
fi

if (( FINAL_MEM > BASELINE_MEM + 10240 )); then
    echo "  ⚠  RSS grew by $(( FINAL_MEM - BASELINE_MEM )) KB — possible memory leak"
    echo "     Profile with: bytehound or valgrind --tool=dhat"
else
    echo "  ✓  RSS within tolerance"
fi

echo ""
echo "Done. For continuous monitoring: watch -n1 \"ls /proc/$(compositor_pid)/fd | wc -l\""
