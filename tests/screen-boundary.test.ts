// 화면 경계 테스트.
//
// 개별 변환 함수의 동작은 각 모듈 옆의 테스트가 본다. 여기서 보는 것은 **소스 전체에 대한
// 두 가지 규칙**이며, 새 파일이 하나 생기는 것만으로 조용히 깨질 수 있는 것들이다.
//
//   1. 길이를 사람이 읽는 형식으로 바꾸는 규칙이 TypeScript에 다시 생기지 않았다.
//      그 규칙은 src-tauri/src/domain/duration.rs 한 곳에만 있고, 화면은 Rust가 보낸
//      durationLabel을 쓴다. 두 벌이 되면 조용히 갈라진다.
//   2. 실패가 console에만 남고 끝나는 경로가 없다 (PRODUCT-SPEC §13).
//
// ## 예외가 하나 있다 — segment timestamp (2026-09-03 · phase-prompt/03 요구 6)
//
// Transcript의 `00:02:14 → 00:02:21`은 **녹음 길이가 아니다.** 녹음 하나가 얼마나 긴가와
// 이 문장이 녹음의 어디인가는 다른 값이고 형식도 다르며, Rust는 후자의 문자열을 보내지
// 않는다 — Transcript payload가 보내는 것은 밀리초 두 개다. 그래서 그 변환은 화면 쪽에
// **한 모듈**에만 있고, 그 모듈은 자기 옆의 리터럴 기대값 테스트로 판정된다
// (src/screens/transcriptView.test.ts).
//
// 예외를 파일 하나로 못 박고, 그 파일이 녹음 길이 쪽으로 넘어오지 않는다는 것도 함께 본다 —
// 규칙이 두 벌이 되는 것을 막는 것이 이 검사의 목적이지 산술을 금지하는 것이 목적이 아니다.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const path = (relative: string) => fileURLToPath(new URL(relative, import.meta.url));

function sourceFiles(directory: string): string[] {
  return readdirSync(directory).flatMap((entry) => {
    const full = `${directory}/${entry}`;
    if (statSync(full).isDirectory()) {
      return sourceFiles(full);
    }
    return /\.(ts|tsx|js|jsx|css)$/.test(entry) ? [full] : [];
  });
}

const frontendSources = sourceFiles(path('../src'));
const recordingsViewSource = readFileSync(path('../src/screens/recordingsView.ts'), 'utf8');
const recordingViewSource = readFileSync(path('../src/screens/recordingView.ts'), 'utf8');

/**
 * 시각을 문자열로 바꾸는 계산이 허용된 **단 하나의** 모듈.
 *
 * 여기서 만드는 것은 Transcript segment의 위치(`00:02:14`)이며 녹음 길이가 아니다.
 * 목록·상세·녹음 화면의 길이 표시는 여전히 Rust가 만든 값을 그대로 쓴다.
 */
const TIMESTAMP_MODULE = path('../src/screens/transcriptView.ts');

describe('길이 포맷은 Rust에만 있다', () => {
  it('src/ 아래에 초를 mm:ss로 바꾸는 계산이 없다', () => {
    // 계산의 모양을 찾는다 — 분·초로 쪼개는 나눗셈/나머지와 두 자리 채우기.
    const durationArithmetic = [
      /%\s*60\b/, //            초 → 분 나머지
      /\/\s*60\b/, //           초 → 분
      /\/\s*1_?000\b/, //       밀리초 → 초
      /padStart\s*\(\s*2/, //   0을 채워 두 자리로
      /toFixed\s*\(\s*2\s*\)[\s\S]{0,40}:/, // 초를 소수로 만들고 콜론을 붙이는 변형
    ];

    for (const file of frontendSources.filter((file) => file !== TIMESTAMP_MODULE)) {
      const source = readFileSync(file, 'utf8');
      for (const shape of durationArithmetic) {
        expect(source, `${file}에 길이 포맷 계산이 있다`).not.toMatch(shape);
      }
    }
  });

  it('예외인 모듈이 실제로 그 자리에 있다', () => {
    // 예외를 파일 경로로 못 박았으므로, 그 파일이 사라지거나 이름이 바뀌면 검사가 조용히
    // 아무것도 면제하지 않는 상태가 된다 — 그때 이 테스트가 먼저 알린다.
    expect(frontendSources).toContain(TIMESTAMP_MODULE);
  });

  it('예외인 모듈이 녹음 길이 쪽으로 넘어오지 않는다', () => {
    // 이 모듈이 만드는 것은 segment의 위치뿐이다. 녹음 하나의 길이(durationLabel ·
    // elapsedLabel)나 전사에 걸린 시간(transcriptionMs)을 여기서 만들기 시작하면 규칙이
    // 두 벌이 되고 조용히 갈라진다.
    const source = readFileSync(TIMESTAMP_MODULE, 'utf8');
    const timestampFacts = [
      /durationMs/,
      /durationLabel/,
      /elapsedMs/,
      /elapsedLabel/,
      // 전사에 걸린 시간의 밀리초. 이 모듈은 Rust가 만든 문장(transcriptionLabel)만 나른다.
      /transcriptionMs/,
    ];

    for (const shape of timestampFacts) {
      expect(source, '녹음 길이는 Rust가 만든 값을 쓴다').not.toMatch(shape);
    }
  });

  it('전사에 걸린 시간이 backend가 준 문장 그대로다', () => {
    // 사람이 Metal 전후를 비교하는 값이다 (phase-prompt/05.6 성공 기준 3). 여기서 밀리초를
    // 나눠 문장을 만들기 시작하면 저장된 값과 화면이 서로 다른 규칙으로 읽히게 된다.
    const source = readFileSync(TIMESTAMP_MODULE, 'utf8');

    expect(source).toMatch(/transcriptionLabel:\s*transcript\.transcriptionLabel/);
  });

  it('목록 항목의 길이가 저장소에서 온 값 그대로다', () => {
    expect(recordingsViewSource).toMatch(/durationLabel:\s*recording\.durationLabel/);
  });

  it('녹음 화면의 경과 시간이 backend가 준 값 그대로다', () => {
    // 녹음 중에 화면에 가장 크게 보이는 값이다 (§19). 여기서 초를 세기 시작하면 화면과
    // 저장되는 길이가 서로 다른 규칙으로 만들어진다.
    expect(recordingViewSource).toMatch(/elapsedLabel:\s*session\.elapsedLabel/);
  });
});

/**
 * 입력 레벨의 **환산과 판정**이 TypeScript에 다시 생기지 않았다는 것을 원문으로 못박는 자리
 * (docs/ADR-0003-recording-engine.md §16.3 · §16.4).
 *
 * 위의 길이 포맷 검사와 같은 이유다 — dBFS 환산도, `-36`/`-60`의 판정 구간도, 사람이 읽는
 * 문장도 `src-tauri/src/audio/level.rs` 한 자리에 있다. 화면이 같은 규칙을 한 벌 더 갖는
 * 순간 둘은 조용히 갈라지고, 그때 화면이 보여 주는 "쓸 만함"은 저장되는 소리와 무관해진다.
 *
 * 갈래의 모양은 `src/screens/recordingView.test.ts`가 고정한다 — 값 없음 · 낮음 · 쓸 만함이
 * 서로 다른 결과이고 경고가 언제 나오는지가 거기 있다. 여기서 보는 것은 **그 규칙을 만드는
 * 자리가 숫자를 다시 만지지 않는다**는 것이며, 파일 하나가 새로 생기는 것만으로 조용히 깨질
 * 수 있으므로 원문을 읽는 이 파일에 있다.
 */
describe('입력 레벨의 환산과 판정은 Rust에만 있다 (ADR-0003 §16.4)', () => {
  /** 규칙을 설명하는 주석은 검사 대상이 아니다 — 찾는 것은 **코드에 쓰인 숫자**다. */
  const codeOf = (file: string): string =>
    readFileSync(file, 'utf8')
      .split('\n')
      .filter((line) => !line.trim().startsWith('*') && !line.trim().startsWith('//'))
      .join('\n');

  it('src/ 아래에 dBFS 환산이 없다', () => {
    // 환산의 모양 — 로그와 16-bit 풀스케일. 하나라도 생기면 화면이 자기 수치를 갖게 된다.
    const conversion = [/\blog10\b/, /\b32768\b/, /\b32_768\b/];

    for (const file of frontendSources) {
      const source = codeOf(file);
      for (const shape of conversion) {
        expect(source, `${file}에 dBFS 환산이 있다`).not.toMatch(shape);
      }
    }
  });

  it('src/ 아래에 판정 임계값이 없다', () => {
    // -36(쓸 만함)과 -60(소리 없음)은 §16.4가 정한 구간이다. 같은 숫자가 화면에 생기면
    // 규칙이 두 벌이 되고, 한쪽만 고쳐지는 날 화면은 사용자에게 거짓말을 하게 된다.
    const thresholds = [/-\s*36(?!\d)/, /-\s*60(?!\d)/];

    for (const file of frontendSources) {
      const source = codeOf(file);
      for (const shape of thresholds) {
        expect(source, `${file}에 레벨 판정 임계값이 있다`).not.toMatch(shape);
      }
    }
  });

  it('화면이 backend의 판정과 문장을 그대로 나른다', () => {
    // 갈래는 `verdict`, 사람이 읽는 문장은 `message`다. 화면이 문장을 조립하기 시작하면
    // 수치와 문장이 서로 다른 자리에서 만들어진다.
    expect(recordingViewSource).toMatch(/kind:\s*level\.verdict/);
    expect(recordingViewSource).toMatch(/text:\s*level\.message/);
  });
});

/**
 * 내보내기가 AI를 보지 않는다는 것을 원문으로 못박는 자리 (INV-8 · `phase-prompt/05` P-3).
 *
 * 값의 모양은 `src/screens/exportView.test.ts`가 고정한다 — 입력에 provider를 담을 자리가 없고,
 * AI 상태가 무엇이든 내보내기 동작이 같다는 것이 거기 있다. 여기서 보는 것은 그 모듈이
 * **안에서 몰래 provider를 보지 않는다**는 것이며, 원문을 읽는 검사이므로 이 파일에 있다
 * (`src/`는 브라우저 코드로 타입 검사되어 node:fs를 쓸 수 없다).
 */
const EXPORT_VIEW_MODULE = path('../src/screens/exportView.ts');

describe('Markdown export는 AI를 보지 않는다 (INV-8)', () => {
  it('export 자리의 순수 모듈에 provider도 AI 상태도 등장하지 않는다', () => {
    expect(frontendSources).toContain(EXPORT_VIEW_MODULE);

    const source = readFileSync(EXPORT_VIEW_MODULE, 'utf8');
    // provider 하나를 보기 시작하면 그 순간 provider가 내보내기를 막을 수 있게 된다.
    expect(source).not.toMatch(/AiProviderStatus|aiProviderStatus|providerName|locality/);
    expect(source).not.toMatch(/\baiStatus\b/);
  });
});

/**
 * secret이 화면 쪽에 **남지 않는다**는 것을 원문으로 못박는 자리 (INV-7 · ADR-0009 §10.4 ·
 * `phase-prompt/05` 요구 10).
 *
 * 값의 모양은 `src/screens/notionSettings.test.ts`가 고정한다 — 화면 상태에 token을 담을 자리가
 * 없다는 것이 거기 있다. 여기서 보는 것은 **저장소에 쓰는 경로가 `src/` 어디에도 생기지
 * 않았다**는 것이며, 파일 하나가 새로 생기는 것만으로 조용히 깨질 수 있으므로 원문을 읽는
 * 이 파일에 있다.
 */
const NOTION_SETTINGS_MODULE = path('../src/screens/notionSettings.ts');
const SETTINGS_SCREEN = path('../src/screens/SettingsScreen.tsx');

describe('token은 화면에 남지 않는다 (INV-7)', () => {
  it('src/ 아래에 브라우저 저장소를 쓰는 경로가 없다', () => {
    // localStorage · sessionStorage · IndexedDB · document.cookie는 전부 webview에 값을
    // **남기는** 자리다. 하나라도 생기면 secret이 앱 밖에서도 읽히는 자리가 된다.
    //
    // 찾는 것은 **실제로 그것을 쓰는 모양**이다(뒤에 오는 `.`이나 `[`). 이름을 문장 안에서
    // 언급한 주석까지 걸리면, 규칙을 적어 둔 파일이 규칙 위반으로 보고된다.
    const browserStorage = [
      /\blocalStorage\s*[.[]/,
      /\bsessionStorage\s*[.[]/,
      /\bindexedDB\s*[.[]/i,
      /document\s*\.\s*cookie/,
    ];

    for (const file of frontendSources) {
      const source = readFileSync(file, 'utf8');
      for (const shape of browserStorage) {
        expect(source, `${file}에 브라우저 저장소 사용이 있다`).not.toMatch(shape);
      }
    }
  });

  it('token 입력란이 화면 상태를 갖지 않고, 저장된 값을 되읽어 채우지 않는다', () => {
    expect(frontendSources).toContain(NOTION_SETTINGS_MODULE);

    const screen = readFileSync(SETTINGS_SCREEN, 'utf8');
    const tokenField = screen.slice(
      screen.indexOf('id="notion-token"'),
      screen.indexOf('id="notion-parent-page"'),
    );

    expect(tokenField, 'token 입력란을 찾지 못했다').not.toBe('');
    // `value`가 붙는 순간 그 값은 React 상태에서 오게 되고, 그러면 화면이 token을 들고 있게 된다.
    expect(tokenField).not.toMatch(/\bvalue=/);
    expect(tokenField).not.toMatch(/\bdefaultValue=/);
    expect(tokenField).toMatch(/ref=\{tokenInput\}/);
    // 넘긴 뒤 곧바로 비운다 — 응답을 기다리지 않는다.
    expect(screen).toMatch(/input\.value = ''/);
  });

  it('token 값을 돌려주는 조회 경로가 없다', () => {
    // 저장된 token을 읽어 오는 command가 없으므로 화면이 그것을 채워 넣을 방법도 없다.
    // `src/ipc/commands.ts`에 그런 이름이 생기면 여기서 먼저 드러난다.
    const commands = readFileSync(path('../src/ipc/commands.ts'), 'utf8');

    expect(commands).not.toMatch(/get_notion_token|getNotionToken|readNotionToken/);
    // Notion token을 인자로 받는 함수는 저장 하나뿐이다.
    const takesAToken = [...commands.matchAll(/export function (\w+)\([^)]*token[^)]*\)/g)].map(
      (match) => match[1],
    );
    expect(takesAToken).toEqual(['saveNotionToken']);
  });

  it('순수 view 모듈이 token 값을 들고 있지 않다', () => {
    const source = readFileSync(NOTION_SETTINGS_MODULE, 'utf8');

    // 이 모듈이 아는 것은 '저장돼 있는가'뿐이다. 값이 지나갈 자리가 생기면 그것이 곧 화면
    // 상태에 남는 secret이 된다.
    expect(source).not.toMatch(/token\s*:\s*string/);
    expect(source).not.toMatch(/saveNotionToken|deleteNotionToken|invoke\s*\(/);
  });
});

/**
 * 파일이 놓인 자리를 여는 일이 **화면 쪽에서 OS를 알지 않는다**는 것을 원문으로 못박는 자리
 * (`phase-prompt/05.6` 성공 기준 2 · R-4 · INV-10).
 *
 * 값의 모양은 `src/screens/savedFileView.test.ts`가 고정한다 — 여는 동작이 값으로 있고, 열지
 * 못했을 때 무엇이 그대로인지도 값에 있다는 것이 거기 있다. 여기서 보는 것은 **그 규칙을 만드는
 * 자리가 OS도 command도 알지 않는다**는 것이며, 컴포넌트 하나가 새로 생기는 것만으로 조용히
 * 깨질 수 있으므로 원문을 읽는 이 파일에 있다.
 *
 * OS 이름이 화면 쪽으로 새면 두 가지가 무너진다 — 다른 플랫폼에서 틀린 문장을 말하게 되고
 * (`savedFileView`의 문장은 사용자에게 그대로 보인다), 갈아 끼울 자리가 backend 하나가 아니게
 * 된다 (`src-tauri/src/platform/file_manager.rs`).
 */
const SAVED_FILE_VIEW_MODULE = path('../src/screens/savedFileView.ts');

describe('자리를 여는 규칙은 순수 모듈에 있고 OS를 알지 않는다 (INV-10)', () => {
  it('그 모듈이 실제로 그 자리에 있고 React 컴포넌트가 아니다', () => {
    expect(frontendSources).toContain(SAVED_FILE_VIEW_MODULE);
    expect(SAVED_FILE_VIEW_MODULE.endsWith('.tsx')).toBe(false);
  });

  it('순수 모듈이 command도 DOM도 OS도 부르지 않는다', () => {
    const source = readFileSync(SAVED_FILE_VIEW_MODULE, 'utf8');

    // 판정이 command 없이 끝나야 창 하나 열지 않고 vitest로 전부 판정된다 (§18).
    expect(source).not.toContain("from '../ipc/commands'");
    expect(source).not.toMatch(/\binvoke\s*[<(]/);
    expect(source).not.toContain('showSavedFile(');
    // **그것을 쓰는 모양**을 찾는다 — 이 모듈의 문장 하나가 "opens a window."로 끝나므로,
    // 낱말 자체를 막으면 사용자에게 보이는 문장이 규칙 위반으로 보고된다.
    expect(source).not.toMatch(/\b(?:document|window)\s*(?:\.\s*\w|\[)/);
  });

  it('src/ 아래 어디에도 어느 OS의 파일 관리자인지 아는 코드가 없다', () => {
    // 화면은 "자리를 연다"까지만 안다. 무엇으로 여는지는 backend의 platform 경계 하나가 안다.
    // 주석이 아니라 **코드에 쓰인 이름**을 찾는다 — 규칙을 설명하는 문장은 검사 대상이 아니다.
    const osNames = [/\bFinder\b/, /\bexplorer\.exe\b/, /'open'/, /"open"/, /\bAppleScript\b/];

    for (const file of frontendSources) {
      const source = readFileSync(file, 'utf8')
        .split('\n')
        .filter((line) => !line.trim().startsWith('*') && !line.trim().startsWith('//'))
        .join('\n');

      for (const shape of osNames) {
        expect(source, `${file}이 OS 파일 관리자를 이름으로 안다 (INV-10)`).not.toMatch(shape);
      }
    }
  });

  it('만들어진 파일의 전체 경로가 화면에서 사라지지 않았다 (§4.1 · 성공 기준 2)', () => {
    // 여는 수단은 경로의 **대체가 아니라 추가다.** 경로 줄이 사라지면 여는 수단이 실패했을 때
    // 사용자에게 남는 길이 없어진다.
    const screen = readFileSync(path('../src/screens/RecordingDetailScreen.tsx'), 'utf8');

    expect(screen).toContain('{body.file.path}');
    expect(screen).toContain('{body.path}');
    // 그리고 그 옆에 여는 수단이 있다 — 두 export 자리 모두에.
    expect([...screen.matchAll(/<ShowFileControl/g)]).toHaveLength(2);
  });
});

describe('실패는 사용자에게 보인다', () => {
  it('src/ 아래에 실패를 console로 흘려보내는 경로가 없다', () => {
    // console.error로 끝나면 사용자는 아무것도 알지 못한다 (§13).
    for (const file of frontendSources) {
      const source = readFileSync(file, 'utf8');
      expect(source, `${file}에 console 출력이 있다`).not.toMatch(/\bconsole\s*\.\s*\w+\s*\(/);
    }
  });
});

/**
 * clipboard를 실제로 부르는 자리가 **하나**라는 것을 원문으로 못박는 자리
 * (`phase-prompt/05.5` R-4 · 요구 5 · docs/ADR-0010-manual-ai-handoff.md §7.2 · INV-10).
 *
 * 값의 모양은 `src/platform/clipboard.test.ts`와 `src/screens/copyView.test.ts`가 고정한다 —
 * 실패가 값으로 돌아온다는 것과 복사 상태 네 갈래가 거기 있다. 여기서 보는 것은 **부르는
 * 자리가 흩어지지 않았다**는 것이며, 컴포넌트 하나가 새로 생기는 것만으로 조용히 깨질 수
 * 있으므로 원문을 읽는 이 파일에 있다.
 *
 * 부르는 자리가 하나라는 것이 지켜지지 않으면 두 가지가 무너진다 — 실패를 다루는 방식이
 * 자리마다 달라지고(§13), 플랫폼이 이 능력을 주지 않을 때 갈아 끼울 자리가 사라진다
 * (ADR-0010 §7.4 · §7.5).
 */
const CLIPBOARD_MODULE = path('../src/platform/clipboard.ts');
const COPY_VIEW_MODULE = path('../src/screens/copyView.ts');

/**
 * webview의 clipboard에 실제로 닿는 코드의 모양.
 *
 * 이름을 문장 안에서 언급한 주석이 아니라 **그것을 쓰는 모양**을 찾는다. `execCommand('copy')`는
 * 같은 일을 하는 옛 경로이며, 경계를 우회하는 두 번째 자리가 되므로 함께 막는다.
 */
const CLIPBOARD_USE = [
  /navigator\s*\.\s*clipboard/,
  /navigator\s*\[\s*['"]clipboard/,
  /\bClipboardItem\b/,
  /execCommand\s*\(\s*['"]copy/,
];

/** 이 검사 자신은 위 모양을 정규식으로 적고 있으므로 스스로에게 걸리지 않게 제외한다. */
const THIS_FILE = path('./screen-boundary.test.ts');

/** `tests/` 아래의 원문. 끝의 `/`를 떼어야 위 경로들과 같은 모양이 된다. */
const testSources = sourceFiles(path('.').replace(/\/$/, ''));

describe('clipboard는 경계 하나 뒤에 있다 (R-4 · INV-10)', () => {
  it('src/ 아래에서 clipboard를 부르는 파일이 정확히 하나다', () => {
    const callers = frontendSources.filter((file) => {
      const source = readFileSync(file, 'utf8');
      return CLIPBOARD_USE.some((shape) => shape.test(source));
    });

    expect(callers).toEqual([CLIPBOARD_MODULE]);
  });

  it('그 하나가 React 컴포넌트가 아니다', () => {
    // 컴포넌트 여기저기에서 부르기 시작하면 실패를 값으로 돌려줄 자리가 사라진다 (R-4).
    expect(CLIPBOARD_MODULE.endsWith('.tsx')).toBe(false);
    expect(frontendSources).toContain(CLIPBOARD_MODULE);
  });

  it('복사 상태를 판정하는 순수 모듈이 clipboard를 부르지 않는다', () => {
    // 네 갈래(아직 안 함 · 복사 중 · 복사됨 · 실패)의 판정은 DOM도 clipboard도 모르는 자리에
    // 있어야 clipboard 없이 vitest로 전부 판정된다 (§18).
    expect(frontendSources).toContain(COPY_VIEW_MODULE);

    const source = readFileSync(COPY_VIEW_MODULE, 'utf8');
    expect(source).not.toMatch(/copyText\s*\(/);
    expect(source).not.toMatch(/systemClipboard\s*\(/);
    expect(source).not.toMatch(/writeText/);
    expect(source).not.toMatch(/document\s*\.|window\s*\./);
    // provider를 보기 시작하면 그 순간 provider가 복사를 막을 수 있게 된다 (MH-1 · MH-2).
    expect(source).not.toMatch(/AiProviderStatus|aiProviderStatus|providerName|locality|ollama/i);
    expect(source).not.toMatch(/\baiStatus\b/);
  });

  it('자동 테스트가 실제 시스템 clipboard를 건드리지 않는다', () => {
    // 경계는 쓰는 대상을 인자로 받으므로 테스트는 언제나 test double을 넘긴다. 테스트 어디에도
    // 이 창의 clipboard를 집는 자리가 없다는 것이 그 사실을 지킨다.
    const testFiles = [
      ...frontendSources.filter((file) => /\.test\.tsx?$/.test(file)),
      ...testSources.filter((file) => file !== THIS_FILE),
    ];

    expect(testFiles.length).toBeGreaterThan(0);
    for (const file of testFiles) {
      const source = readFileSync(file, 'utf8');
      for (const shape of CLIPBOARD_USE) {
        expect(source, `${file}가 실제 clipboard를 집는다`).not.toMatch(shape);
      }
    }
  });
});
