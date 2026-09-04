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
    // elapsedLabel)를 여기서 만들기 시작하면 규칙이 두 벌이 되고 조용히 갈라진다.
    const source = readFileSync(TIMESTAMP_MODULE, 'utf8');
    const timestampFacts = [/durationMs/, /durationLabel/, /elapsedMs/, /elapsedLabel/];

    for (const shape of timestampFacts) {
      expect(source, '녹음 길이는 Rust가 만든 값을 쓴다').not.toMatch(shape);
    }
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

describe('실패는 사용자에게 보인다', () => {
  it('src/ 아래에 실패를 console로 흘려보내는 경로가 없다', () => {
    // console.error로 끝나면 사용자는 아무것도 알지 못한다 (§13).
    for (const file of frontendSources) {
      const source = readFileSync(file, 'utf8');
      expect(source, `${file}에 console 출력이 있다`).not.toMatch(/\bconsole\s*\.\s*\w+\s*\(/);
    }
  });
});
