#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
YAML_DIR="$SCRIPT_DIR/resources/lifecycle/context"
LIFECYCLE_CLIENT="/home/lge/Desktop/2track/orchestrator/target/release/lifecycle_client"
COUNTER_ELF="/home/lge/Desktop/2track/orchestrator/src/orchestration/examples/grpc_lifecycle/test_bins/counter_with_state_elf"

SERVICE_RESUME="counter-file-resume-main"
STATE_RESUME="/tmp/counter_file_resume_state.txt"

if [ -z "${HOST_IP:-}" ]; then
  HOST_IP=$(hostname -I | awk '{print $1}')
  echo "Using auto-detected HOST_IP: $HOST_IP"
fi

API_URL="${API_URL:-http://${HOST_IP}:47099/api/artifact}"

echo "========================================================"
echo "Lifecycle Context Resume Test"
echo "========================================================"
echo "Scenario: force-crash x3 -> restart -> context continue"
echo

if ! ss -tln | grep -q ":50051"; then
  echo "❌ ERROR: lifecycle_server is not listening on :50051"
  exit 1
fi

if [ ! -x "$COUNTER_ELF" ]; then
  echo "❌ ERROR: ELF not executable: $COUNTER_ELF"
  exit 1
fi

if [ ! -x "$LIFECYCLE_CLIENT" ]; then
  echo "❌ ERROR: lifecycle_client not found: $LIFECYCLE_CLIENT"
  exit 1
fi

cleanup() {
  echo
  echo "[Cleanup] stop context service"
  "$LIFECYCLE_CLIENT" stop --service "$SERVICE_RESUME" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# Start fresh
rm -f "$STATE_RESUME"

# 1) Launch resume scenario
echo "[1/4] Apply resume-launch YAML"
curl -s -X POST "$API_URL" -H "Content-Type: application/yaml" \
  --data-binary @"$YAML_DIR/counter-file-resume-launch.yaml" | head -1

# 2) 3회 연속 강제 비정상 종료 후 context 재개 검증
echo
echo "[2/4] Verify resume continuity for 3 forced crashes"
PREV_VALUE=0
for ROUND in 1 2 3; do
  echo "  - Round $ROUND: wait for process/state stabilization"
  sleep 2

  if [ ! -f "$STATE_RESUME" ]; then
    echo "❌ FAILED: state file not found before round $ROUND: $STATE_RESUME"
    exit 1
  fi

  BEFORE_VALUE=$(tr -d ' \n\r\t' < "$STATE_RESUME")
  if ! [[ "$BEFORE_VALUE" =~ ^[0-9]+$ ]]; then
    echo "❌ FAILED: invalid state value before round $ROUND: $BEFORE_VALUE"
    exit 1
  fi

  if [ "$BEFORE_VALUE" -lt "$PREV_VALUE" ]; then
    echo "❌ FAILED: state regressed before round $ROUND ($BEFORE_VALUE < $PREV_VALUE)"
    exit 1
  fi

  STATUS_BEFORE=$($LIFECYCLE_CLIENT status --service "$SERVICE_RESUME" 2>&1 || true)
  PID=$(echo "$STATUS_BEFORE" | awk '
    /pid=/ {
      pid=""; state="";
      if (match($0, /pid=[0-9]+/)) {
        pid=substr($0, RSTART+4, RLENGTH-4)
      }
      if (match($0, /state=[^ ]+/)) {
        state=substr($0, RSTART+6, RLENGTH-6)
      }
      if (pid != "" && pid+0 > 0 && state !~ /^PendingRestart/) {
        print pid; exit
      }
    }
  ')
  if [ -z "$PID" ]; then
    echo "❌ FAILED: could not find PID before round $ROUND"
    echo "$STATUS_BEFORE"
    exit 1
  fi

  echo "    state(before)=$BEFORE_VALUE, pid=$PID -> SIGKILL"
  kill -9 "$PID"

  # restartDelaySecs=1 + monitor tick 이후 "새 PID"가 잡힐 때까지 대기
  NEW_PID=""
  for _ in {1..20}; do
    sleep 0.5
    STATUS_AFTER=$($LIFECYCLE_CLIENT status --service "$SERVICE_RESUME" 2>&1 || true)
    CANDIDATE_PID=$(echo "$STATUS_AFTER" | awk '
      /pid=/ {
        pid=""; state="";
        if (match($0, /pid=[0-9]+/)) {
          pid=substr($0, RSTART+4, RLENGTH-4)
        }
        if (match($0, /state=[^ ]+/)) {
          state=substr($0, RSTART+6, RLENGTH-6)
        }
        if (pid != "" && pid+0 > 0 && state !~ /^PendingRestart/) {
          print pid; exit
        }
      }
    ')
    if [ -n "$CANDIDATE_PID" ] && [ "$CANDIDATE_PID" != "$PID" ]; then
      NEW_PID="$CANDIDATE_PID"
      break
    fi
  done

  if [ -z "$NEW_PID" ]; then
    echo "❌ FAILED: restarted PID not detected after round $ROUND"
    exit 1
  fi

  if [ ! -f "$STATE_RESUME" ]; then
    echo "❌ FAILED: state file not found after round $ROUND"
    exit 1
  fi

  # 재시작 직후의 첫 유효 상태값을 최대 5초간 폴링
  AFTER_VALUE=""
  EXPECTED_NEXT=$((BEFORE_VALUE + 1))
  for _ in {1..10}; do
    CURRENT_VALUE=$(tr -d ' \n\r\t' < "$STATE_RESUME")
    if [[ "$CURRENT_VALUE" =~ ^[0-9]+$ ]] && [ "$CURRENT_VALUE" -ge "$EXPECTED_NEXT" ]; then
      AFTER_VALUE="$CURRENT_VALUE"
      break
    fi
    sleep 0.5
  done

  if [ -z "$AFTER_VALUE" ]; then
    echo "❌ FAILED: resumed state not observed after round $ROUND (expected >= $EXPECTED_NEXT)"
    exit 1
  fi

  if ! [[ "$AFTER_VALUE" =~ ^[0-9]+$ ]]; then
    echo "❌ FAILED: invalid state value after round $ROUND: $AFTER_VALUE"
    exit 1
  fi

  if [ "$AFTER_VALUE" -le "$BEFORE_VALUE" ]; then
    echo "❌ FAILED: context did not continue after round $ROUND (before=$BEFORE_VALUE, after=$AFTER_VALUE)"
    exit 1
  fi

  echo "    ✓ resumed: expected_start=$EXPECTED_NEXT, observed_start_or_later=$AFTER_VALUE, new_pid=$NEW_PID"
  PREV_VALUE=$AFTER_VALUE
done

echo "✓ PASSED: context continuity verified across 3 forced crashes"

echo
echo "[3/4] Lifecycle status check"
STATUS_OUT=$($LIFECYCLE_CLIENT status --service "$SERVICE_RESUME" 2>&1 || true)
echo "$STATUS_OUT"
if echo "$STATUS_OUT" | grep -Eq "restarts=[1-9]"; then
  echo "✓ PASSED: restart_count observed in process status"
elif echo "$STATUS_OUT" | grep -Eq "restarted=[1-9]"; then
  echo "✓ PASSED: manager restarted stat observed"
else
  echo "⚠ WARNING: restart counter not visible in this sample output"
fi

echo
echo "[4/4] Final state snapshot"
if [ -f "$STATE_RESUME" ]; then
  FINAL_VALUE=$(tr -d ' \n\r\t' < "$STATE_RESUME")
  echo "state_file=$STATE_RESUME value=$FINAL_VALUE"
else
  echo "state_file missing: $STATE_RESUME"
fi

echo
echo "✅ Context test completed"
echo "   - forced crash x3"
echo "   - restart performed each time"
echo "   - state value continued without regression"
