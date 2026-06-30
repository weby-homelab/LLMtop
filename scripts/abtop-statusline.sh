#!/bin/bash
# abtop StatusLine hook for Claude Code
# Reads StatusLine JSON from stdin and writes rate limit data to a file.
#
# Install: add to ~/.claude/settings.json:
#   "statusLine": { "command": "/path/to/abtop-statusline.sh" }
#
# Or run: abtop --setup

OUTPUT_FILE="$HOME/.claude/abtop-rate-limits.json"

# Read JSON from stdin
input=$(cat)

# Extract rate_limits using python/jq/node (whichever is available)
if command -v python3 &>/dev/null; then
    echo "$input" | python3 -c "
import sys, json, time
try:
    data = json.load(sys.stdin)
    rl = data.get('rate_limits', {})
    if not rl:
        sys.exit(0)
    out = {'source': 'claude', 'updated_at': int(time.time())}
    session = rl.get('session', {})
    weekly = rl.get('weekly', {})
    if session:
        out['five_hour'] = {
            'used_percentage': session.get('used_percentage', 0),
            'resets_at': session.get('resets_at', 0)
        }
    if weekly:
        out['seven_day'] = {
            'used_percentage': weekly.get('used_percentage', 0),
            'resets_at': weekly.get('resets_at', 0)
        }
    with open('$OUTPUT_FILE', 'w') as f:
        json.dump(out, f)
except Exception:
    pass
"
elif command -v jq &>/dev/null; then
    five_pct=$(echo "$input" | jq -r '.rate_limits.session.used_percentage // empty' 2>/dev/null)
    if [ -n "$five_pct" ]; then
        five_reset=$(echo "$input" | jq -r '.rate_limits.session.resets_at // 0')
        seven_pct=$(echo "$input" | jq -r '.rate_limits.weekly.used_percentage // 0')
        seven_reset=$(echo "$input" | jq -r '.rate_limits.weekly.resets_at // 0')
        now=$(date +%s)
        cat > "$OUTPUT_FILE" <<EOF
{"source":"claude","updated_at":$now,"five_hour":{"used_percentage":$five_pct,"resets_at":$five_reset},"seven_day":{"used_percentage":$seven_pct,"resets_at":$seven_reset}}
EOF
    fi
fi
