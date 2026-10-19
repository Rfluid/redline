#!/usr/bin/env bash
# Point this clone at a release instead of main: install and update both run it.
#   scripts/checkout-release.sh            # the latest release (highest vX.Y.Z; prereleases skipped)
#   scripts/checkout-release.sh v0.3.0-beta  # a specific tag, prereleases included
# bin/redline then rebuilds (cargo) or downloads that release's binary on its next run.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

die() {
    echo "error: $*" >&2
    exit 1
}

case "${1:-}" in
    -h|--help) sed -n '2,5p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
esac

git fetch --tags --force --quiet "${REMOTE:-origin}" || die "could not fetch tags from ${REMOTE:-origin}"

tag="${1:-}"
if [[ -z "$tag" ]]; then
    tag="$(git tag --list 'v*' --sort=-v:refname | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | head -n1 || true)"
    [[ -n "$tag" ]] || die "no release tags (vX.Y.Z) found"
fi
git rev-parse -q --verify "refs/tags/$tag" >/dev/null || die "no such tag: $tag"

# Never discard local edits to tracked files (untracked ones, like target/, are fine).
git diff --quiet HEAD -- || die "this clone has local changes; commit or stash them first"

current="$(git describe --tags --exact-match 2>/dev/null || true)"
if [[ "$current" == "$tag" ]]; then
    echo "redline: already on $tag"
else
    git -c advice.detachedHead=false checkout --quiet --detach "$tag"
    echo "redline: ${current:-$(git rev-parse --short HEAD@{1})} -> $tag"
fi
