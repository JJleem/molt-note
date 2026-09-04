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
const typesSource = readText('../src/ipc/types.ts');
const libSource = readText('../src-tauri/src/lib.rs');
const rustFailureSource = readText('../src-tauri/src/domain/failure.rs');
const payloadSource = readText('../src-tauri/src/commands/payload.rs');

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
 * 그다음 다섯이 Phase 4의 AI 노트 표면이다 — **provider 상태 조회 · 생성 시작 · 진행 상태
 * 조회 · 저장된 노트 읽기 둘**이다. 여기서도 쓰기 이름은 늘지 않는다: 노트는 생성으로만 늘고
 * 재생성은 대체가 아니라 추가이므로 (docs/ADR-0008-note-ai-provider.md §9.2), 고치거나 지우는
 * 이름이 만들어질 자리가 없다.
 *
 * 그다음 하나가 Phase 5의 **Markdown export**다 (docs/ADR-0009-notion-and-export.md §4).
 * 저장된 것을 읽어 파일 하나를 더할 뿐이므로 여기서도 쓰기 이름은 늘지 않으며, 이미 있는
 * 파일을 덮어쓰지 않는다 (§4.3).
 *
 * 마지막 여섯이 같은 Phase의 **Notion 전송**이다 — 전송 시작 · 진행 상태 조회 · 저장된 전송
 * 기록 읽기 · 연결 확인 · token 저장 · token 삭제 (§10 · §5-D · ADR-0009 §8 · §10).
 * 여기서도 저장된 것을 고치거나 지우는 이름은 늘지 않는다: 지우는 하나는 이 앱이 자격증명
 * 저장소에 넣은 항목이며, 녹음 · 전사 · 노트 · 이미 만들어진 Notion 페이지를 지우는 이름은
 * 여전히 없다 (INV-3 · INV-4). **token을 돌려주는 이름도 없다** (INV-7).
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
  'ai_provider_status',
  'start_ai_note',
  'ai_note_status',
  'list_ai_notes',
  'get_ai_note',
  'export_markdown',
  'start_notion_sync',
  'notion_sync_status',
  'get_notion_sync',
  'check_notion_connection',
  'save_notion_token',
  'delete_notion_token',
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
    // 않는다 (docs/ADR-0006-audio-playback.md).
    // 아래 정규식이 그 선을 지킨다 —
    // `*_recording`은 Recording 레코드를 만드는 영속화 표면의 이름이므로,
    // 녹음 동작이 그 이름으로 새로 생기는 것은 여전히 막는다.
    // `queue`·`batch`·`schedule`은 여러 Recording 일괄 처리 큐의 이름이며 DEFERRED다 (§16).
    //
    // **export는 `export_markdown` 하나만 열렸다.** Phase 5가 실제로 만든 것이 그 하나이며
    // (docs/ADR-0009-notion-and-export.md §4), PDF·DOCX 같은 다른 포맷과 일괄 export는 여전히
    // 범위 밖이다 (phase-prompt/05의 Out of Scope).
    //
    // **`notion`은 이제 여기 없다** — 전송 표면이 실제로 열렸기 때문이다 (§10). 그 대신 아래의
    // 'Notion 표면은 여섯뿐이다'가 그 선을 지킨다: 이름이 하나 늘면 그쪽에서 먼저 드러난다.
    //
    // **벤더 이름은 여전히 여기 있다** — AI 표면이 열린 뒤에도 command 이름은 벤더 중립이어야
    // 한다 (INV-9). 어떤 provider를 쓰는지는 설정 값이고, 그것을 아는 코드는 adapter 안에만
    // 있다. `ollama_*` 같은 이름이 등록되면 그 경계가 새어 나온 것이다. Notion은 그와 다르다 —
    // 제품이 보내기로 한 목적지 그 자체이며 (PRODUCT-SPEC §10), 고를 수 있는 provider가 아니다.
    const outOfScope =
      /(start|stop|pause|resume)_recording|play|whisper|ollama|llama|openai|anthropic|claude|gemini|export(?!_markdown\b)|pdf|docx|queue|batch|schedule/i;

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

  it('AI 표면은 provider 상태 · 생성 시작 · 진행 상태 · 저장된 노트 읽기 둘뿐이다', () => {
    // 여러 Recording 일괄 AI 처리 큐도, 프롬프트 편집도 이 Phase의 범위 밖이다
    // (§16 DEFERRED · phase-prompt/04의 Out of Scope). 그것이 생기면 표면에 먼저 드러난다.
    const ai = registeredCommands().filter((command) => /ai_note|ai_provider/i.test(command));

    expect(ai.sort()).toEqual([
      'ai_note_status',
      'ai_provider_status',
      'get_ai_note',
      'list_ai_notes',
      'start_ai_note',
    ]);
  });

  it('저장된 AI 노트를 고치거나 지우는 command가 없다', () => {
    // 재생성은 기존 노트를 대체하지 않고 하나를 더 남긴다 (ADR-0008 §9.2). 저장소가 내놓는
    // `ai_notes` 쓰기도 추가 하나뿐이므로, 고치거나 지우는 수단이 표면에 있어서도 안 된다.
    const mutating = registeredCommands().filter((command) =>
      /^(update|set|edit|delete|remove|save)_.*(ai_note|note)/i.test(command),
    );

    expect(mutating).toEqual([]);
  });

  it('Markdown export 표면은 파일 하나를 만드는 이름 하나뿐이다', () => {
    // 일괄 export도, export한 것을 지우거나 다시 쓰는 이름도 이 Phase의 범위 밖이다
    // (phase-prompt/05의 Out of Scope · §16 DEFERRED). 그것이 생기면 표면에 먼저 드러난다.
    const exports = registeredCommands().filter((command) => /export/i.test(command));

    expect(exports).toEqual(['export_markdown']);
  });

  it('Notion 표면은 전송 둘 · 저장된 기록 읽기 · 연결 확인 · token 둘뿐이다', () => {
    // 여러 Recording 일괄 sync도, 보낸 페이지를 지우거나 다시 쓰는 이름도 이 Phase의 범위
    // 밖이다 (docs/ADR-0009-notion-and-export.md §13 · §16 DEFERRED). 그것이 생기면 표면에
    // 먼저 드러난다.
    const notion = registeredCommands().filter((command) => /notion/i.test(command));

    expect(notion.sort()).toEqual([
      'check_notion_connection',
      'delete_notion_token',
      'get_notion_sync',
      'notion_sync_status',
      'save_notion_token',
      'start_notion_sync',
    ]);
  });

  it('저장된 Notion 전송 기록을 고치거나 지우는 command가 없다', () => {
    // `notion_syncs`에 쓰는 자리는 전송 순서 하나뿐이다 (§8.4). 화면에서 시작한 어떤 동작도
    // 이미 남은 전송 기록을 고치거나 지우지 못한다 (INV-3).
    const mutating = registeredCommands().filter((command) =>
      /^(update|set|edit|delete|remove|save)_.*sync/i.test(command),
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

describe('wire 계약에 벤더가 없다 (INV-9)', () => {
  // 벤더는 바뀐다. 바뀔 때 흔들리는 것이 adapter 하나여야 하며, 그러려면 화면과 backend가
  // 주고받는 값의 모양에 특정 제공자의 이름·주소·에러 코드가 없어야 한다
  // (docs/ADR-0008-note-ai-provider.md §1 · phase-prompt/04 Verification Boundary).
  const VENDORS = /ollama|llama|openai|gpt-|anthropic|claude|gemini|groq|mistral|huggingface/i;

  it('command payload 타입에 벤더 고유 이름이 없다', () => {
    expect(payloadSource).not.toMatch(VENDORS);
  });

  it('frontend 타입에 벤더 고유 이름이 없다', () => {
    expect(typesSource).not.toMatch(VENDORS);
  });

  it('frontend 타입에 AI 서버로 가는 주소나 엔드포인트가 없다', () => {
    // 호출은 Rust backend에서 나간다 (ADR-0008 §5). webview에는 AI 서버로 가는 통로가 없고,
    // 주소를 아는 자리도 backend 하나다 — 주소가 프론트 타입에 적히는 순간 그 사실이 깨진다.
    for (const source of [typesSource, commandsSource]) {
      expect(source).not.toMatch(/https?:\/\/(localhost|127\.0\.0\.1|\d)/i);
      expect(source).not.toMatch(/\/api\/(tags|generate|chat)/i);
    }
  });

  it('provider가 로컬인지 외부인지가 frontend까지 도달한다 (INV-5)', () => {
    // 사용자는 전사가 기기를 떠나는지 알 수 있어야 한다 (§12 · 요구 15). 그 값이 타입에
    // 없으면 화면은 그것을 보여줄 방법이 없다.
    expect(typesSource).toMatch(/AiProviderLocality\s*=\s*'local'\s*\|\s*'external'/);
    expect(typesSource).toMatch(/locality:\s*AiProviderLocality\s*\|\s*null/);
    // Rust 쪽도 같은 두 값을 그대로 실어 보낸다.
    expect(payloadSource).toContain('pub locality: Option<String>');
  });
});

describe('자격증명은 이 경계를 한 방향으로만 지난다 (INV-7)', () => {
  // integration token은 저장하는 command의 **입력**으로 한 번 지나갈 뿐이다
  // (docs/ADR-0009-notion-and-export.md §10.4). 돌아오는 길이 하나라도 있으면 그 값은 화면
  // 상태 · 로그 · devtools로 새어 나갈 수 있다.

  it('token 값을 담는 payload 필드가 Rust 쪽에 없다', () => {
    // 있는 것은 `tokenStored: bool`처럼 **사실**을 말하는 필드뿐이다 — 값을 담는 자리가 없다.
    expect(payloadSource).not.toMatch(/pub\s+\w*token\w*\s*:\s*(Option\s*<\s*)?String/i);
    expect(payloadSource).toContain('pub token_stored: bool');
  });

  it('token 값을 담는 필드가 frontend 타입에도 없다', () => {
    expect(typesSource).not.toMatch(/readonly\s+\w*token\w*\s*:\s*string/i);
    expect(typesSource).toMatch(/readonly tokenStored: boolean/);
    expect(typesSource).toMatch(/readonly stored: boolean/);
  });

  it('token을 돌려받는 command가 없다', () => {
    // 함수 시그니처에서 token이 나타날 수 있는 자리는 **인자 하나뿐**이다.
    const signatures = [...commandsSource.matchAll(/export function ([\s\S]*?)\{/g)].map(
      (matched) => matched[1],
    );
    const carryingToken = signatures.filter((signature) => /token/i.test(signature));

    expect(carryingToken.length).toBeGreaterThan(0);
    for (const signature of carryingToken) {
      const returns = signature.split('):').at(-1) ?? '';
      expect(returns, `token을 돌려주는 command가 있다: ${signature}`).not.toMatch(/token(?!Status)/i);
    }
  });
});

describe('frontend는 Notion으로 직접 나가지 않는다', () => {
  it('src/ 아래에 네트워크로 나가는 통로가 없다', () => {
    // 요청을 만드는 자리는 Rust의 adapter 하나이며 (ADR-0009 §5 · INV-9의 태도), webview에는
    // 임의의 요청을 만들 수단이 없다. 있으면 자격증명도 문서도 이 경계 밖으로 나갈 수 있다.
    const outbound = [/\bfetch\s*\(/, /XMLHttpRequest/, /\bWebSocket\b/, /EventSource/];

    for (const file of frontendSources) {
      const source = readFileSync(file, 'utf8');
      for (const shape of outbound) {
        expect(source, `${file}이 직접 네트워크로 나간다`).not.toMatch(shape);
      }
    }
  });

  it('src/ 아래에 Notion API 지식이 없다', () => {
    // 주소 · 헤더 이름 · API 버전 · 오류 코드는 adapter 디렉터리 밖으로 나가지 않는다.
    // Rust 쪽의 같은 검사는 `src-tauri/tests/notion_adapter.rs`에 있다.
    const vendorKnowledge = [/notion\.com/i, /Notion-Version/i, /Bearer\b/, /\/v1\//];

    for (const file of frontendSources) {
      const source = readFileSync(file, 'utf8');
      for (const shape of vendorKnowledge) {
        expect(source, `${file}에 Notion API 지식이 있다`).not.toMatch(shape);
      }
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

  it('frontend 타입에는 Rust가 만들지 않는 종류가 없다', () => {
    // 반대 방향도 막는다 — 화면에만 있는 종류는 어느 실패로도 도착하지 않으므로, 있으면
    // 그것은 오타이거나 사라진 실패의 흔적이다. 둘 다 화면이 잘못된 안내를 하게 만든다.
    //
    // `unexpected` 하나만 예외다. 그것은 **frontend 경계에서만 만들어진다** (`toFailure`).
    const kinds = new Set(
      [...rustFailureSource.matchAll(/Self::\w+\s*=>\s*"(\w+)"/g)].map((matched) => matched[1]),
    );

    const union = failureTypeSource.match(/export type FailureKind =([^;]+);/)?.[1] ?? '';
    const declared = [...union.matchAll(/'(\w+)'/g)].map((matched) => matched[1]);
    expect(declared.length).toBeGreaterThan(0);

    for (const kind of declared) {
      if (kind === 'unexpected') {
        continue;
      }
      expect(kinds.has(kind), `Rust에 '${kind}'가 없다`).toBe(true);
    }
    expect(declared.length, '두 목록의 크기가 다르다').toBe(kinds.size + 1);
  });
});
