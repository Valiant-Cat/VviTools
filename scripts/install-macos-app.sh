#!/bin/zsh

set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  print -u2 "install:macos 仅支持 macOS。"
  exit 1
fi

project_root="${0:A:h:h}"
source_app="$project_root/src-tauri/target/release/bundle/macos/VviTools.app"
install_root="${VVITOOLS_INSTALL_DIR:-/Applications}"
destination_app="$install_root/VviTools.app"
temporary_app="$install_root/.VviTools.app.installing.$$"

if [[ ! -d "$source_app" ]]; then
  print -u2 "未找到构建产物：$source_app"
  print -u2 "请先运行 npm run tauri -- build --bundles app。"
  exit 1
fi

mkdir -p "$install_root"

cleanup() {
  rm -rf "$temporary_app"
}
trap cleanup EXIT

ditto "$source_app" "$temporary_app"
restart_app=false
for app_pid in $(pgrep -x vvitools || true); do
  running_path="$(ps -p "$app_pid" -o comm= || true)"
  if [[ "$running_path" != "$destination_app/Contents/MacOS/vvitools" ]]; then
    continue
  fi
  restart_app=true
  kill -TERM "$app_pid"
  for attempt in {1..50}; do
    if ! kill -0 "$app_pid" 2>/dev/null; then
      break
    fi
    sleep 0.1
  done
  if kill -0 "$app_pid" 2>/dev/null; then
    print -u2 "旧版 VviTools 未退出，已取消替换。请退出应用后重试。"
    exit 1
  fi
done
rm -rf "$destination_app"
mv "$temporary_app" "$destination_app"
touch "$destination_app"
rm -rf "$source_app"

if [[ "$restart_app" == true ]]; then
  open "$destination_app"
fi

print "已安装到 $destination_app，并已清理构建目录中的应用副本"
