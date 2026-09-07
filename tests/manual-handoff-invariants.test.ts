// **Manual AI Handoff의 여덟 불변 — 화면 쪽 원문 검사**
// (`phase-prompt/05.5` 성공 기준 · docs/ADR-0010-manual-ai-handoff.md · PRODUCT-SPEC §13 · §18).
//
// 값 수준 판정은 두 자리에 있다 — 실제 저장소와 파일시스템을 지나는 쪽은
// `src-tauri/tests/manual_handoff_invariants.rs`가, 화면 값의 갈래는 각 모듈 옆의 테스트가
// 본다 (`src/screens/copyView.test.ts` · `src/screens/aiHandoffView.test.ts` ·
// `src/platform/clipboard.test.ts`). 여기서 보는 것은 **원문 전체에 대한 규칙**이며, 파일 하나가
// 새로 생기거나 한 줄이 늘어나는 것만으로 조용히 깨질 수 있는 것들이다.
//
//   MH-1 · MH-2  이 경로의 어느 자리도 AI provider를 읽지 않는다 — 담을 자리 자체가 없다
//   MH-3         이 경로에 나가는 통로도 주소도 없다
//   MH-4         산출물의 타입에도 이 경로의 코드에도 오디오가 없다
//   MH-5         wire에도 화면에도 옛 Transcript version을 고를 수단이 없다
//   MH-6         frontend 타입과 화면 문장에 벤더 이름도 벤더 schema도 없다
//   MH-7         복사와 export의 실패 경로에 저장된 것을 바꾸는 호출이 없다
//   MH-8         기존 Connected Provider 경로가 그 자리에 그대로 있다
//   그리고        이 Phase의 자동 테스트가 실제 자원을 세우지 않고, 픽셀 비교 테스트가 없다
//
// ## 이미 있는 검사를 다시 쓰지 않는다
//
// 아래는 여기서 다시 쓰지 않고 그 자리를 가리킨다. 같은 사실을 두 번 적으면 둘이 어긋날 때
// 어느 쪽이 규칙인지 알 수 없게 된다.
//
//   command 표면이 정확히 서른하나이고 셋이 늘었다      tests/ipc-boundary.test.ts
//   src/ 전체에 네트워크로 나가는 통로가 없다 (§12)     tests/ipc-boundary.test.ts
//   clipboard를 부르는 자리가 하나다 (R-4 · INV-10)     tests/screen-boundary.test.ts
//   자동 테스트가 실제 시스템 clipboard를 건드리지 않는다 tests/screen-boundary.test.ts
//   실패가 console에만 남지 않는다 (§13)                tests/screen-boundary.test.ts
//
// 이 파일은 DOM도 Tauri도 clipboard도 네트워크도 부르지 않는다 — 파일을 읽고 문자열을 볼 뿐이다.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const path = (relative: string) => fileURLToPath(new URL(relative, import.meta.url));

function sourceFiles(directory: string, pattern: RegExp): string[] {
  return readdirSync(directory).flatMap((entry) => {
    const full = `${directory}/${entry}`;
    if (statSync(full).isDirectory()) {
      return sourceFiles(full, pattern);
    }
    return pattern.test(entry) ? [full] : [];
  });
}

/**
 * 실행되는 코드만 남긴다 — 규칙을 설명하는 주석이 규칙 위반으로 잡히지 않게 한다.
 *
 * 이 저장소의 프론트엔드 주석은 `//`와 `/** ... *\/` 둘 다 쓴다.
 */
function code(source: string): string {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .split('\n')
    .filter((line) => !line.trim().startsWith('//'))
    .join('\n');
}

/**
 * 선언 한 줄에서 시작해 그 블록이 닫힐 때까지의 원문.
 *
 * 파일 전체가 아니라 **문제의 자리만** 본다 — `commands.ts`도 `RecordingDetailScreen.tsx`도 이
 * Phase와 상관없는 코드를 담고 있으므로, 파일 단위로 금지어를 찾으면 검사가 아무 뜻도 없게 된다.
 */
function block(source: string, header: string, close: string): string {
  const start = source.indexOf(header);
  expect(start, `${header}가 소스에 있어야 한다`).toBeGreaterThanOrEqual(0);
  const rest = source.slice(start);
  const end = rest.indexOf(close);
  expect(end, `${header}의 블록이 닫혀야 한다`).toBeGreaterThanOrEqual(0);
  return rest.slice(0, end + close.length);
}

/** `readonly name:` 꼴로 선언된 필드 이름 전부. 타입이 **무엇을 담을 수 있는가**다. */
function fields(declaration: string): string[] {
  return [...declaration.matchAll(/readonly\s+(\w+)/g)].map((matched) => matched[1]);
}

const read = (relative: string) => readFileSync(path(relative), 'utf8');

const frontendSources = sourceFiles(path('../src'), /\.(ts|tsx)$/);
const productSources = frontendSources.filter((file) => !/\.test\.tsx?$/.test(file));

const commandsSource = code(read('../src/ipc/commands.ts'));
const typesSource = code(read('../src/ipc/types.ts'));
const detailScreenSource = code(read('../src/screens/RecordingDetailScreen.tsx'));
const copyViewSource = code(read('../src/screens/copyView.ts'));
const aiHandoffViewSource = code(read('../src/screens/aiHandoffView.ts'));

/**
 * Manual AI Handoff가 화면에서 실제로 지나는 자리 전부.
 *
 * ```text
 * 순수 모듈 셋      복사 상태 · 두 줄의 위계와 Export for AI 자리 · clipboard 경계
 * command wrapper 셋 프롬프트 · 전사 텍스트 · AI-ready 문서
 * 호출하는 자리 둘   화면 컴포넌트가 실제로 거는 복사와 내보내기
 * ```
 */
const MANUAL_PATH: readonly (readonly [string, string])[] = [
  ['copyView.ts', copyViewSource],
  ['aiHandoffView.ts', aiHandoffViewSource],
  ['platform/clipboard.ts', code(read('../src/platform/clipboard.ts'))],
  ['getAiPrompt', block(commandsSource, 'export function getAiPrompt(', '\n}\n')],
  ['getTranscriptText', block(commandsSource, 'export function getTranscriptText(', '\n}\n')],
  ['exportAiRequest', block(commandsSource, 'export function exportAiRequest(', '\n}\n')],
  ['beginCopy', block(detailScreenSource, '  const beginCopy = ', '\n  };\n')],
  ['beginAiExport', block(detailScreenSource, '  const beginAiExport = ', '\n  };\n')],
];

/**
 * 산출물에도 타입에도 나오면 안 되는 AI 채팅 벤더의 이름 (MH-6 · §10).
 *
 * **Notion은 여기 없다** — 고를 수 있는 provider가 아니라 이 제품이 보내기로 한 목적지 그
 * 자체이기 때문이다 (PRODUCT-SPEC §10 · tests/ipc-boundary.test.ts의 같은 구분).
 */
const VENDOR_NAMES = /chatgpt|openai|anthropic|\bclaude\b|gemini|copilot|codex|ollama|llama|mistral/i;

/** 어느 벤더의 API를 아는지 드러내는 문자열 (MH-6 · INV-9). */
const VENDOR_SCHEMA = ['/api/generate', '/api/tags', 'num_ctx', 'OLLAMA_HOST', 'rich_text'];

// --- 원문 검사가 실제로 그 코드를 읽고 있는가 -------------------------------------------

describe('이 파일의 검사가 실제 코드를 읽는다', () => {
  it('경로의 여덟 자리를 전부 읽었고, 그 안에 실제 코드가 있다', () => {
    // **아무것도 읽지 못한 검사는 언제나 통과한다.** 슬라이스가 빗나가거나 주석 제거가 너무
    // 많이 지우면 아래의 모든 금지어 검사가 조용히 무의미해진다.
    expect(MANUAL_PATH).toHaveLength(8);

    const markers: Record<string, string> = {
      'copyView.ts': 'export function copyPanel(',
      'aiHandoffView.ts': 'export function manualHandoff(',
      'platform/clipboard.ts': 'export async function copyText(',
      getAiPrompt: "'get_ai_prompt'",
      getTranscriptText: "'get_transcript_text'",
      exportAiRequest: "'export_ai_request'",
      // 복사는 돌아온 값의 **텍스트**를 clipboard에 올린다. 그 값에는 크기와 조각의 자리도
      // 함께 실려 있다 (`phase-prompt/05.6` 성공 기준 4).
      beginCopy: 'copyText(value.text)',
      beginAiExport: 'exportAiRequest(id, mode, portion)',
    };

    for (const [name, source] of MANUAL_PATH) {
      expect(source, `${name}을 읽지 못했다`).toContain(markers[name]);
    }
  });

  it('주석 제거가 실제로 일어난다', () => {
    // 아래 줄은 "오디오는 절대 포함되지 않는다"고 **문장으로** 말한다. 그 문장은 남고,
    // 규칙을 설명하는 주석은 사라진다 — 그 차이가 MH-4의 소스 검사가 진짜 검사라는 증거다.
    const raw = read('../src/screens/aiHandoffView.ts');

    expect(raw).toContain('INV-6');
    expect(code(raw)).not.toContain('INV-6');
    expect(code(raw)).toContain('The audio file is never included.');
  });
});

// --- MH-1 · MH-2 — provider가 하나도 없어도 셋이 전부 동작한다 --------------------------

describe('MH-1 · MH-2 — 이 경로는 AI provider를 읽을 수단이 없다', () => {
  it('복사와 export가 지나는 어느 자리도 provider 상태나 AI 설정을 읽지 않는다', () => {
    // "provider가 없어서 복사할 수 없다"는 상태를 만들 수단 자체가 없어야 한다. 한 줄만
    // 들어오면 그 상태는 언제든 만들어질 수 있으므로, 값이 아니라 **읽을 수단**을 본다.
    //
    // `provider`라는 낱말 자체는 막지 않는다 — 아래 줄에는 "provider가 없어도 된다"고 말하는
    // 문장이 있어야 하기 때문이다 (MANUAL_NO_PROVIDER_NOTICE). 막는 것은 **provider 상태와 AI
    // 설정을 읽는 이름**이다.
    const readsProvider = [
      'AiProviderStatus',
      'aiProviderStatus',
      'AiConnection',
      'checkedAiProvider',
      'aiProvider',
      'aiBaseUrl',
      'aiModel',
      'getSettings',
      'updateSettings',
    ];

    for (const [name, source] of MANUAL_PATH) {
      for (const forbidden of readsProvider) {
        expect(source, `${name}이 ${forbidden}를 읽는다`).not.toContain(forbidden);
      }
      expect(source, `${name}에 벤더 이름이 있다`).not.toMatch(VENDOR_NAMES);
    }
  });

  it('아래 줄과 복사 자리의 입력 타입에 provider를 담을 자리가 없다', () => {
    // 담을 자리가 없으면 실수로도 실릴 수 없다. 필드 목록을 **정확히** 고정하는 이유는
    // 하나가 늘어나는 순간 그것이 결정이라는 사실을 드러내기 위해서다.
    //
    // **필드가 하나 늘었다 — `show`다** (`phase-prompt/05.6` 성공 기준 2 · R-4). 그것이
    // 결정이라는 사실이 여기서 드러나는 것이 이 검사의 목적이므로, 무엇이 왜 늘었는지 적는다:
    // 이 값은 **이미 만들어진 파일 하나가 놓인 자리를 여는 시도**이며 담고 있는 것은 파일
    // 경로다. AI provider도, AI 설정도, 벤더도 아니다 — 그것을 여기에 담을 자리는 여전히 없고,
    // 그래서 provider 때문에 아래 줄이 막히는 상태를 만들 수단도 여전히 없다 (MH-1 · MH-2).
    expect(fields(block(aiHandoffViewSource, 'export interface ManualHandoffInput {', '\n}\n')))
      .toEqual(['recording', 'mode', 'copy', 'aiExport', 'show']);

    expect(fields(block(copyViewSource, 'export interface CopyPanelInput {', '\n}\n')))
      .toEqual(['recording', 'mode', 'attempt']);
  });

  it('세 command가 recordingId · mode · 가져갈 조각 말고는 아무것도 보내지 않는다', () => {
    // wire에 실리는 것이 전부다 — provider도, 주소도, 모델도 실을 자리가 없다.
    //
    // **인자가 하나 늘었다 — `portion`이다** (`phase-prompt/05.6` 성공 기준 4). 그것이
    // 결정이라는 사실이 여기서 드러나는 것이 이 검사의 목적이므로, 무엇이 왜 늘었는지 적는다:
    // 이 값은 **산출물의 몇 번째 조각을 가져갈 것인가**이며 담고 있는 것은 1부터 세는 번호
    // 하나다. AI provider도, 벤더도, 옛 Transcript version을 고르는 수단도 아니다 — 그런 것을
    // 여기 담을 자리는 여전히 없고, 그래서 provider 때문에 이 셋이 막히는 상태를 만들 수단도
    // 여전히 없다 (MH-1 · MH-2 · MH-5).
    //
    // **이름은 늘지 않았다** (ADR-0010 §8.1). 늘어난 것은 인자와 응답의 모양뿐이며,
    // `tests/ipc-boundary.test.ts`가 세는 command 표면은 그대로 서른둘이다.
    expect(block(commandsSource, 'export function getAiPrompt(', '\n}\n')).toContain(
      "call<HandoffText>('get_ai_prompt', { recordingId, mode, portion })",
    );
    expect(block(commandsSource, 'export function getTranscriptText(', '\n}\n')).toContain(
      "call<HandoffText>('get_transcript_text', { recordingId, portion })",
    );
    expect(block(commandsSource, 'export function exportAiRequest(', '\n}\n')).toContain(
      "call<ExportedAiRequest>('export_ai_request', { recordingId, mode, portion })",
    );
  });
});

// --- MH-3 — 나가는 행위의 주체는 사람이다 -----------------------------------------------

describe('MH-3 — 복사와 export는 이 기기 안에서 끝난다', () => {
  it('이 경로에 나가는 통로도 주소도 없다', () => {
    // `src/` 전체에 나가는 통로가 없다는 것은 tests/ipc-boundary.test.ts가 본다. 여기서
    // 더하는 것은 **주소 하나조차 이 경로에 없다**는 것이다 — 통로가 열리는 날에도 이 경로만은
    // 그대로여야 하기 때문이다.
    const outbound = [
      /\bfetch\s*\(/,
      /XMLHttpRequest/,
      /\bWebSocket\b/,
      /EventSource/,
      /sendBeacon/,
      /https?:\/\//,
      /\blocalhost\b/,
      /\b127\.0\.0\.1\b/,
    ];

    for (const [name, source] of MANUAL_PATH) {
      for (const shape of outbound) {
        expect(source, `${name}이 네트워크로 나가거나 주소를 안다`).not.toMatch(shape);
      }
    }
  });

  it('앱이 아무 데도 보내지 않는다는 사실이 화면 값으로 있다', () => {
    // 사용자에게도 말한다 — 이 사실은 코드에만 있는 것이 아니라 아래 줄에 언제나 붙어 있다.
    expect(aiHandoffViewSource).toContain('MANUAL_LOCAL_NOTICE');
    expect(aiHandoffViewSource).toContain('localNotice: MANUAL_LOCAL_NOTICE');
  });
});

// --- MH-4 — audio는 이 경로에 들어올 자리가 없다 -----------------------------------------

describe('MH-4 — 사람이 가져가는 것은 텍스트뿐이다', () => {
  it('돌아오는 값의 타입에 오디오를 담을 자리가 없다', () => {
    expect(fields(block(typesSource, 'export interface ExportedFile {', '\n}\n'))).toEqual([
      'recordingId',
      'path',
      'fileName',
    ]);
  });

  it('이 경로의 어느 자리도 오디오 파일을 읽거나 실어 나르지 않는다', () => {
    // `audio`라는 낱말 자체는 막지 않는다 — 아래 줄에는 "오디오는 절대 포함되지 않는다"고
    // 말하는 문장이 있어야 하기 때문이다 (MANUAL_LOCAL_NOTICE). 막는 것은 **오디오를 가리키는
    // 값**이다.
    for (const [name, source] of MANUAL_PATH) {
      for (const forbidden of ['audioPath', 'audioFormat', 'convertFileSrc', '.wav']) {
        expect(source, `${name}이 오디오에 닿는다: ${forbidden}`).not.toContain(forbidden);
      }
    }
  });
});

// --- MH-5 — current가 가리키는 Transcript만 쓴다 -----------------------------------------

describe('MH-5 — 옛 version을 고를 수단이 없다', () => {
  it('세 wrapper 어디에도 transcriptId가 실리지 않는다', () => {
    for (const name of ['getAiPrompt', 'getTranscriptText', 'exportAiRequest']) {
      const wrapper = block(commandsSource, `export function ${name}(`, '\n}\n');

      expect(wrapper, `${name}가 recordingId를 보내지 않는다`).toContain('recordingId');
      expect(wrapper, `${name}가 transcriptId를 보낸다 (MH-5)`).not.toMatch(/\btranscriptId\b/);
    }
  });

  it('복사와 export 자리가 보는 것은 current 포인터 하나다', () => {
    // 화면이 version 목록에서 무엇을 고르기 시작하면 §7.2가 두 벌이 된다.
    for (const source of [copyViewSource, aiHandoffViewSource]) {
      expect(source).toContain('currentTranscriptId');
      expect(source).not.toMatch(/\btranscriptId\b/);
      expect(source).not.toContain('listTranscripts');
      expect(source).not.toContain('getTranscript(');
    }
  });
});

// --- MH-6 — 벤더를 알지 않는다 -----------------------------------------------------------

describe('MH-6 — frontend 타입도 화면 문장도 벤더를 알지 않는다', () => {
  it('벤더 이름을 글자로 아는 제품 파일이 provider를 고르는 자리 하나뿐이다', () => {
    // 벤더 이름은 화면에 **값으로** 온다 (`AiProviderStatus.providerName`). 예외는 하나다 —
    // 사용자가 provider를 **고르는** 목록이며 (`SELECTABLE_AI_PROVIDERS`), 고를 수 있는 것의
    // 이름은 고르기 전에도 화면에 있어야 한다. Rust 쪽에서 adapter 디렉터리와 그것을 마운트하는
    // 한 줄만 벤더를 아는 것과 같은 예외다 (INV-9 · src-tauri/tests/ollama_adapter.rs).
    //
    // 그 예외를 **파일 하나로 못박는다.** 목록이 늘어나는 것은 결정이며, 그 결정 없이 조용히
    // 늘어나면 여기서 먼저 드러난다 — 특히 이 Phase가 만든 Manual Handoff 자리는 벤더를 알지
    // 않아야 하고 (MH-6), 그것이 이 검사가 지키는 선이다.
    const PROVIDER_CHOOSER = path('../src/screens/aiProviderSettings.ts');

    const knowsAVendor = productSources.filter((file) =>
      VENDOR_NAMES.test(code(readFileSync(file, 'utf8'))),
    );

    expect(knowsAVendor).toEqual([PROVIDER_CHOOSER]);
    // 그 하나도 **고르는 목록**에서만 이름을 안다 — 노트도, 복사도, export도 아니다.
    const chooser = code(readFileSync(PROVIDER_CHOOSER, 'utf8'));
    for (const forbidden of ['getAiPrompt', 'getTranscriptText', 'exportAiRequest', 'copyText']) {
      expect(chooser, `provider 목록이 ${forbidden}를 안다`).not.toContain(forbidden);
    }
  });

  it('IPC 타입과 이 경로의 코드에 벤더 고유 schema가 없다', () => {
    const surfaces: readonly (readonly [string, string])[] = [
      ['ipc/types.ts', typesSource],
      ['ipc/commands.ts', commandsSource],
      ...MANUAL_PATH,
    ];

    for (const [name, source] of surfaces) {
      for (const schema of VENDOR_SCHEMA) {
        expect(source, `${name}이 벤더 schema를 안다: ${schema}`).not.toContain(schema);
      }
    }
  });
});

// --- MH-7 — 실패해도 잃는 것이 없다 ------------------------------------------------------

describe('MH-7 — 복사와 export의 실패 경로가 저장된 것을 건드리지 않는다', () => {
  it('두 순수 모듈은 command를 부를 수단 자체가 없다', () => {
    // 상태를 판정하는 자리가 저장소에 닿지 않으므로, 복사가 실패했을 때 "정리"하는 경로가
    // 만들어질 자리가 없다.
    for (const source of [copyViewSource, aiHandoffViewSource]) {
      expect(source).not.toContain("from '../ipc/commands'");
      expect(source).not.toContain('invoke');
    }
  });

  it('실제로 거는 두 자리가 읽기와 파일 하나 더하기 말고는 아무것도 하지 않는다', () => {
    const mutating = [
      'deleteRecording',
      'createRecording',
      'updateSettings',
      'startAiNote',
      'startTranscription',
      'startNotionSync',
      'saveNotionToken',
      'deleteNotionToken',
      'exportMarkdown',
    ];

    for (const name of ['beginCopy', 'beginAiExport']) {
      const site = block(detailScreenSource, `  const ${name} = `, '\n  };\n');

      for (const forbidden of mutating) {
        expect(site, `${name}이 ${forbidden}를 부른다`).not.toContain(forbidden);
      }
    }
  });

  it('실패해도 원본이 그대로라는 사실이 화면 값으로 있다', () => {
    // 사용자에게 "복구했다"거나 "정리했다"고 말하지 않는다 — 바뀐 것이 없다고 말한다 (§13).
    expect(copyViewSource).toContain('preservedNotice: COPY_PRESERVED_NOTICE');
    expect(aiHandoffViewSource).toContain('preservedNotice: AI_EXPORT_PRESERVED_NOTICE');
  });
});

// --- MH-8 — 기존 Connected Provider 경로가 그대로다 --------------------------------------

describe('MH-8 — 기존 생성 경로가 그 자리에 그대로 있다', () => {
  it('위 줄은 aiNoteView가 만든 값을 그대로 들고 있을 뿐 다시 만들지 않는다', () => {
    expect(aiHandoffViewSource).toContain("from './aiNoteView'");
    expect(fields(block(aiHandoffViewSource, 'export interface AutomaticRow {', '\n}\n'))).toEqual([
      'heading',
      'text',
      'available',
      'optionalNotice',
      'tab',
    ]);

    // 생성·상태 조회·이력을 이 모듈이 다시 구현하지 않는다.
    for (const forbidden of ['startAiNote', 'aiNoteStatus', 'listAiNotes', 'getAiNote']) {
      expect(aiHandoffViewSource, `아래 줄 모듈이 ${forbidden}를 다시 만든다`).not.toContain(
        forbidden,
      );
    }
  });

  it('화면은 여전히 기존 생성 경로를 건다', () => {
    // 이 Phase가 더한 것은 또 하나의 길이지 대체가 아니다 — 옛 길의 호출이 그대로 있어야 한다.
    for (const call of ['startAiNote(', 'aiNoteStatus(', 'listAiNotes(', 'aiProviderStatus(']) {
      expect(detailScreenSource, `기존 생성 경로의 ${call}가 사라졌다`).toContain(call);
    }
  });
});

// --- 이 Phase의 자동 테스트가 무엇을 쓰는가 (§18) -----------------------------------------

describe('자동 테스트가 실제 자원을 세우지 않는다 (§18)', () => {
  /**
   * 이 Phase가 더한 테스트 전부.
   *
   * 시스템 clipboard를 건드리지 않는다는 것은 tests/screen-boundary.test.ts가 저장소의 모든
   * 테스트에 대해 이미 본다 — 여기서 보는 것은 나머지 셋이다: 실제 provider · 실제 Notion ·
   * 실제 자격증명 저장소.
   */
  //
  // 이 파일 자신은 목록에 없다 — 금지 문자열을 목록으로 적고 있는 파일이 그 목록에 걸리기
  // 때문이며, `tests/domain_invariants.rs`와 `src-tauri/tests/ollama_adapter.rs`가 금지 문자열을
  // 다루는 방식과 같다. 이 파일이 실제로 무엇을 쓰는지는 위쪽 import 셋이 보여 준다:
  // 파일을 읽는 것 말고는 아무것도 하지 않는다.
  const phaseTests = [
    '../src-tauri/tests/manual_ai_handoff.rs',
    '../src-tauri/tests/manual_handoff_invariants.rs',
    '../src/screens/copyView.test.ts',
    '../src/screens/aiHandoffView.test.ts',
    '../src/platform/clipboard.test.ts',
  ];

  it('실제 provider도 실제 Notion도 실제 자격증명 저장소도 세우지 않는다', () => {
    // **쓰는 모양**을 찾는다 — 금지어를 목록으로 적고 있는 테스트가 스스로에게 걸리지 않도록,
    // 이름을 언급하는 것이 아니라 실제로 부르는 꼴(`::`·`(`)을 본다.
    const realResources = [
      'ureq::',
      'TcpStream::',
      'TcpListener::bind',
      'OsSecretStore',
      'app_secret_store(',
      'keyring::',
      'notion::network',
      'OllamaProvider::new',
    ];

    for (const relative of phaseTests) {
      const source = read(relative);
      for (const forbidden of realResources) {
        expect(source, `${relative}가 실제 자원을 세운다: ${forbidden}`).not.toContain(forbidden);
      }
    }
  });

  it('픽셀 비교 테스트가 없다', () => {
    // 화면의 판정은 값으로 한다. 스크린샷 비교는 이 Phase의 범위 밖이며
    // (`phase-prompt/05.5` Important Rules), 들어오면 여기서 먼저 드러난다.
    const pixelComparison = [
      'toMatchImageSnapshot',
      'jest-image-snapshot',
      'pixelmatch',
      'puppeteer',
      'playwright',
      'toMatchSnapshot',
    ];

    // 이 목록의 문자열은 이 파일 자신에도 들어 있으므로 스스로는 검사 대상이 아니다.
    const testFiles = [
      ...frontendSources.filter((file) => /\.test\.tsx?$/.test(file)),
      ...sourceFiles(path('.').replace(/\/$/, ''), /\.test\.ts$/),
    ].filter((file) => file !== path('./manual-handoff-invariants.test.ts'));

    expect(testFiles.length).toBeGreaterThan(0);
    for (const file of testFiles) {
      const source = readFileSync(file, 'utf8');
      for (const forbidden of pixelComparison) {
        expect(source, `${file}에 픽셀 비교 테스트가 있다`).not.toContain(forbidden);
      }
    }
  });
});
