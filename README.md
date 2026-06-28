# 클립샷 (ClipShot)

화면을 캡처하면 **즉시 클립보드에 복사**되고, 동시에 **최근 30개**가 자동으로 보관되는
가벼운 크로스플랫폼(Windows/macOS) 화면 캡처 유틸리티입니다. 캡처하자마자 어디든 바로
붙여넣기(⌘V / Ctrl+V) 할 수 있고, 지난 캡처는 히스토리에서 다시 꺼내 복사·내보내기 할 수 있습니다.

## 핵심 기능

- **4가지 캡처 모드**: 영역 / 창 / 디스플레이 / 모든 디스플레이
- **저장 = 클립보드**: 캡처 즉시 클립보드 복사 + 최근 N개(기본 30) 자동 히스토리
- **히스토리 뷰어**: 썸네일 그리드에서 다시 복사 / 파일로 내보내기(PNG·JPG) / 삭제 / 전체 비우기
- **전역 단축키**: 어디서든 캡처(설정에서 재바인딩). 기본값은 `CmdOrCtrl+Shift+2/W/3/4/H`
- **트레이 상주**: 메뉴바/트레이에서 빠르게 캡처, 창을 닫아도 백그라운드 동작
- **효과/옵션**: 캡처 플래시, 셔터 소리, 완료 토스트, 시작 프로그램 등록, 한/영 전환
- **가벼움 & 로컬 전용**: Tauri 기반 작은 바이너리. 모든 처리는 기기 내에서만, 외부 전송 없음

## 기술 스택

- **셸/코어**: Tauri 2 (Rust)
- **프론트엔드**: SvelteKit + TypeScript (SPA, adapter-static)
- **캡처**: `xcap` (macOS ScreenCaptureKit/CoreGraphics, Windows WGC/BitBlt)
- **플러그인**: global-shortcut, clipboard-manager, autostart, dialog

## 개발

전제: Node 18+, Rust(stable). 최초 1회 `rustup`으로 Rust 설치.

```bash
npm install
npm run tauri dev      # 개발 실행(핫리로드)
```

빌드/검증:

```bash
# 프론트엔드
npm run check          # svelte-check + tsc
npm run build          # 정적 산출물 → build/

# Rust 코어
cd src-tauri
cargo test             # 단위 테스트(히스토리/설정/단축키/크롭·스티치)
cargo clippy --all-targets
```

릴리스 빌드(직접 배포용):

```bash
npm run tauri build    # macOS: .app/.dmg, Windows: .msi/.exe (각 OS에서 실행)
# 결과물: src-tauri/target/release/bundle/
```

## 권한

- **macOS**: 첫 캡처 시 *화면 기록* 권한이 필요합니다. 시스템 설정 → 개인정보 보호 및 보안 →
  화면 기록에서 ClipShot을 허용하세요. 전역 단축키는 별도 권한이 필요 없습니다.
- **Windows**: 화면 캡처·전역 단축키 모두 별도 권한이 필요 없습니다.

## 알려진 한계 (v1)

- 영역 캡처는 **주 모니터** 기준입니다(멀티모니터 영역 선택은 추후).
- 영상(화면 녹화)은 v1 범위에서 제외했습니다.

자세한 작업 지침은 `AGENTS.md`, 로드맵은 `ROADMAP.md`를 참고하세요.
