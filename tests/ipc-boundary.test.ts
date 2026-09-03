// command 경계 테스트.
//
// 검사 대상은 화면 동작이 아니라 **frontend와 Rust 사이의 경계 그 자체**다
// (PRODUCT-SPEC §12 · docs/ADR-0001-local-persistence.md).
//
// 세 가지를 본다:
//   1. src/ 아래에 SQL이나 임의 질의 경로가 없다 — 저장소를 아는 것은 Rust뿐이다.
//   2. 등록된 command 목록과 frontend가 부르는 목록이 정확히 같고, 지금 범위를 넘지 않는다.
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
 * 앞의 여섯은 Phase 1(recording CRUD · settings)이고, 그다음은 녹음 표면이다 —
 * **입력 장치 열거**와 **녹음 session**(시작 · 일시정지 · 재개 · 정지 · 상태 조회),
 * 그리고 **레코드와 파일이 어긋난 상태의 감지**(`list_missing_audio`)다.
 * 나머지 셋이 Phase 3의 전사 표면이다 — **한 건 시작 · 상태 조회 · 저장된 결과 읽기**다.
 * 진행 중인 session도 진행 중인 전사도 소유하는 것은 backend이며, 화면은 이 command로만
 * 그것을 다룬다 (R-001 · docs/ADR-0004-recording-session-lifecycle.md ·
 * src-tauri/src/commands/transcriber.rs).
 *
 * `get_transcript`는 Phase 3의 요구 6이 요구하는 읽기다 — Recording Detail의 Transcript 탭이
 * timestamp와 함께 문장을 보여주려면 저장된 Transcript를 읽는 이름이 있어야 한다. **읽기뿐이며
 * 쓰기 이름은 늘지 않았다**: Transcript는 immutable이고 (§7.1 · INV-2) 저장소가 내놓는 쓰기
 * 경로도 추가 하나뿐이다.
 *
 * 이 목록에 없는 이름이 등록되면 그것은 Phase 범위가 넘쳤다는 뜻이다 — 그래서 이 검사는
 * 부분집합이 아니라 **정확히 같은 집합**을 요구한다.
 */
const REGISTERED_COMMANDS = [
  'list_recordings',
  'get_recording',
  'get_transcript',
  'create_recording',
  'delete_recording',
  'get_settings',
  'update_settings',
  'list_input_devices',
  'start_capture',
  'pause_capture',
  'resume_capture',
  'stop_capture',
  'capture_status',
  'list_missing_audio',
  'start_transcription',
  'transcription_status',
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
    // 녹음 표면은 **capture 계열 네 동작과 상태 조회**까지다. 저장된 녹음의 재생은
    // command로 하지 않는다 — 파일은 asset protocol로 흐르므로 이 표면에 이름이 생기지
    // 않는다 (docs/ADR-0006-audio-playback.md). AI · Notion은 각각 Phase 4 · 5다.
    // 아래 정규식이 그 선을 지킨다 —
    // `*_recording`은 Recording 레코드를 만드는 영속화 표면의 이름이므로,
    // 녹음 동작이 그 이름으로 새로 생기는 것은 여전히 막는다.
    // `queue`·`batch`·`schedule`은 여러 Recording 동시 전사 큐의 이름이며 DEFERRED다 (§16).
    const outOfScope =
      /(start|stop|pause|resume)_recording|play|whisper|\bai_|notion|ollama|export|queue|batch|schedule/i;

    for (const command of registeredCommands()) {
      expect(command, `${command}는 아직 만들지 않은 기능의 command다`).not.toMatch(outOfScope);
    }
  });

  it('전사 표면은 한 건 시작 · 상태 조회 · 결과 읽기 셋뿐이다', () => {
    // 여러 Recording 동시 전사 큐는 이 Phase의 범위 밖이다 (PRODUCT-SPEC §16 DEFERRED ·
    // phase-prompt/03의 Out of Scope). 큐가 생기면 표면에 먼저 드러난다 — 목록을 걸거나,
    // 대기열을 묻거나, 취소하는 이름이 필요해지기 때문이다.
    const transcription = registeredCommands().filter((command) => /transcri/i.test(command));

    expect(transcription.sort()).toEqual([
      'get_transcript',
      'start_transcription',
      'transcription_status',
    ]);
  });

  it('저장된 Transcript를 고치거나 지우는 command가 없다', () => {
    // Transcript는 immutable · versioned다 (§7.1 · INV-2). 재전사는 기존 것을 고치지 않고
    // 새 것을 추가하며, 그 추가를 하는 것은 backend의 전사 경로뿐이다. transcript 편집·삭제
    // UI도 이 Phase의 범위 밖이다 — 그러므로 그 수단이 표면에 있어서도 안 된다.
    const mutating = registeredCommands().filter((command) =>
      /^(update|set|edit|delete|remove|append|save)_.*transcript/i.test(command),
    );

    expect(mutating).toEqual([]);
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
