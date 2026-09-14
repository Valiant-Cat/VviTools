#!/bin/zsh

set -euo pipefail

identity="VviTools Local Development"
project_root="${0:A:h:h}"
login_keychain="$(security default-keychain -d user | sed -E 's/^[[:space:]]*"//; s/"[[:space:]]*$//')"

if ! security find-identity -v -p codesigning "$login_keychain" | grep -Fq "\"$identity\""; then
  print -u2 "未找到稳定代码签名身份：$identity"
  print -u2 "请先运行 npm run signing:setup。"
  exit 1
fi

cd "$project_root"

npm run tauri -- build \
  --bundles app \
  --config "{\"bundle\":{\"macOS\":{\"signingIdentity\":\"$identity\"}}}"

npm run install:macos
