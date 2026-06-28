# 클립샷(ClipShot) 배포 식별자. 비밀 값은 repo에 넣지 말고 로컬/ CI secret에 둔다.
# Tauri 기반 직접 배포(웹사이트 다운로드). 빌드는 packaging/build.sh 참고.

export APP_DISPLAY_NAME="클립샷"
export APP_PRODUCT_NAME="ClipShot"
export APP_BUNDLE_ID="com.heung.clip-shot"   # 출시 전 본인 reverse-DNS로 교체 권장

# ── 빌드(각 OS에서 실행) ──
#   cd utilities/clip-shot && ./packaging/build.sh
#   결과물: src-tauri/target/release/bundle/  (macOS: dmg, Windows: msi/nsis)
#
# ── macOS 공증(Apple Developer $99 가입 시) ──
#   Developer ID Application 인증서로 서명 후 notarytool로 공증·staple.
#   서명 ID는 환경변수로 주입(예시, 실제 값은 커밋 금지):
# export APPLE_SIGNING_IDENTITY="Developer ID Application: NAME (TEAMID)"
# export APPLE_ID="you@example.com"
# export APPLE_PASSWORD="<app-specific-password>"
# export APPLE_TEAM_ID="<TEAMID>"
#
# ── Windows 코드서명(선택) ──
#   SmartScreen 경고 완화를 위해 인증서로 서명. 인증서/암호는 CI secret으로.
