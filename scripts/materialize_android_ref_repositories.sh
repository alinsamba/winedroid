#!/usr/bin/env bash
set -euo pipefail

REF_ROOT="${1:-${ANDROID_REF_ROOT:-$HOME/Dev/android-ref}}"

ART_URL="https://android.googlesource.com/platform/art"
ANDROIDX_URL="https://android.googlesource.com/platform/frameworks/support"

ART_CHECKOUT="$REF_ROOT/art"
ANDROIDX_CHECKOUT="$REF_ROOT/androidx"

read_pinned_revision() {
    local checkout="$1"
    local recorded
    local current

    recorded="$(tr -d '\n' < "$checkout/.pin")"
    current="$(GIT_NO_LAZY_FETCH=1 git -C "$checkout" rev-parse 'HEAD^{commit}')"
    if [ "$recorded" != "$current" ]; then
        echo "recorded pin does not match checkout HEAD: $checkout" >&2
        return 1
    fi
    printf '%s\n' "$recorded"
}

write_present_symlink_objects() {
    local checkout="$1"
    local object_directory="$2"
    local index_record
    local metadata
    local path
    local mode
    local expected
    local stage
    local target
    local actual

    while IFS= read -r index_record; do
        metadata="${index_record%%$'\t'*}"
        path="${index_record#*$'\t'}"
        read -r mode expected stage <<< "$metadata"
        if [ "$mode" != "120000" ] || [ ! -L "$checkout/$path" ]; then
            continue
        fi
        target="$(readlink "$checkout/$path")"
        actual="$(printf '%s' "$target" \
            | GIT_NO_LAZY_FETCH=1 \
                GIT_OBJECT_DIRECTORY="$object_directory" \
                GIT_ALTERNATE_OBJECT_DIRECTORIES="$checkout/.git/objects" \
                git -C "$checkout" hash-object -w --stdin)"
        if [ "$actual" != "$expected" ]; then
            echo "symlink target does not match pin: $checkout/$path" >&2
            return 1
        fi
    done < <(GIT_NO_LAZY_FETCH=1 git -C "$checkout" ls-files -s)
}

require_clean_tracked_files() {
    local checkout="$1"
    local repository_name="$2"
    local object_directory="$TEMP_ROOT/$repository_name-index-objects"
    local refresh_output

    if ! GIT_NO_LAZY_FETCH=1 git -C "$checkout" diff-index \
        --cached --quiet HEAD --; then
        echo "refusing to overwrite staged changes in: $checkout" >&2
        return 1
    fi

    mkdir -p "$object_directory"
    write_present_symlink_objects "$checkout" "$object_directory"
    if ! refresh_output="$(GIT_NO_LAZY_FETCH=1 \
        GIT_OBJECT_DIRECTORY="$object_directory" \
        GIT_ALTERNATE_OBJECT_DIRECTORIES="$checkout/.git/objects" \
        git -C "$checkout" update-index --refresh 2>&1)"; then
        echo "refusing to overwrite tracked changes in: $checkout" >&2
        printf '%s\n' "$refresh_output" >&2
        return 1
    fi
}

has_hidden_files() {
    local checkout="$1"

    GIT_NO_LAZY_FETCH=1 git -C "$checkout" ls-files -v \
        | awk '$1 == "S" { hidden = 1 } END { exit !hidden }'
}

download_repository() {
    local repo_url="$1"
    local revision="$2"
    local repository_name="$3"
    local archive="$TEMP_ROOT/$repository_name.tar.gz"
    local staging="$TEMP_ROOT/$repository_name"

    mkdir -p "$staging"
    curl -L --fail --silent --show-error \
        --retry 3 --retry-all-errors \
        --proto '=https' --proto-redir '=https' \
        "$repo_url/+archive/$revision.tar.gz" \
        --output "$archive"
    tar -xzf "$archive" -C "$staging"
}

materialize_repository() {
    local checkout="$1"
    local repository_name="$2"

    rsync -a "$TEMP_ROOT/$repository_name/" "$checkout/"
    GIT_NO_LAZY_FETCH=1 git -C "$checkout" ls-files -z \
        | GIT_NO_LAZY_FETCH=1 git -C "$checkout" update-index \
            --no-skip-worktree -z --stdin
    require_clean_tracked_files "$checkout" "$repository_name"
    echo "materialized complete pinned repository: $checkout"
}

ART_REVISION="$(read_pinned_revision "$ART_CHECKOUT")"
ANDROIDX_REVISION="$(read_pinned_revision "$ANDROIDX_CHECKOUT")"

TEMP_ROOT="$(mktemp -d "$REF_ROOT/.materialize-android-ref.XXXXXX")"
trap 'rm -rf "$TEMP_ROOT"' EXIT

require_clean_tracked_files "$ART_CHECKOUT" art
require_clean_tracked_files "$ANDROIDX_CHECKOUT" androidx

if ! has_hidden_files "$ART_CHECKOUT" \
    && ! has_hidden_files "$ANDROIDX_CHECKOUT"; then
    echo "complete pinned repositories are already materialized"
    exit 0
fi

if has_hidden_files "$ART_CHECKOUT"; then
    download_repository "$ART_URL" "$ART_REVISION" art
fi

if has_hidden_files "$ANDROIDX_CHECKOUT"; then
    download_repository "$ANDROIDX_URL" "$ANDROIDX_REVISION" androidx
fi

if [ -d "$TEMP_ROOT/art" ]; then
    materialize_repository "$ART_CHECKOUT" art
fi

if [ -d "$TEMP_ROOT/androidx" ]; then
    materialize_repository "$ANDROIDX_CHECKOUT" androidx
fi
