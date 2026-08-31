#!/usr/bin/env bash
# 校验 Release tag 与各构建入口的版本一致，CHANGELOG 有对应小节，
# 且 updater endpoint 指向本仓库（防占位符忘改）。
# Validates the release tag against every version entry, requires a CHANGELOG
# section, and ensures the updater endpoint points at this repository.
set -euo pipefail

TAG="${GITHUB_REF_NAME:?缺少 GITHUB_REF_NAME}"
if [[ ! "${TAG}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$ ]]; then
    echo "Release tag 必须是 vMAJOR.MINOR.PATCH（可带预发布或构建后缀）：${TAG}" >&2
    exit 1
fi

VERSION="${TAG#v}"
METADATA_FILE="$(mktemp)"
trap 'rm -f "${METADATA_FILE}"' EXIT

cargo metadata \
    --manifest-path src-tauri/Cargo.toml \
    --no-deps \
    --format-version 1 >"${METADATA_FILE}"

CARGO_VERSION="$(node -e '
const fs = require("fs");
const metadata = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
const packageInfo = metadata.packages.find((item) => item.name === "freetex");
if (!packageInfo) process.exit(1);
process.stdout.write(packageInfo.version);
' "${METADATA_FILE}")"
CONFIG_VERSION="$(node -e 'process.stdout.write(require("./src-tauri/tauri.conf.json").version)' )"
FRONTEND_VERSION="$(node -e 'process.stdout.write(require("./frontend/package.json").version)' )"

for entry in "Cargo=${CARGO_VERSION}" "Tauri=${CONFIG_VERSION}" "Frontend=${FRONTEND_VERSION}"; do
    name="${entry%%=*}"
    version="${entry#*=}"
    if [[ "${version}" != "${VERSION}" ]]; then
        echo "${name} 版本 ${version} 与 tag ${TAG} 不一致" >&2
        exit 1
    fi
done

if ! grep -Fq "## v${VERSION} " CHANGELOG.md; then
    echo "CHANGELOG.md 缺少 v${VERSION} 版本小节" >&2
    exit 1
fi

# updater endpoint 必须指向当前仓库，防止发布到别人的仓库或占位符忘改
# the updater endpoint must point at this repo (catches a forgotten placeholder)
REPO="${GITHUB_REPOSITORY:?缺少 GITHUB_REPOSITORY}"
ENDPOINT="$(node -e 'process.stdout.write(require("./src-tauri/tauri.conf.json").plugins.updater.endpoints[0])')"
if [[ "${ENDPOINT}" != "https://github.com/${REPO}/releases/latest/download/latest.json" ]]; then
    echo "updater endpoint 应为 https://github.com/${REPO}/releases/latest/download/latest.json，实际为 ${ENDPOINT}" >&2
    exit 1
fi

echo "Release ${TAG} 版本校验通过（Cargo/Tauri/Frontend/CHANGELOG/updater endpoint）"
