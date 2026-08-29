#!/usr/bin/env sh
# Lint commit messages: single conventional subject line, no body, no trailers.
# Usage:
#   tools/git/commit-lint.sh --file <path>      lint one message file (commit-msg hook)
#   tools/git/commit-lint.sh <rev-range>        lint every commit in a range (CI)
set -eu

TYPES='docs|research|spec|rfc|adr|feat|fix|test|refactor|perf|build|ci|chore'
MAX=72

lint() {
  msg=$(printf '%s\n' "$1" | sed '/^#/d' | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}')
  lines=$(printf '%s\n' "$msg" | sed '/^[[:space:]]*$/d' | wc -l | tr -d ' ')
  subject=$(printf '%s\n' "$msg" | head -n 1)
  fail=0
  if [ "$lines" -ne 1 ]; then
    echo "commit-lint: message must be exactly one line (found $lines): '$subject'"; fail=1
  fi
  if ! printf '%s' "$subject" | grep -Eq "^($TYPES): [a-z0-9].*[^.[:space:]]$"; then
    echo "commit-lint: subject must match '<type>: <lower-case imperative subject>' with type in {$TYPES}: '$subject'"; fail=1
  fi
  if [ "${#subject}" -gt "$MAX" ]; then
    echo "commit-lint: subject exceeds $MAX characters (${#subject}): '$subject'"; fail=1
  fi
  if printf '%s\n' "$msg" | grep -Eiq 'co-authored-by|signed-off-by|generated with|claude|copilot|chatgpt|openai|anthropic|https?://'; then
    echo "commit-lint: trailers, tool attribution and URLs are prohibited: '$subject'"; fail=1
  fi
  return $fail
}

status=0
if [ "${1:-}" = "--file" ]; then
  lint "$(cat "$2")" || status=1
else
  range=${1:-HEAD~1..HEAD}
  for sha in $(git rev-list --no-merges "$range"); do
    lint "$(git log -1 --format=%B "$sha")" || status=1
  done
fi
exit $status
