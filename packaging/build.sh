#!/usr/bin/env bash
# 클립샷 릴리스 번들 빌드. 실행한 OS에 맞는 설치파일을 생성한다.
#   macOS  → .app / .dmg
#   Windows→ .msi / .exe(NSIS)   (Windows에서 실행할 것)
#
# 사용:  cd utilities/clip-shot && ./packaging/build.sh
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Apple Developer 가입 시 서명 ID를 환경변수로 주입하면 Tauri가 서명까지 수행한다.
#   export APPLE_SIGNING_IDENTITY / APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID
# (deploy.config.sh 참고)

echo "▶ 의존성 설치"
npm install

echo "▶ Tauri 릴리스 빌드"
npm run tauri build

echo
echo "✅ 완료. 산출물:"
echo "   src-tauri/target/release/bundle/"
ls -1 src-tauri/target/release/bundle 2>/dev/null || true
