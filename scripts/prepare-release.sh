#!/usr/bin/env bash
# Prepare the next release: pick the version from the latest vX.Y.Z tag, set it on every
# workspace crate (and [workspace.package] / [workspace.dependencies] when they exist), move
# CHANGELOG.md's [Unreleased] notes under it, and refresh Cargo.lock. Adapted from aura's
# scripts/prepare-release.sh; redline versions all its crates in lockstep, since they ship as
# one binary. Committing, tagging and pushing the tag (which runs release.yml) stay manual.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

BUMP_KIND="${BUMP_KIND:-patch}"
PRE_ID="${PRE_ID:-}"
REMOTE="${REMOTE:-origin}"
EXPLICIT_VERSION=""
STABLE_TAG_RE='^v[0-9]+\.[0-9]+\.[0-9]+$'
VERSION_RE='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z][0-9A-Za-z.-]*)?$'
CHANGELOG=CHANGELOG.md

usage() {
    cat <<'EOF'
Usage: scripts/prepare-release.sh [options]

Fetches the release tags, computes the next version from the latest stable
tag (vX.Y.Z), and sets it on every workspace crate.

Options:
  --bump <kind>        patch|minor|major. Default: patch
  --pre <id>           Make it a prerelease: --bump minor --pre beta -> X.Y.0-beta.
                       Prerelease tags are published as prereleases, never as
                       the latest release.
  --version <ver>      Use this exact version (X.Y.Z or X.Y.Z-pre) instead
  --remote <name>      Git remote to fetch tags from. Default: origin
  --no-fetch           Skip 'git fetch --tags'
  --dry-run            Show the plan without editing files
  -h, --help           Show this help

Environment overrides: BUMP_KIND, PRE_ID, REMOTE

What it edits:
  - Cargo.toml of every workspace member with a literal version (all set to
    the same version), [workspace.package] version, and the version of
    member crates in [workspace.dependencies], when those exist.
  - CHANGELOG.md, stable releases only: [Unreleased] notes move under
    "## [X.Y.Z] - <today>" and the compare links are updated. A prerelease
    leaves it alone; its release notes come from [Unreleased].
  - Cargo.lock (cargo update --workspace: members only, deps stay pinned).
EOF
}

die() {
    echo "error: $*" >&2
    exit 1
}

require_tool() {
    command -v "$1" >/dev/null 2>&1 || die "required tool not found: $1"
}

bump_version() {
    local major minor patch
    IFS=. read -r major minor patch <<<"$1"
    case "$2" in
        patch) patch=$((patch + 1)) ;;
        minor) minor=$((minor + 1)); patch=0 ;;
        major) major=$((major + 1)); minor=0; patch=0 ;;
        *) die "unsupported bump kind: $2" ;;
    esac
    printf '%s.%s.%s\n' "$major" "$minor" "$patch"
}

# Prints -1, 0 or 1 for left <, =, > right. Compares X.Y.Z only.
compare_core() {
    local l r i
    IFS=. read -r -a l <<<"${1%%-*}"
    IFS=. read -r -a r <<<"${2%%-*}"
    for i in 0 1 2; do
        if (( l[i] < r[i] )); then echo -1; return; fi
        if (( l[i] > r[i] )); then echo 1; return; fi
    done
    echo 0
}

# set_section_version <manifest> <section> <version>: the first `version = "..."` of [section].
set_section_version() {
    SECTION="$2" VERSION="$3" perl -0pi -e '
        my ($s, $v) = (quotemeta $ENV{SECTION}, $ENV{VERSION});
        s/(^\[$s\]\h*\n(?:(?!\[)\N*\n)*?version\s*=\s*")[^"]*(")/$1$v$2/m;
    ' "$1"
}

# set_workspace_dep_version <crate> <version>: `crate = { path = ..., version = "..." }`.
set_workspace_dep_version() {
    CRATE="$1" VERSION="$2" perl -0pi -e '
        my ($c, $v) = (quotemeta $ENV{CRATE}, $ENV{VERSION});
        s/(^\[workspace\.dependencies\]\h*\n(?:(?!\[)\N*\n)*?$c\s*=\s*\{[^}\n]*\bversion\s*=\s*")[^"]*(")/$1$v$2/m;
    ' Cargo.toml
}

# Moves the [Unreleased] notes under the new version and updates the compare links.
update_changelog() {
    local version="$1" tag="v$1" today
    today="$(date +%Y-%m-%d)"
    VERSION="$version" TAG="$tag" TODAY="$today" perl -0pi -e '
        my ($v, $t, $d) = @ENV{qw(VERSION TAG TODAY)};
        s/^## \[Unreleased\]\n/## [Unreleased]\n\n## [$v] - $d\n/m or die "no ## [Unreleased] heading\n";
        if (s{^\[Unreleased\]: (\S+)/compare/(\S+)\.\.\.HEAD$}{[Unreleased]: $1/compare/$t...HEAD\n[$v]: $1/compare/$2...$t}m) {}
        else { warn "no [Unreleased] compare link; add the [$v] link by hand\n" }
    ' "$CHANGELOG"
}

unreleased_notes() {
    awk '/^## \[Unreleased\]/ {on=1; next} on && /^## \[/ {exit} on' "$CHANGELOG" | grep -c '[^[:space:]]' || true
}

FETCH_TAGS=true
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --bump) [[ $# -ge 2 ]] || die "--bump requires a value"; BUMP_KIND="$2"; shift 2 ;;
        --pre) [[ $# -ge 2 ]] || die "--pre requires a value"; PRE_ID="$2"; shift 2 ;;
        --version) [[ $# -ge 2 ]] || die "--version requires a value"; EXPLICIT_VERSION="${2#v}"; shift 2 ;;
        --remote) [[ $# -ge 2 ]] || die "--remote requires a value"; REMOTE="$2"; shift 2 ;;
        --no-fetch) FETCH_TAGS=false; shift ;;
        --dry-run) DRY_RUN=true; shift ;;
        -h|--help) usage; exit 0 ;;
        *) die "unknown option: $1" ;;
    esac
done

case "$BUMP_KIND" in
    patch|minor|major) ;;
    *) die "invalid bump kind: $BUMP_KIND" ;;
esac
[[ -z "$PRE_ID" || "$PRE_ID" =~ ^[0-9A-Za-z][0-9A-Za-z.-]*$ ]] || die "invalid prerelease id: $PRE_ID"

require_tool git
require_tool perl
require_tool cargo
require_tool jq

if $FETCH_TAGS; then
    git fetch --tags --quiet "$REMOTE"
fi

LATEST_TAG="$(git tag --list 'v*' --sort=-v:refname | grep -E "$STABLE_TAG_RE" | head -n1 || true)"
LATEST_VERSION="${LATEST_TAG#v}"

if [[ -n "$EXPLICIT_VERSION" ]]; then
    TARGET_VERSION="$EXPLICIT_VERSION"
elif [[ -n "$LATEST_TAG" ]]; then
    TARGET_VERSION="$(bump_version "$LATEST_VERSION" "$BUMP_KIND")${PRE_ID:+-$PRE_ID}"
else
    die "no stable release tag (vX.Y.Z) found; pass --version"
fi
[[ "$TARGET_VERSION" =~ $VERSION_RE ]] || die "invalid version: $TARGET_VERSION"
TARGET_TAG="v$TARGET_VERSION"

if [[ -n "$LATEST_TAG" && "$(compare_core "$TARGET_VERSION" "$LATEST_VERSION")" != 1 ]]; then
    die "$TARGET_VERSION is not newer than the latest release $LATEST_TAG"
fi
if git rev-parse -q --verify "refs/tags/$TARGET_TAG" >/dev/null; then
    die "tag $TARGET_TAG already exists"
fi

# Workspace members: name, manifest, and whether [package] has a literal version.
METADATA="$(cargo metadata --no-deps --format-version 1)"
mapfile -t MEMBERS < <(jq -r '.packages[] | "\(.name)\t\(.manifest_path)\t\(.version)"' <<<"$METADATA")

echo "Latest stable release: ${LATEST_TAG:-none}"
echo "Next release: $TARGET_TAG$([[ "$TARGET_VERSION" == *-* ]] && echo ' (prerelease, not marked latest)')"
for m in "${MEMBERS[@]}"; do
    IFS=$'\t' read -r name manifest current <<<"$m"
    printf '  %s: %s -> %s (%s)\n' "$name" "$current" "$TARGET_VERSION" "${manifest#"$PWD/"}"
done

PRERELEASE=false
[[ "$TARGET_VERSION" == *-* ]] && PRERELEASE=true
if ! $PRERELEASE; then
    [[ "$(unreleased_notes)" -gt 0 ]] || echo "warning: CHANGELOG.md has no [Unreleased] notes; the release notes will be empty" >&2
    echo "  $CHANGELOG: [Unreleased] -> [$TARGET_VERSION]"
fi

if $DRY_RUN; then
    echo ""
    echo "Dry run only. No files were modified."
    exit 0
fi

grep -q '^\[workspace\.package\]' Cargo.toml && set_section_version Cargo.toml workspace.package "$TARGET_VERSION"
for m in "${MEMBERS[@]}"; do
    IFS=$'\t' read -r name manifest _ <<<"$m"
    set_section_version "$manifest" package "$TARGET_VERSION" # no-op with version.workspace = true
    set_workspace_dep_version "$name" "$TARGET_VERSION"
done

cargo update --workspace --quiet

# Every member must now report the target version (catches a manifest the edits missed).
while IFS=$'\t' read -r name version; do
    [[ "$version" == "$TARGET_VERSION" ]] || die "$name is still at $version; set it by hand"
done < <(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | "\(.name)\t\(.version)"')

$PRERELEASE || update_changelog "$TARGET_VERSION"

echo ""
echo "Next manual steps:"
echo "  1. Review: git diff (Cargo.toml, Cargo.lock$($PRERELEASE || echo ', CHANGELOG.md'))"
echo "  2. just check"
echo "  3. git commit -am 'chore(release): $TARGET_TAG' and merge it to main"
echo "  4. git tag -a $TARGET_TAG -m $TARGET_TAG && git push $REMOTE $TARGET_TAG"
echo "     (the tag runs .github/workflows/release.yml)"
