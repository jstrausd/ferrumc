#!/bin/bash
# GEN-05.9 runtime A5: extended original helper, normal and Python -O, then guards in both modes.
set -u
C=/home/jstrauss/Schreibtisch/mchosting/ferrumc/.codex-tmp/gen05_9-runtime/clay-oracle
cd "$C/runner"
env -u JAVA_TOOL_OPTIONS -u _JAVA_OPTIONS -u JDK_JAVA_OPTIONS -u JDK_JAVAC_OPTIONS -u CLASSPATH python3    run_oracle.py "$C/oracle-normal"    > "$C/logs/oracle-normal.stdout"    2> "$C/logs/oracle-normal.stderr" &
P1=$!
env -u JAVA_TOOL_OPTIONS -u _JAVA_OPTIONS -u JDK_JAVA_OPTIONS -u JDK_JAVAC_OPTIONS -u CLASSPATH python3 -O run_oracle.py "$C/oracle-optimized" > "$C/logs/oracle-optimized.stdout" 2> "$C/logs/oracle-optimized.stderr" &
P2=$!
wait $P1; E1=$?; echo "oracle-normal exit=$E1" >> "$C/logs/status.txt"
wait $P2; E2=$?; echo "oracle-optimized exit=$E2" >> "$C/logs/status.txt"
env -u JAVA_TOOL_OPTIONS -u _JAVA_OPTIONS -u JDK_JAVA_OPTIONS -u JDK_JAVAC_OPTIONS -u CLASSPATH python3    check_guards.py "$C/guards-normal"    "$C/oracle-normal" > "$C/logs/guards-normal.stdout" 2> "$C/logs/guards-normal.stderr"; echo "guards-normal exit=$?" >> "$C/logs/status.txt"
env -u JAVA_TOOL_OPTIONS -u _JAVA_OPTIONS -u JDK_JAVA_OPTIONS -u JDK_JAVAC_OPTIONS -u CLASSPATH python3 -O check_guards.py "$C/guards-optimized" "$C/oracle-normal" > "$C/logs/guards-optimized.stdout" 2> "$C/logs/guards-optimized.stderr"; echo "guards-optimized exit=$?" >> "$C/logs/status.txt"
echo done >> "$C/logs/status.txt"
