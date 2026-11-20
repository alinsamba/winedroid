#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
REPO_ROOT="$(pwd)"
PARENT_DIR="$(dirname "$REPO_ROOT")"

usage() {
  printf '%s\n' \
    'scripts/agent_worktrees.sh add [count]     create worktrees (default 3)' \
    'scripts/agent_worktrees.sh list            show worktrees' \
    'scripts/agent_worktrees.sh remove <n>      remove one agent worktree' \
    'scripts/agent_worktrees.sh remove all      remove all agent worktrees' >&2
  exit 64
}

worktree_path() {
  echo "$PARENT_DIR/wd-agent-$1"
}

add_one() {
  local n="$1"
  local path branch
  path="$(worktree_path "$n")"
  branch="agent-$n"

  if [ -d "$path" ]; then
    echo "wd-agent-$n: already exists at $path"
    return 0
  fi

  if git show-ref --verify --quiet "refs/heads/$branch"; then
    git worktree add "$path" "$branch"
  else
    git worktree add "$path" -b "$branch" main
  fi
  echo "wd-agent-$n: created at $path on branch $branch"
}

remove_one() {
  local n="$1"
  local path branch
  path="$(worktree_path "$n")"
  branch="agent-$n"

  if [ ! -d "$path" ]; then
    echo "wd-agent-$n: no worktree at $path"
    return 0
  fi

  if ! git -C "$path" diff --quiet || ! git -C "$path" diff --cached --quiet; then
    echo "wd-agent-$n: has uncommitted changes; refusing to remove" >&2
    echo "  inspect $path, then remove with: git worktree remove --force $path" >&2
    return 1
  fi

  git worktree remove "$path"
  if git branch -d "$branch" 2>/dev/null; then
    echo "wd-agent-$n: removed worktree and merged branch $branch"
  else
    echo "wd-agent-$n: removed worktree; branch $branch kept (not merged into main)"
  fi
}

command="${1:-add}"
case "$command" in
add)
  count="${2:-3}"
  case "$count" in
  '' | *[!0-9]*) usage ;;
  esac
  if ! git diff --quiet HEAD -- AGENTS.md scripts/ 2>/dev/null; then
    echo "note: AGENTS.md or scripts/ have uncommitted changes; new worktrees" >&2
    echo "branch from main and will not see them until they are committed." >&2
  fi
  for n in $(seq 1 "$count"); do
    add_one "$n"
  done
  git worktree list
  ;;
list)
  git worktree list
  ;;
remove)
  target="${2:-}"
  if [ "$target" = "all" ]; then
    for path in "$PARENT_DIR"/wd-agent-*; do
      [ -d "$path" ] || continue
      remove_one "${path##*wd-agent-}"
    done
  elif [ -n "$target" ]; then
    remove_one "$target"
  else
    usage
  fi
  ;;
*)
  usage
  ;;
esac
