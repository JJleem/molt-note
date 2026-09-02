// command 경계 테스트.
//
// 검사 대상은 화면 동작이 아니라 **frontend와 Rust 사이의 경계 그 자체**다
// (PRODUCT-SPEC §12 · docs/ADR-0001-local-persistence.md).
//
// 세 가지를 본다:
//   1. src/ 아래에 SQL이나 임의 질의 경로가 없다 — 저장소를 아는 것은 Rust뿐이다.
//   2. 등록된 command 목록과 frontend가 부르는 목록이 정확히 같고, Phase 1 범위를 넘지 않는다.
//   3. Rust의 실패 종류가 frontend 타입에 전부 있다 — 실패가 조용히 어긋나지 않는다.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const path = (relative: string) => fileURLToPath(new URL(relative, import.meta.url));
const readText = (relative: string) => readFileSync(path(relative), 'utf8');

/** src/ 아래의 모든 소스 파일 경로. */
function sourceFiles(directory: string): string[] {
  return readdirSync(directory).flatMap((entry) => {
    const full = `${directory}/${entry}`;
    if (statSync(full).isDirectory()) {
      return sourceFiles(full);
    }
    return /\.(ts|tsx|js|jsx|css|html)$/.test(entry) ? [full] : [];
  });
}

const frontendSources = sourceFiles(path('../src'));
const commandsSource = readText('../src/ipc/commands.ts');
const failureTypeSource = readText('../src/ipc/failure.ts');
const libSource = readText('../src-tauri/src/lib.rs');
const rustFailureSource = readText('../src-tauri/src/domain/failure.rs');

/**
 * 지금까지 노출하기로 한 command 전부.
 *
 * 앞의 여섯은 Phase 1(recording CRUD · settings)이고, 나머지 셋은 Phase 2A가 engine 검증을
 * 위해 더한 spike 표면이다 — **입력 장치 열거**와 **캡처 시작 / 정지 한 쌍**
 * (docs/ADR-0003-recording-engine.md §12).
 *
 * 이 목록에 없는 이름이 등록되면 그것은 Phase 범위가 넘쳤다는 뜻이다 — 그래서 이 검사는
 * 부분집합이 아니라 **정확히 같은 집합**을 요구한다.
 */
const REGISTERED_COMMANDS = [
  'list_recordings',
  'get_recording',
  'create_recording',
  'delete_recording',
  'get_settings',
  'update_settings',
  'list_input_devices',
  'start_capture',
  'stop_capture',
];

/** lib.rs의 generate_handler![...]에 등록된 command 이름. */
function registeredCommands(): string[] {
  const handler = libSource.match(/generate_handler!\[([\s\S]*?)\]/);
  expect(handler, 'lib.rs가 command를 등록해야 한다').not.toBeNull();
  return [...(handler?.[1] ?? '').matchAll(/commands::(\w+)/g)].map((matched) => matched[1]);
}

describe('frontend는 저장소를 알지 않는다', () => {
  it('src/ 아래에 SQL 문장이 없다', () => {
    // 문자열 하나가 아니라 질의의 모양을 찾는다 — updateSettings 같은 이름에 걸리지 않게 한다.
    const sqlShapes = [
      /\bselect\s+[\w*][\s\S]{0,80}?\bfrom\b/i,
      /\binsert\s+into\b/i,
      /\bdelete\s+from\b/i,
      /\bupdate\s+\w+\s+set\b/i,
      /\bcreate\s+(table|index)\b/i,
      /\bdrop\s+(table|index)\b/i,
      /\bpragma\s+\w+/i,
    ];

    for (const file of frontendSources) {
      const source = readFileSync(file, 'utf8');
      for (const shape of sqlShapes) {
        expect(source, `${file}에 SQL이 있다`).not.toMatch(shape);
      }
    }
  });

  it('src/ 아래에서 command를 부르는 곳은 ipc 모듈뿐이다', () => {
    // 화면이 직접 invoke를 부르면 경계가 여러 곳으로 흩어진다.
    const callers = frontendSources.filter((file) => /\binvoke\s*[<(]/.test(readFileSync(file, 'utf8')));

    expect(callers).toEqual([path('../src/ipc/commands.ts')]);
  });
});

describe('command 표면', () => {
  it('허용된 목록과 정확히 같은 command만 등록되어 있다', () => {
    expect(registeredCommands().sort()).toEqual([...REGISTERED_COMMANDS].sort());
  });

  it('아직 만들지 않은 기능의 command가 등록되어 있지 않다', () => {
    // Phase 2A가 만드는 것은 **start / stop 한 쌍의 spike 캡처**까지다.
    // pause/resume · 재생 · Recording 영속화는 Phase 2B이고, 전사 · AI · Notion은
    // 각각 Phase 3 · 4 · 5다. 아래 정규식이 그 선을 지킨다 — `*_recording`은 Recording
    // 레코드를 만드는 영속화 표면의 이름이므로 여전히 막혀 있다.
    const outOfScope =
      /(start|stop|pause|resume)_recording|(pause|resume)_capture|play|transcri|whisper|\bai_|notion|ollama|export/i;

    for (const command of registeredCommands()) {
      expect(command, `${command}는 Phase 1의 command가 아니다`).not.toMatch(outOfScope);
    }
  });

  it('frontend가 부르는 이름이 등록된 이름과 정확히 같다', () => {
    const called = [...commandsSource.matchAll(/call<[^>]*>\(\s*'([\w]+)'/g)].map(
      (matched) => matched[1],
    );

    expect(called.sort()).toEqual(registeredCommands().sort());
  });

  it('scaffold 예제 command가 남아 있지 않다', () => {
    expect(libSource).not.toContain('greet');
    for (const file of frontendSources) {
      expect(readFileSync(file, 'utf8'), `${file}에 예제 잔재가 있다`).not.toContain('greet');
    }
  });
});

describe('실패 타입', () => {
  it('Rust의 실패 종류가 frontend 타입에 전부 있다', () => {
    // Rust가 새 종류를 추가했는데 화면이 모르면, 실패가 조용히 다른 모양으로 도착한다.
    const kinds = [...rustFailureSource.matchAll(/Self::\w+\s*=>\s*"(\w+)"/g)].map(
      (matched) => matched[1],
    );
    expect(kinds.length).toBeGreaterThan(0);

    const union = failureTypeSource.match(/export type FailureKind =([^;]+);/)?.[1] ?? '';
    for (const kind of kinds) {
      expect(union, `FailureKind에 '${kind}'가 없다`).toContain(`'${kind}'`);
    }
  });
});
