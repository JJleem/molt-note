# TASK-010 Evidence — §14.3의 UNVERIFIED 항목이 승격되지 않았음을 보이는 대조표

AC3은 "§14.3의 UNVERIFIED 항목이 VERIFIED로 승격되지 않았다"를 요구한다.
아래는 `docs/PRODUCT-SPEC.md` §14.3(줄 729~781)이 UNVERIFIED 또는 조건부 표기로 남긴 항목과,
`docs/ADR-0003-recording-engine.md`에서의 표기를 1:1로 대조한 것이다.

ADR의 태그 규칙은 ADR §3에 정의되어 있다.
`[R]` 저장소 근거 · `[A✓]` 파일로 확인 · `[A?]` 자동 검증 가능하나 미실행 ·
`[H]` Human Review · `[U]` UNVERIFIED.

| § 14.3의 항목 | §14.3 표기 | ADR-0003 표기 | 승격 여부 |
| --- | --- | --- | --- |
| 2026년 macOS Tauri 번들 WKWebView에서 MediaRecorder pause/resume 실동작 | UNVERIFIED | §5.4 `[R §14.3][U]` — "§14.3이 명시적으로 UNVERIFIED로 남겼다" | 승격 없음 |
| WKWebView MediaRecorder의 실제 컨테이너/코덱 | 확인 항목 1 (UNVERIFIED) | §5.7 · §7.1 `[R §14.3][U]` | 승격 없음 |
| 번들된 `.app`에서 마이크 TCC 프롬프트가 우회 없이 뜨는가 (#11951) | 확인 항목 2 (UNVERIFIED) | §5.9 · §7.1 `[R §14.3][U]`, §12 항목 1 | 승격 없음 |
| 1시간 규모 녹음 안정성 · crash 내성 (R-005) | 확인 항목 3 (UNVERIFIED) | §5.1 · §7.1 · §9 `[R §14.3][U]` | 승격 없음 |
| WebView2와 WKWebView의 출력 포맷 일치 여부 | 확인 항목 4 (UNVERIFIED) | §5.10 · §7.1 `[R §14.3][U]` | 승격 없음 |
| Windows 마이크 privacy 토글 차단 시 앱이 받는 정확한 오류 | UNVERIFIED | §5.10 · §10-5 `[R §14.3][U]` | 승격 없음 |
| Windows Win32 앱에 manifest 선언 불필요 | VERIFIED-by-secondary-corroboration | §5.10 · §10-5 — **같은 문구를 그대로 유지**했다 | 표기 유지 |
| Safari 14.1+ pause/resume 지원 | VERIFIED | §5.4 `[R §14.3] VERIFIED` (스펙/브라우저 지원에 한함) | 사실 범위 유지 |
| 커뮤니티 플러그인의 star 수 · 유지보수 상태 | "관찰이지 결정이 아니다" | §5.14 · §7.2 — 같은 문장을 인용하고, star 수를 그 자체로 탈락 근거로 쓰지 않는다고 명시 | 표기 유지 |

## §14.3에 없어서 이 문서가 UNVERIFIED로 새로 표시한 것

§14.3이 조사하지 않은 항목을 "알려진 사실"처럼 쓰지 않았다. ADR에서 `[U]`로 남긴 것:

- webview의 장치 열거/선택 API 동작과 권한 이전 label 노출 여부 (§5.2 · §5.3)
- 커뮤니티 플러그인이 고정하는 출력 포맷 · 열거/선택 API 표면 · pause/resume 지원 (§5.2~§5.7 · §7.2)
- `cpal` crash 시 부분 WAV의 복구 가능성 · 디스크 backpressure 시의 손실 (§5.6)
- Tauri #5853의 재현 여부 (§5.6 · §7.1 — "보고의 존재"는 확인된 사실, 재현은 하지 않음)

## §14.1 인용에 대한 주의

ADR §5.11은 로컬 rustc 1.94.0이 `cpal` MSRV 1.85를 만족한다고 적으면서
"**§14.1의 값을 인용한 것이며 이 Task가 `rustc --version`을 다시 실행한 것은 아니다**"를
같은 줄에 명시했다. 이 Run에서는 self-check 외의 명령 실행이 허용되지 않았다.
