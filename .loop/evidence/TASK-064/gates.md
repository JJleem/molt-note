# TASK-064 — Gate 실행 결과

실행: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
(Runtime 소유 진입점 · 참고용. 완료 판정은 Runtime과 Verifier의 몫이다.)

```text
build: PASS  exit=0  1.1s   (npm run build   — tsc && vite build)
lint:  PASS  exit=0  1.9s   (npm run lint    — eslint . && cargo clippy -D warnings)
test:  PASS  exit=0  4.8s   (npm run test    — vitest run && cargo test)
```

## vitest 요약 (test Gate 안)

```text
 Test Files  26 passed (26)
      Tests  494 passed (494)
```

이 Task 이전은 25 파일이었다. 늘어난 하나가 `tests/ui-foundation.test.ts`이며 26개 검사를
담고 있다.

## 이 Task가 더한 검사

`tests/ui-foundation.test.ts` — `phase-prompt/05.5`의 두 번째 성공 기준(요구 9 · 10 · 11 · 12)
다섯 가지.

```text
1  타입/여백 스케일 정의 · 사용         describe('타입 스케일과 여백 스케일이 있고, 화면이 그 위에 선다')
2  보이는 focus                          describe('보이는 focus 상태가 있다')
3  dark 블록의 색 토큰                   describe('새로 더한 색 토큰이 dark 블록에도 있다')
4  금지된 장식 부재                      describe('금지된 장식이 들어오지 않았다')
5  색만으로 상태를 말하지 않음           describe('상태는 언제나 문장으로 온다 — 색은 거들 뿐이다')
```

숫자 기준선 둘(`HARDCODED_FONT_SIZE_BASELINE = 0` · `HARDCODED_SPACING_BASELINE = 0`)은
파일 상단과 각 상수의 주석에서 **목표가 아니라 회귀 방지선**임을 밝힌다.

픽셀/스크린샷 비교 검사는 더하지 않았다. 그 금지 자체는
`tests/manual-handoff-invariants.test.ts`가 저장소의 모든 테스트에 대해 이미 지키며,
새 파일도 그 검사의 대상이다 (26 파일 전부 통과).
