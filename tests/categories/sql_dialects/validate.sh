#!/bin/bash
set -uo pipefail

# validate.sh — Validate SQL dialect rules against test cases
#
# Usage:
#   ./validate.sh                     # validate all
#   ./validate.sh gaussdb             # validate GaussDB only
#   ./validate.sh gaussdb update_set  # validate specific category

FILTER_DIALECT="${1:-}"
FILTER_CATEGORY="${2:-}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ASTGREP="cargo run --quiet -- analyze"
PASS=0
FAIL=0

TMPFILE=$(mktemp)
find "$SCRIPT_DIR" -name "*.sql" | sort > "$TMPFILE"

while IFS= read -r sql_file; do
    [[ -z "$sql_file" ]] && continue

    if [[ -n "$FILTER_DIALECT" ]] && [[ ! "$sql_file" =~ /$FILTER_DIALECT/ ]]; then
        continue
    fi
    if [[ -n "$FILTER_CATEGORY" ]] && [[ ! "$sql_file" =~ /$FILTER_CATEGORY/ ]]; then
        continue
    fi

    RULE_ID=$(grep -m1 "@rule" "$sql_file" 2>/dev/null | sed 's/.*@rule *//' | tr -d '[:space:]')
    EXPECT=$(grep -m1 "@expect" "$sql_file" 2>/dev/null | sed 's/.*@expect *//' | tr -d '[:space:]')
    DESC=$(grep -m1 "@desc" "$sql_file" 2>/dev/null | sed 's/.*@desc *//')

    if [[ -z "$RULE_ID" ]] || [[ -z "$EXPECT" ]]; then
        continue
    fi

    if [[ "$sql_file" =~ /gaussdb/ ]]; then
        CLI_DIALECT="gaussdb"
    elif [[ "$sql_file" =~ /polardb_mysql/ ]]; then
        CLI_DIALECT="polardb-mysql"
    else
        continue
    fi

    CATEGORY_DIR=$(dirname "$sql_file")
    CASES_DIR=$(dirname "$CATEGORY_DIR")
    DIALECT_ROOT=$(dirname "$CASES_DIR")
    RULES_DIR="$DIALECT_ROOT/rules"

    if [[ ! -d "$RULES_DIR" ]]; then
        continue
    fi

    OUTPUT=$($ASTGREP --dialect "$CLI_DIALECT" --rules "$RULES_DIR/" "$sql_file" 2>/dev/null || true)
    FINDING_COUNT=$(echo "$OUTPUT" | grep "\"rule_id\": \"$RULE_ID\"" | wc -l | tr -d ' ')

    if [[ "$EXPECT" == "MATCH" ]]; then
        if [[ "$FINDING_COUNT" -gt 0 ]]; then
            echo "  PASS  $RULE_ID  $DESC"
            PASS=$((PASS + 1))
        else
            echo "  FAIL  $RULE_ID  $DESC  (expected MATCH, got 0)"
            FAIL=$((FAIL + 1))
        fi
    elif [[ "$EXPECT" == "NO_MATCH" ]]; then
        if [[ "$FINDING_COUNT" -eq 0 ]]; then
            echo "  PASS  $RULE_ID  $DESC"
            PASS=$((PASS + 1))
        else
            echo "  FAIL  $RULE_ID  $DESC  (expected NO_MATCH, got $FINDING_COUNT)"
            FAIL=$((FAIL + 1))
        fi
    fi
done < "$TMPFILE"
rm -f "$TMPFILE"

echo ""
echo "==============================================="
echo " Results: $PASS passed, $FAIL failed"
echo "==============================================="

exit ${FAIL:-0}
