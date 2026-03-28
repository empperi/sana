#!/usr/bin/env sh
# Runs cargo check after any .rs file edit and surfaces errors back to Claude.
# For Mac/Linux developers. Requires python3 (pre-installed on modern macOS and most Linux distros).

input=$(cat)

file_path=$(printf '%s' "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('tool_input',{}).get('file_path',''))")
cwd=$(printf '%s' "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('cwd',''))")

# Only act on .rs files
case "$file_path" in
  *.rs) ;;
  *) exit 0 ;;
esac

# Detect frontend vs backend workspace
case "$file_path" in
  */frontend/*) cargo_dir="$cwd/frontend"; label="frontend" ;;
  *) cargo_dir="$cwd"; label="backend" ;;
esac

printf 'cargo check (%s)...\n' "$label"

output=$(cd "$cargo_dir" && cargo check 2>&1)
exit_code=$?

if [ $exit_code -eq 0 ]; then
  printf 'OK\n'
  exit 0
else
  printf '%s\n' "$output"
  exit 2
fi
