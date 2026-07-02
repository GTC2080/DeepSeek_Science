#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd -P)"
CACHE_PARENT="$(cd "${PROJECT_ROOT}/.." && pwd -P)/.cache"
TARGET="${CACHE_PARENT}/deepseek-science-target"

if [[ -z "${TARGET}" || "${TARGET}" == "/" || "${TARGET}" == "${HOME}" ]]; then
  echo "Refusing to delete suspicious target: ${TARGET}" >&2
  exit 1
fi

if [[ "${TARGET}" == "." || "${TARGET}" == ".." ]]; then
  echo "Refusing to delete relative current/parent directory: ${TARGET}" >&2
  exit 1
fi

case "${TARGET}" in
  "${CACHE_PARENT}/deepseek-science-target") ;;
  *)
    echo "Refusing to delete outside expected cache location: ${TARGET}" >&2
    exit 1
    ;;
esac

echo "Planned delete target:"
echo "  ${TARGET}"

if [[ ! -d "${TARGET}" ]]; then
  echo "Nothing to delete."
  exit 0
fi

if command -v du >/dev/null 2>&1; then
  SIZE="$(du -sh "${TARGET}" 2>/dev/null | awk '{print $1}')"
  echo "Approximate size: ${SIZE:-unknown}"
fi

printf "Type DELETE to remove this directory: "
read -r CONFIRMATION

if [[ "${CONFIRMATION}" != "DELETE" ]]; then
  echo "Aborted."
  exit 1
fi

rm -rf -- "${TARGET}"
echo "Removed: ${TARGET}"
