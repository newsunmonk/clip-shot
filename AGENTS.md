# AGENTS.md — 클립샷(ClipShot)

> Codex와 Claude Code가 이 앱에서 공유하는 작업 지침입니다. 사람용 소개는 `README.md`를 봅니다.

## 앱 개요

- 한 줄 설명: 캡처 즉시 클립보드 복사 + 최근 30개 자동 히스토리. 영역/창/디스플레이/모든 디스플레이 캡처를 지원하는 가벼운 Windows·macOS 유틸리티.
- 플랫폼: Tauri 2(Rust 코어 + SvelteKit 웹 UI) 데스크탑. macOS / Windows.
- 제품 이름: `ClipShot`(표시 이름 한국어 "클립샷"), 번들 ID `com.heung.clip-shot`.
- 범위: **이미지 캡처만**(영상 녹화 제외). 저장은 **클립보드 + 히스토리 중심**(지정 폴더 자동 저장 없음, 히스토리에서 수동 내보내기).

## 기술 스택과 구조

- 트레이 상주 앱. 메인 창을 닫으면 종료가 아니라 숨김(트레이·전역 단축키 유지).
- 영역 선택은 별도 투명 오버레이 창(label `overlay`). 같은 번들을 로드하고 `+page.svelte`가 창 label로 메인/오버레이 UI를 분기한다.
- 폴더 구조
  - `src-tauri/src/settings.rs` — 설정 모델 + JSON 영속화 + 하위호환(모든 신규 필드 `#[serde(default)]`, 로드 시 `sanitize`로 clamp).
  - `src-tauri/src/history.rs` — 히스토리 저장소. `push_and_trim`(링버퍼, 기본 30 상한), 풀 PNG+썸네일, `unique_filename`(내보내기 충돌 접미사).
  - `src-tauri/src/capture.rs` — xcap 캡처. 순수 함수 `crop`/`stitch` + 디스플레이/모든디스플레이/창/영역 캡처.
  - `src-tauri/src/clipboard.rs` — RGBA → `tauri::image::Image` 클립보드 write.
  - `src-tauri/src/hotkeys.rs` — 가속기 문자열 `validate`/`find_conflicts`(global_hotkey 규칙과 동일).
  - `src-tauri/src/lib.rs` — 상태/명령/트레이/전역 단축키 등록/창 수명 관리(`finalize`가 캡처 공통 마무리).
  - `src/lib/` — `Main.svelte`(캡처바·히스토리·설정·창 선택), `Overlay.svelte`(영역 선택), `ipc.ts`(타입·invoke), `i18n.ts`(한/영).

## 핵심 동작 규칙

- 모든 캡처는 `finalize`를 거친다: **클립보드 write → 히스토리 add(풀 PNG+썸네일) → `captured` 이벤트 emit**. 30개 초과 시 가장 오래된 항목을 파일째 제거.
- 전역 단축키 핸들러는 동작을 직접 수행하지 않고 `hotkey-action` 이벤트를 프론트로 보낸다. 프론트가 영역=오버레이, 창=picker, 디스플레이/모든=즉시 캡처로 분기한다.
- 설정 저장 시 `hotkeys::validate_all`로 단축키 유효성·충돌을 검사한 뒤 영속화하고 단축키를 재등록한다.
- 영역 좌표는 오버레이의 CSS(logical)px × 주 모니터 scale → 물리 px로 변환해 `capture_region`에 넘긴다.
- 신규 영속 필드는 항상 기본값을 주어 옛 settings.json과 호환되게 한다.

## 현지화(한/영)

- 기본 한국어(`ko`), 영어(`en`). UI 문자열은 `src/lib/i18n.ts`의 dict에 ko/en을 함께 등록하고 `t(lang, key)`로 조회한다. 새 문자열을 추가하면 두 언어를 같이 넣는다.
- 트레이 메뉴 라벨 등 Rust 측 한국어 문자열은 현재 한국어 고정(추후 필요 시 현지화).

## 빌드와 테스트

```bash
cd utilities/clip-shot
npm install
npm run check                       # 프론트 타입체크
npm run build                       # 프론트 정적 빌드
cd src-tauri && cargo test          # Rust 단위 테스트
cargo clippy --all-targets          # 린트(경고 0 유지)
npm run tauri dev                   # 통합 실행
npm run tauri build                 # 릴리스 번들(각 OS에서)
```

- 캡처/오버레이/권한은 OS 의존이라 자동 테스트 대상이 아니다. 순수 로직(`crop`/`stitch`/링버퍼/설정 마이그레이션/단축키 검증)을 `cargo test`로 보증한다.
- 프로젝트 구조 변경 시 `src-tauri/capabilities/default.json`의 windows/permissions와 `tauri.conf.json`을 함께 갱신한다.

## 배포 (직접 배포)

- `npm run tauri build` 산출물(macOS `.dmg`, Windows `.msi`/`.exe`)을 직접 배포한다. `packaging/build.sh` 참고.
- macOS는 Apple Developer($99) Developer ID 서명 + 공증(notarize) 권장. 미가입 시 ad-hoc 빌드도 가능(첫 실행 Gatekeeper 경고 안내 필요).
- 식별자는 `deploy.config.sh`에 두고 비밀 값은 repo 밖/CI secret에 둔다.

## 개인정보

- 모든 캡처·히스토리 처리는 기기 내에서만 일어난다. 이미지·경로를 외부로 전송하지 않고 추적도 하지 않는다.
- 히스토리는 OS 앱 데이터 디렉터리(`history/`)에 저장된다.
