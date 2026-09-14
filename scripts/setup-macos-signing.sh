#!/bin/zsh

set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  print -u2 "signing:setup 仅支持 macOS。"
  exit 1
fi

identity="VviTools Local Development"
login_keychain="$(security default-keychain -d user | sed -E 's/^[[:space:]]*"//; s/"[[:space:]]*$//')"

if security find-identity -v -p codesigning "$login_keychain" | grep -Fq "\"$identity\""; then
  print "代码签名身份已存在：$identity"
  exit 0
fi

temporary_dir="$(mktemp -d)"
certificate_path="$temporary_dir/vvitools-local.crt"
private_key_path="$temporary_dir/vvitools-local.key"
archive_path="$temporary_dir/vvitools-local.p12"
archive_password="$(openssl rand -hex 24)"

cleanup() {
  find "$temporary_dir" -type f -delete 2>/dev/null || true
  rmdir "$temporary_dir" 2>/dev/null || true
}
trap cleanup EXIT

openssl req -new -newkey rsa:3072 -x509 -sha256 -days 3650 -nodes \
  -subj "/CN=$identity/O=VviTools Local Development" \
  -addext "basicConstraints=critical,CA:FALSE" \
  -addext "keyUsage=critical,digitalSignature" \
  -addext "extendedKeyUsage=critical,codeSigning" \
  -keyout "$private_key_path" \
  -out "$certificate_path" \
  >/dev/null 2>&1

openssl pkcs12 -export -legacy \
  -inkey "$private_key_path" \
  -in "$certificate_path" \
  -name "$identity" \
  -passout "pass:$archive_password" \
  -out "$archive_path"

security import "$archive_path" \
  -k "$login_keychain" \
  -P "$archive_password" \
  -T /usr/bin/codesign \
  -T /usr/bin/security

security add-trusted-cert \
  -r trustRoot \
  -p codeSign \
  -k "$login_keychain" \
  "$certificate_path"

if ! security find-identity -v -p codesigning "$login_keychain" | grep -Fq "\"$identity\""; then
  print -u2 "证书已导入，但未被识别为有效代码签名身份。"
  exit 1
fi

print "已创建代码签名身份：$identity"
