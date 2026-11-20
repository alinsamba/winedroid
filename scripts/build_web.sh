#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
target_dir="$project_root/target/wasm32-unknown-emscripten/release"
distribution_dir="$project_root/web/dist"
emscripten_cache="$project_root/target/emscripten-cache"
skia_key="da8fc6731fc439bc3b6a-wasm32-unknown-emscripten-jpegd-jpege-pdf"
skia_cache="$emscripten_cache/rust-skia-0.97.2"
skia_archive="$skia_cache/skia-binaries-$skia_key.tar.gz"
skia_compatible_archive="$skia_cache/skia-binaries-$skia_key-compatible.tar.gz"
skia_compatible_checksum="$skia_compatible_archive.sha256"
skia_compatible_source="$skia_compatible_archive.source-sha256"
skia_url="https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-$skia_key.tar.gz"
skia_sha256="d475029738dea1aa4c645e5e0bce24497a88b5bd489efb5d788e3eee2132edeb"

mkdir -p "$distribution_dir" "$emscripten_cache" "$skia_cache"

if [ ! -f "$skia_archive" ]; then
    curl -L --fail --retry 3 --output "$skia_archive.partial" "$skia_url"
    downloaded_sha256=$(shasum -a 256 "$skia_archive.partial" | awk '{print $1}')
    if [ "$downloaded_sha256" != "$skia_sha256" ]; then
        echo "rust-skia archive checksum mismatch" >&2
        exit 1
    fi
    mv "$skia_archive.partial" "$skia_archive"
fi

cached_sha256=$(shasum -a 256 "$skia_archive" | awk '{print $1}')
if [ "$cached_sha256" != "$skia_sha256" ]; then
    echo "cached rust-skia archive checksum mismatch: $skia_archive" >&2
    exit 1
fi

rebuild_compatible=0
if [ ! -f "$skia_compatible_archive" ] || [ ! -f "$skia_compatible_checksum" ] || [ ! -f "$skia_compatible_source" ]; then
    rebuild_compatible=1
else
    compatible_sha256=$(shasum -a 256 "$skia_compatible_archive" | awk '{print $1}')
    recorded_compatible_sha256=$(sed -n '1p' "$skia_compatible_checksum")
    recorded_source_sha256=$(sed -n '1p' "$skia_compatible_source")
    if [ "$compatible_sha256" != "$recorded_compatible_sha256" ] || [ "$recorded_source_sha256" != "$skia_sha256" ]; then
        rebuild_compatible=1
    fi
fi

if [ "$rebuild_compatible" -eq 1 ]; then
    skia_unpack_dir=$(mktemp -d "$emscripten_cache/skia-archive.XXXXXX")
    trap 'rm -r -- "$skia_unpack_dir"' EXIT HUP INT TERM
    tar -xzf "$skia_archive" -C "$skia_unpack_dir"
    cp \
        "$skia_unpack_dir/skia-binaries/libskia.a" \
        "$skia_unpack_dir/skia-binaries/libskia.wasm.a"
    tar -czf "$skia_compatible_archive.partial" -C "$skia_unpack_dir" skia-binaries
    mv "$skia_compatible_archive.partial" "$skia_compatible_archive"
    compatible_sha256=$(shasum -a 256 "$skia_compatible_archive" | awk '{print $1}')
    printf '%s\n' "$compatible_sha256" > "$skia_compatible_checksum.partial"
    mv "$skia_compatible_checksum.partial" "$skia_compatible_checksum"
    printf '%s\n' "$skia_sha256" > "$skia_compatible_source.partial"
    mv "$skia_compatible_source.partial" "$skia_compatible_source"
    rm -r -- "$skia_unpack_dir"
    trap - EXIT HUP INT TERM
fi

cd "$project_root"

env \
    CARGO_TARGET_DIR="$project_root/target" \
    EM_CACHE="$emscripten_cache" \
    SKIA_BINARIES_URL="file://$skia_compatible_archive" \
    cargo build \
        --manifest-path "$project_root/Cargo.toml" \
        --release \
        --locked \
        --target wasm32-unknown-emscripten \
        --no-default-features \
        --bin winedroid_web

cp "$target_dir/winedroid_web.js" "$distribution_dir/winedroid.js"
cp "$target_dir/winedroid_web.wasm" "$distribution_dir/winedroid_web.wasm"
