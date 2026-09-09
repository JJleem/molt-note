// **UI 기반 검사** — `phase-prompt/05.5`의 두 번째 성공 기준을 자동으로 못박는 자리
// (요구 9 · 10 · 11 · 12 · PRODUCT-SPEC §19 · R-1 · R-5).
//
// > 화면 전체가 하나의 UI 기반 위에 선다 — 타이포그래피 · 여백 · 버튼 · 입력 · 상태 ·
// > 빈 상태가 화면마다 제각각이지 않다.
//
// 그 기준은 사람이 보고 판단하는 것(차분한가 · 읽기 쉬운가)과 **자동으로 판정할 수 있는
// 것**으로 나뉜다. 이 파일은 후자만 본다. 여기 있는 다섯 검사는 화면이 예쁜지 말하지 않으며,
// **기반이 무너졌는지**를 말한다.
//
//   1. 타입 스케일과 여백 스케일이 정의돼 있고, 주요 화면이 그것을 쓴다
//   2. 보이는 focus 상태가 존재한다
//   3. 새로 더한 색 토큰이 dark 블록에도 있다 — 이미 있던 장치를 깨뜨리지 않았다
//   4. 금지된 장식이 들어오지 않았다 — gradient · glassmorphism · 그림자 · 장식적 애니메이션
//   5. 색만으로 상태를 말하지 않는다
//
// ## 이 파일이 `tests/` 에 있는 이유
//
// 판정 대상이 **원문**이다 — `src/App.css`의 규칙과 화면 컴포넌트가 실제로 붙이는 class다.
// `src/` 아래의 테스트는 브라우저 코드로 타입 검사되어 `node:fs`를 쓸 수 없으므로,
// 기존 `tests/screen-boundary.test.ts`와 같은 자리에 둔다. 그 파일의 `sourceFiles`가 이미
// `.css`를 읽는 것과 같은 이유다.
//
// ## 화면을 그리지 않는다
//
// 5번의 판정은 **값 수준**이다 — 상태를 만드는 순수 view 모듈을 실제로 불러서 그 결과에
// 사람이 읽는 문장이 있는지 본다. 색은 그 값 어디에도 없으므로, 뜻을 나르는 것은 언제나
// 문장이다. 화면을 띄우거나 그림을 떠서 비교하는 검사를 이 파일에 두지 않는다 — 그것은
// 이 Phase의 범위 밖이며 (`phase-prompt/05.5` Important Rules), 그 금지 자체는
// `tests/manual-handoff-invariants.test.ts`가 저장소의 모든 테스트에 대해 이미 지킨다.
//
// ## 숫자 기준선에 대하여
//
// 아래에 숫자로 못박는 것이 둘 있다 (`HARDCODED_FONT_SIZE_BASELINE` ·
// `HARDCODED_SPACING_BASELINE`). **그 숫자는 목표가 아니라 회귀 방지선이다.** 지금 값에서
// 다시 늘어나면 이 검사가 먼저 깨지고, 줄어들면 기준선을 함께 낮춘다. 숫자를 올려서 검사를
// 통과시키는 것은 기준을 포기하는 결정이므로 diff에 드러나야 한다.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

import type {
  AiProviderState,
  AiProviderStatus,
  ProcessingStatus,
  Recording,
  SessionState,
} from '../src/ipc/types';
import type { Failure } from '../src/ipc/failure';
import { statusBadge } from '../src/screens/recordingsView';
import { INITIAL_RECORDING, observedSession, sessionDisplay } from '../src/screens/recordingView';
import {
  AI_CHECK_FAILED_TEXT,
  AI_NOT_CHECKED_TEXT,
  CHECKING_AI_PROVIDER_TEXT,
  checkedAiProvider,
  failedAiCheck,
} from '../src/screens/aiProviderSettings';
import {
  NO_COPY_ATTEMPT,
  copiedText,
  copyPanel,
  failedCopy,
  startedCopy,
  type CopyAttempt,
} from '../src/screens/copyView';
import {
  NO_AI_EXPORT_ATTEMPT,
  exportedAiRequest,
  failedAiExport,
  manualHandoff,
  startedAiExport,
  type AiExportAttempt,
} from '../src/screens/aiHandoffView';
import { NO_SHOW_FILE_ATTEMPT } from '../src/screens/savedFileView';

const path = (relative: string) => fileURLToPath(new URL(relative, import.meta.url));
const read = (relative: string) => readFileSync(path(relative), 'utf8');

// --- 원문을 읽는 도구 ---------------------------------------------------------------------

/**
 * 규칙을 설명하는 주석이 규칙 위반으로 잡히지 않게 한다.
 *
 * `App.css`는 자기가 무엇을 금지하는지 주석으로 적고 있다 — "그림자도 gradient도 두지
 * 않는다"는 문장이 gradient 금지 검사에 걸리면 그 검사는 아무 뜻도 없어진다.
 */
function withoutComments(css: string): string {
  return css.replace(/\/\*[\s\S]*?\*\//g, '');
}

const cssSource = withoutComments(read('../src/App.css'));

/** 선언 하나 — `padding: var(--space-3) var(--space-2)`. */
interface Declaration {
  readonly property: string;
  readonly value: string;
}

/** 규칙 하나 — 선택자와 그 안의 선언들. */
interface Rule {
  readonly selector: string;
  readonly declarations: readonly Declaration[];
}

function declarationsOf(body: string): Declaration[] {
  return body.split(';').flatMap((piece) => {
    const colon = piece.indexOf(':');
    if (colon < 0) {
      return [];
    }
    const property = piece.slice(0, colon).trim();
    return property === '' ? [] : [{ property, value: piece.slice(colon + 1).trim() }];
  });
}

/**
 * 중괄호를 품지 않는 **가장 안쪽 규칙**을 전부 모은다.
 *
 * `@media`처럼 감싸는 블록은 자기 안에 중괄호를 갖고 있으므로 규칙으로 잡히지 않고, 그 안의
 * 규칙만 잡힌다. 감싸는 블록 자체를 봐야 하는 검사(dark · reduced-motion)는 아래
 * {@link blockOf}로 그 블록만 따로 떼어 본다.
 */
function rulesOf(css: string): Rule[] {
  return [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((matched) => ({
    selector: matched[1].trim().replace(/\s+/g, ' '),
    declarations: declarationsOf(matched[2]),
  }));
}

/** 선택자가 언급하는 class 이름 전부. `.btn--primary:hover`는 `btn--primary` 하나다. */
function classesOf(selector: string): string[] {
  return [...selector.matchAll(/\.([a-zA-Z][\w-]*)/g)].map((matched) => matched[1]);
}

/** `header`로 시작하는 블록의 중괄호 안쪽 원문. 중첩된 규칙이 그대로 들어온다. */
function blockOf(css: string, header: string): string {
  const start = css.indexOf(header);
  expect(start, `${header} 블록이 없다`).toBeGreaterThanOrEqual(0);

  const open = css.indexOf('{', start);
  let depth = 0;
  for (let index = open; index < css.length; index += 1) {
    if (css[index] === '{') {
      depth += 1;
    } else if (css[index] === '}') {
      depth -= 1;
      if (depth === 0) {
        return css.slice(open + 1, index);
      }
    }
  }
  throw new Error(`${header} 블록이 닫히지 않았다`);
}

/** `--이름: 값` 꼴로 정의된 토큰 전부. */
function tokensOf(block: string): Map<string, string> {
  return new Map(
    [...block.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)].map((matched) => [
      matched[1],
      matched[2].trim(),
    ]),
  );
}

const rules = rulesOf(cssSource);

const DARK_HEADER = '@media (prefers-color-scheme: dark)';
const REDUCED_MOTION_HEADER = '@media (prefers-reduced-motion: reduce)';

/** 밝은 테마의 토큰이 정의된 자리. dark 블록보다 앞에 있는 `:root` 하나다. */
const lightRootTokens = tokensOf(blockOf(cssSource.slice(0, cssSource.indexOf(DARK_HEADER)), ':root'));

/** dark 블록 안의 `:root`. */
const darkRootTokens = tokensOf(blockOf(blockOf(cssSource, DARK_HEADER), ':root'));

// --- 검사 1 — 타입 스케일과 여백 스케일 --------------------------------------------------

/**
 * 다섯 위계 (요구 9). 화면이 이 밖의 크기를 만들지 않는다.
 *
 * `--type-display`는 여섯 번째 위계가 아니라 **위계 밖의 한 칸**이며, 아래에서 따로 본다.
 */
const TYPE_SCALE = [
  '--type-page-title',
  '--type-section-title',
  '--type-body',
  '--type-secondary',
  '--type-caption',
];

/** 여백 스케일 (요구 9). 이 여섯 밖의 값을 새로 만들지 않는다. */
const SPACE_SCALE = ['--space-1', '--space-2', '--space-3', '--space-4', '--space-5', '--space-6'];

/** 여백을 만드는 속성. `width`·`border`·`outline-offset`은 여백이 아니므로 여기 없다. */
const SPACING_PROPERTY =
  /^(?:padding|margin|gap|row-gap|column-gap)(?:-(?:top|right|bottom|left|inline|block)(?:-(?:start|end))?)?$/;

/** 하드코딩된 길이 — `12px` · `1.5rem`. `0`과 `auto`는 스케일이 필요 없는 값이므로 아니다. */
const HARDCODED_LENGTH = /(?<![\w-])\d*\.?\d+(?:px|rem|em)\b/;

/**
 * 지금 남아 있는 **임의의 하드코딩 `font-size`** 개수.
 *
 * **이 숫자는 목표가 아니라 회귀 방지선이다.** Phase 시작 시점에는 17곳이었고(R-1) 지금은
 * 전부 타입 스케일 토큰을 지난다. 화면 하나가 자기만의 크기를 다시 만들면 이 숫자가 늘고
 * 검사가 먼저 깨진다. 줄어들면 기준선을 함께 낮추고, 올려야 한다면 그것은 기준을 포기하는
 * 결정이므로 diff에 드러나야 한다.
 */
const HARDCODED_FONT_SIZE_BASELINE = 0;

/**
 * 지금 남아 있는 **임의의 하드코딩 여백** 개수. 위와 같은 뜻의 회귀 방지선이다.
 *
 * Phase 시작 시점에는 padding/margin px 하드코딩이 36곳이었다 (R-1).
 */
const HARDCODED_SPACING_BASELINE = 0;

/** 이 기반 위에 서야 하는 화면들 (요구 10) — 그리고 그 넷을 감싸는 앱 틀. */
const MAIN_SCREENS = [
  '../src/App.tsx',
  '../src/screens/RecordingsScreen.tsx',
  '../src/screens/RecordingScreen.tsx',
  '../src/screens/RecordingDetailScreen.tsx',
  '../src/screens/SettingsScreen.tsx',
];

/** 화면이 실제로 붙이는 class 이름 전부. `className`에 적힌 문자열만 본다. */
function classNamesUsedBy(source: string): Set<string> {
  const used = new Set<string>();
  const add = (literal: string) => {
    for (const name of literal.split(/\s+/)) {
      if (name !== '') {
        used.add(name);
      }
    }
  };

  for (const attribute of source.matchAll(/className\s*=\s*(?:"([^"]*)"|\{([^{}]*)\})/g)) {
    if (attribute[1] !== undefined) {
      add(attribute[1]);
      continue;
    }
    for (const quoted of attribute[2].matchAll(/'([^']*)'|"([^"]*)"/g)) {
      add(quoted[1] ?? quoted[2]);
    }
  }
  return used;
}

/** 그 class들을 그리는 규칙에 실제로 적힌 값 전부. */
function declaredValuesFor(names: Set<string>): string[] {
  return rules
    .filter((rule) => classesOf(rule.selector).some((name) => names.has(name)))
    .flatMap((rule) => rule.declarations.map((declaration) => declaration.value));
}

describe('타입 스케일과 여백 스케일이 있고, 화면이 그 위에 선다 (요구 9 · 10)', () => {
  // 지키려는 것: **화면마다 임의의 크기와 여백을 흩지 않는다.** 크기 하나가 화면 안에서
  // 새로 정해지는 순간, 다음 화면은 또 다른 값을 정하고 그때부터 "기반"은 없다.

  it('다섯 위계와 여섯 여백 칸이 토큰으로 정의돼 있다', () => {
    for (const token of [...TYPE_SCALE, ...SPACE_SCALE]) {
      expect(lightRootTokens.has(token), `${token}이 정의돼 있지 않다`).toBe(true);
    }
  });

  it('위계 밖의 한 칸은 한 자리에서만 쓰인다', () => {
    // `--type-display`(녹음 화면의 경과 시간)는 예외로 둔 값이다. 예외가 둘이 되면 그것은
    // 더 이상 예외가 아니라 여섯 번째 위계이며, 그 결정은 조용히 일어나면 안 된다.
    expect(lightRootTokens.has('--type-display')).toBe(true);

    const users = rules.filter((rule) =>
      rule.declarations.some((declaration) => declaration.value.includes('var(--type-display)')),
    );

    expect(users.map((rule) => rule.selector)).toEqual(['.recording__elapsed']);
  });

  it('임의의 하드코딩 font-size가 기준선을 넘지 않는다', () => {
    // 이 숫자는 회귀 방지선이지 목표가 아니다 — 위 HARDCODED_FONT_SIZE_BASELINE의 설명을 본다.
    const arbitrary = rules.flatMap((rule) =>
      rule.declarations
        .filter(
          (declaration) =>
            declaration.property === 'font-size' && !declaration.value.includes('var(--type-'),
        )
        .map((declaration) => `${rule.selector} { font-size: ${declaration.value} }`),
    );

    expect(arbitrary, '타입 스케일을 지나지 않는 font-size가 늘었다').toHaveLength(
      HARDCODED_FONT_SIZE_BASELINE,
    );
  });

  it('임의의 하드코딩 여백이 기준선을 넘지 않는다', () => {
    // 같은 뜻의 회귀 방지선이다. 토큰 정의 자체(`--space-1: 4px`)는 스케일이 만들어지는
    // 자리이므로 세지 않는다 — 세는 것은 **그 스케일을 지나지 않는 여백**이다.
    const arbitrary = rules.flatMap((rule) =>
      rule.declarations
        .filter(
          (declaration) =>
            SPACING_PROPERTY.test(declaration.property) && HARDCODED_LENGTH.test(declaration.value),
        )
        .map((declaration) => `${rule.selector} { ${declaration.property}: ${declaration.value} }`),
    );

    expect(arbitrary, '여백 스케일을 지나지 않는 여백이 늘었다').toHaveLength(
      HARDCODED_SPACING_BASELINE,
    );
  });

  it('모든 여백이 스케일을 지나거나 0이다', () => {
    // 개수만 세면 `1.5em` 같은 다른 단위가 조용히 들어올 수 있다. 그래서 **남은 값 전부가
    // 실제로 스케일을 지나는가**도 함께 본다 — 여백이 없다는 뜻의 `0`·`auto`만 예외다.
    for (const rule of rules) {
      for (const declaration of rule.declarations) {
        if (!SPACING_PROPERTY.test(declaration.property)) {
          continue;
        }
        const scaleFree = /^(?:0|auto|none|\s)+$/.test(declaration.value);
        expect(
          scaleFree || declaration.value.includes('var(--space-'),
          `${rule.selector}의 ${declaration.property}가 여백 스케일 밖의 값이다: ${declaration.value}`,
        ).toBe(true);
      }
    }
  });

  it('주요 화면이 타입 스케일과 여백 스케일 위에 선다', () => {
    // 토큰이 정의만 되고 아무도 쓰지 않으면 기반이 아니라 장식이다. 각 화면이 붙이는 class를
    // 모아, 그 class를 그리는 규칙이 실제로 두 스케일에 닿는지 본다.
    for (const relative of MAIN_SCREENS) {
      const used = classNamesUsedBy(read(relative));
      expect(used.size, `${relative}가 class를 하나도 붙이지 않는다`).toBeGreaterThan(0);

      const values = declaredValuesFor(used);
      expect(
        values.some((value) => value.includes('var(--type-')),
        `${relative}가 타입 스케일을 쓰지 않는다`,
      ).toBe(true);
      expect(
        values.some((value) => value.includes('var(--space-')),
        `${relative}가 여백 스케일을 쓰지 않는다`,
      ).toBe(true);
    }
  });

  it('화면이 자기 안에서 스타일을 다시 정하지 않는다', () => {
    // inline style 하나가 생기는 순간 그 값은 어떤 스케일도 지나지 않으며, 위의 어떤 검사에도
    // 잡히지 않는다. 스타일이 있는 자리는 `App.css` 하나다.
    for (const relative of MAIN_SCREENS) {
      expect(read(relative), `${relative}에 inline style이 있다`).not.toMatch(/\bstyle\s*=/);
    }
  });
});

// --- 검사 2 — 보이는 focus ----------------------------------------------------------------

describe('보이는 focus 상태가 있다 (요구 12 · R-5)', () => {
  // 지키려는 것: **키보드로 옮겨 다닐 때 지금 어디에 있는지 보인다.** Phase 시작 시점에
  // `:focus` 스타일은 0곳이었고 (R-1 · R-5), 그것이 이 Phase가 메우기로 한 가장 큰 접근성
  // 구멍이다. 다시 0이 되면 여기서 먼저 드러난다.

  const focusRules = rules.filter((rule) => /:focus(?:-visible)?\b/.test(rule.selector));

  it('focus 스타일이 0곳이 아니다', () => {
    expect(focusRules.length, 'focus 스타일이 하나도 없다').toBeGreaterThan(0);
  });

  it('아무 조건 없이 모든 focus에 걸리는 규칙이 하나 있다', () => {
    // class를 붙인 자리에만 focus 링이 있으면, 새로 만든 버튼 하나는 링 없이 태어난다.
    // 바닥에 깔리는 규칙 하나가 그것을 막는다.
    const base = focusRules.find((rule) => rule.selector === ':focus-visible');

    expect(base, '바닥에 깔리는 :focus-visible 규칙이 없다').toBeDefined();
    expect(
      base?.declarations.some(
        (declaration) => declaration.property === 'outline' && !/^(?:none|0)\b/.test(declaration.value),
      ),
      ':focus-visible이 실제로 보이는 것을 그리지 않는다',
    ).toBe(true);
  });

  it('focus 링을 꺼 버리는 자리가 없다', () => {
    // `outline: none`은 링을 지우는 가장 흔한 한 줄이다. 대체 표시 없이 들어오면 위의 바닥
    // 규칙이 있어도 그 자리만 조용히 보이지 않게 된다.
    for (const rule of rules) {
      for (const declaration of rule.declarations) {
        if (declaration.property !== 'outline') {
          continue;
        }
        expect(
          /^(?:none|0)\b/.test(declaration.value),
          `${rule.selector}가 focus 링을 끈다`,
        ).toBe(false);
      }
    }
  });
});

// --- 검사 3 — dark 장치가 깨지지 않았다 ---------------------------------------------------

/** 값이 색인가. 새 표기가 들어오면 아래 {@link isScale}과 함께 이 판정을 늘려야 한다. */
function isColor(value: string): boolean {
  return /#[0-9a-f]{3,8}\b|\b(?:rgba?|hsla?|oklch|oklab|lab|lch|color)\(|\bcurrentColor\b/i.test(
    value,
  );
}

/** 값이 크기·굵기·비율인가. 테마와 무관한 값이며 dark 블록에서 다시 정의하지 않는다. */
function isScale(value: string): boolean {
  return /^-?\d*\.?\d+(?:px|rem|em|%)?$/.test(value);
}

/**
 * 값이 **움직임**인가 — 시간과 가속도 곡선 (2026-09-09).
 *
 * 크기도 색도 아니지만 테마와는 무관하다. 어두운 화면에서 전환이 더 느릴 이유가 없다.
 * 그래서 아래 `dark 블록이 색만 다시 정의한다`가 그대로 이것을 막는다.
 */
function isMotion(value: string): boolean {
  return /^\d*\.?\d+m?s\b/.test(value.trim());
}

/**
 * 값이 **글꼴 목록**인가 (2026-09-09).
 *
 * 색도 크기도 아니고 테마와도 무관하다. 어두운 화면에서 글꼴이 바뀔 이유가 없으므로
 * `dark 블록이 색만 다시 정의한다`가 그대로 이것도 막는다.
 */
function isFontStack(value: string): boolean {
  return /(?:^|,)\s*(?:'[^']+'|"[^"]+"|[-\w]+)/.test(value) && /sans-serif|serif|monospace|system-ui/.test(value);
}

describe('새로 더한 색 토큰이 dark 블록에도 있다 (R-1 · Out of Scope)', () => {
  // 지키려는 것: **이미 있던 dark mode 장치를 깨뜨리지 않는다.** dark는 이 Phase의 목표가
  // 아니다 — 목표가 아니라는 것과 부숴도 된다는 것은 다르다. 색 토큰 하나가 light에만 생기면
  // 어두운 표면 위에서 그 자리만 밝은 값으로 남는다.

  it('검사가 모든 토큰의 형태를 알고 있다', () => {
    // 아래 두 검사는 "색인 것"과 "색이 아닌 것"을 갈라 다르게 다룬다. 어느 쪽으로도 읽히지
    // 않는 값이 들어오면 그 토큰은 두 검사 어디에도 걸리지 않고 조용히 빠져나간다.
    for (const [token, value] of [...lightRootTokens, ...darkRootTokens]) {
      expect(
        isColor(value) || isScale(value) || isMotion(value) || isFontStack(value),
        `${token}의 형태를 이 검사가 모른다: ${value}`,
      ).toBe(
        true,
      );
    }
  });

  it('light의 색 토큰이 전부 dark 블록에도 정의돼 있다', () => {
    const lightColors = [...lightRootTokens].filter(([, value]) => isColor(value)).map(([token]) => token);

    expect(lightColors.length, '색 토큰을 하나도 찾지 못했다').toBeGreaterThan(0);
    for (const token of lightColors) {
      expect(darkRootTokens.has(token), `${token}이 dark 블록에 없다`).toBe(true);
    }
  });

  it('dark 블록이 색만 다시 정의한다', () => {
    // 크기·여백·굵기는 테마와 무관하다. 그것이 dark 블록에 들어오면 스케일이 두 벌이 되고,
    // 한 벌만 고쳤을 때 테마에 따라 글자 크기가 달라진다.
    for (const [token, value] of darkRootTokens) {
      expect(isColor(value), `${token}은 색이 아닌데 dark 블록에서 다시 정의된다`).toBe(true);
      expect(lightRootTokens.has(token), `${token}이 light에 없다`).toBe(true);
    }
  });
});

// --- 검사 4 — 금지된 장식 -----------------------------------------------------------------

describe('장식을 쓰되 규율은 지킨다 (2026-09-09 운영자 결정)', () => {
  // **금지에서 규율로 바뀌었다.** 2026-09-09에 운영자가 gradient · glassmorphism · 그림자 ·
  // 움직임 금지를 걷어냈다. 화면이 쓰기 불편하고 밋밋하다는 판단이었고, 그 결정은 이 diff에
  // 남아 있다.
  //
  // 걷어낸 것은 **금지**이지 **규율**이 아니다. 여백/타입 스케일 · focus · dark 토큰은
  // 그대로다 (위의 describe들). 그것이 없으면 장식은 자유가 아니라 난장이 된다.
  //
  // 그리고 하나는 취향이 아니라 접근성이므로 남긴다: **움직임을 원하지 않는 사람에게
  // 꺼지는 것.** 다만 허용 목록으로 세지 않는다 — 새 움직임을 더하면서 목록을 잊는 일이
  // 실제로 생기기 때문이다. 대신 전역 차단 규칙 하나를 요구한다.

  const declaredCss = rules.flatMap((rule) =>
    rule.declarations.map((declaration) => ({ rule, declaration })),
  );

  it('움직이는 자리가 있으면 reduced-motion에서 전부 꺼진다', () => {
    const moving = declaredCss.filter(
      ({ declaration }) =>
        declaration.property === 'animation' || declaration.property === 'transition',
    );
    if (moving.length === 0) {
      return;
    }

    const reduced = blockOf(cssSource, REDUCED_MOTION_HEADER);
    expect(reduced.length, 'reduced-motion 블록이 없다').toBeGreaterThan(0);

    for (const property of ['animation', 'transition']) {
      expect(
        new RegExp(`${property}[^;]*none\\s*!important`).test(reduced),
        `reduced-motion이 ${property}을 전부 끄지 않는다`,
      ).toBe(true);
    }
  });
});

// --- 검사 5 — 색만으로 상태를 말하지 않는다 -----------------------------------------------

/**
 * 상태를 말하는 자리가 들고 다닐 수 있는 **사람이 읽는 문장**의 이름.
 *
 * 여기에 없는 이름으로 문장을 담기 시작하면 아래 검사는 그것을 문장으로 보지 못하고 실패한다 —
 * 그때 고칠 것은 이 목록이지 화면이 아니다.
 */
const HUMAN_TEXT_FIELDS = [
  'text',
  'headline',
  'hint',
  'label',
  'stateText',
  'statusText',
  'resolution',
] as const;

/** 그 값이 실제로 들고 있는 문장 전부. 빈 문자열은 없는 것과 같으므로 세지 않는다. */
function humanWords(value: unknown): string[] {
  if (value === null || typeof value !== 'object') {
    return [];
  }
  const fields = value as Record<string, unknown>;
  return HUMAN_TEXT_FIELDS.map((field) => fields[field]).filter(
    (found): found is string => typeof found === 'string' && found.trim() !== '',
  );
}

function expectSpeaksInWords(what: string, value: unknown): void {
  expect(humanWords(value), `${what}가 사람이 읽는 문장 없이 상태를 말한다`).not.toHaveLength(0);
}

const RECORDING: Recording = {
  id: 'rec-1',
  title: 'Weekly sync',
  createdAt: '2026-09-05T09:00:00Z',
  updatedAt: '2026-09-05T09:52:31Z',
  durationMs: 3151000,
  durationLabel: '52:31',
  audioPath: '/data/audio/rec-1.m4a',
  audioFormat: 'm4a',
  microphone: 'MacBook Pro Microphone',
  currentTranscriptId: 'tr-1',
  transcriptionStatus: 'done',
  aiStatus: 'none',
  notionStatus: 'none',
};

/** 아직 전사가 없는 같은 녹음. 복사와 export가 "재료가 없다"로 갈리는 자리다. */
const RECORDING_WITHOUT_TRANSCRIPT: Recording = {
  ...RECORDING,
  currentTranscriptId: null,
  transcriptionStatus: 'none',
};

const FAILURE: Failure = {
  kind: 'storage',
  message: '저장소를 읽지 못했다.',
  detail: null,
  sourceDataSafe: true,
  retryable: true,
};

/** clipboard가 이 창에 아예 없을 때의 실패. 복사 실패 갈래 중 다른 길을 안내하는 쪽이다. */
const CLIPBOARD_FAILURE: Failure = {
  ...FAILURE,
  kind: 'unexpected',
  message: '이 창에서는 클립보드에 쓸 수 없다.',
  detail: 'clipboard=unavailable',
};

const PROCESSING_STATUSES: readonly ProcessingStatus[] = [
  'none',
  'pending',
  'running',
  'done',
  'failed',
];

const SESSION_STATES: readonly SessionState[] = ['idle', 'recording', 'paused', 'stopped'];

const AI_PROVIDER_STATES: readonly AiProviderState[] = [
  'notConfigured',
  'ready',
  'noModels',
  'unavailable',
];

function aiProviderStatus(state: AiProviderState): AiProviderStatus {
  return {
    state,
    providerId: state === 'notConfigured' ? null : 'some-provider',
    providerName: state === 'notConfigured' ? null : 'Some Provider',
    locality: state === 'notConfigured' ? null : 'local',
    models: state === 'ready' ? ['a-model'] : [],
    failure: state === 'unavailable' ? FAILURE : null,
  };
}

describe('상태는 언제나 문장으로 온다 — 색은 거들 뿐이다 (요구 12)', () => {
  // 지키려는 것: **색을 구분하지 못하는 사람에게도 상태가 전달된다.** 판정은 값 수준이다 —
  // 상태를 만드는 것은 순수 view 모듈이고, 그 모듈에는 색이 아예 없으므로 뜻을 나르는 것은
  // 언제나 문장이다. 아래 검사들은 각 모듈의 **모든 갈래**를 실제로 불러서 그 문장이 있는지
  // 본다. 갈래가 전부 불렸다는 것도 함께 못박는다 — 부르지 못한 갈래에 대해서는 이 검사가
  // 아무것도 말하지 않기 때문이다.

  it('목록의 후처리 상태 다섯 갈래에 전부 이름과 문장이 있다', () => {
    for (const status of PROCESSING_STATUSES) {
      const badge = statusBadge('Transcript', status);

      expect(badge.label, `${status}에 무엇의 상태인지가 없다`).not.toBe('');
      expect(badge.text.trim(), `${status}에 사람이 읽는 표현이 없다`).not.toBe('');
      // 저장된 식별자를 그대로 보여주는 것은 문장이 아니다.
      expect(badge.text, `${status}가 식별자를 그대로 보여준다`).not.toBe(status);
    }
  });

  it('녹음 화면의 상태 네 갈래와 "아직 모른다"에 전부 문장이 있다', () => {
    for (const state of SESSION_STATES) {
      const display = sessionDisplay(
        // 입력 레벨은 이 검사의 대상이 아니다 — 재지 않은 상태(`null`)로 둔다.
        observedSession(INITIAL_RECORDING, {
          state,
          elapsedMs: 7000,
          elapsedLabel: '0:07',
          level: null,
        }),
      );

      expectSpeaksInWords(`session ${state}`, display);
      // 녹음 중이라는 사실이 `live` 하나로만 전해지면 그것은 색과 굵기뿐이다.
      expect(display.stateText.trim(), `session ${state}에 문장이 없다`).not.toBe('');
    }

    expectSpeaksInWords('아직 물어보지 못한 session', sessionDisplay(INITIAL_RECORDING));
  });

  it('AI provider 상태 다섯 갈래에 전부 문장이 있다', () => {
    const connections = [
      ...AI_PROVIDER_STATES.map((state) => checkedAiProvider(aiProviderStatus(state))),
      failedAiCheck(FAILURE),
    ];

    expect(new Set(connections.map((connection) => connection.kind))).toEqual(
      new Set(['notConfigured', 'running', 'noModels', 'notRunning', 'checkFailed']),
    );
    for (const connection of connections) {
      expectSpeaksInWords(`AI ${connection.kind}`, connection);
    }

    // 아직 물어보지 않았다 · 물어보는 중이다도 상태다. 그 둘은 값이 아니라 상수로 있다.
    for (const text of [AI_NOT_CHECKED_TEXT, CHECKING_AI_PROVIDER_TEXT, AI_CHECK_FAILED_TEXT]) {
      expect(text.trim()).not.toBe('');
    }
  });

  it('복사 자리의 여섯 갈래에 전부 문장이 있다', () => {
    const attempts: readonly CopyAttempt[] = [
      NO_COPY_ATTEMPT,
      startedCopy('prompt', RECORDING.id, 1),
      copiedText('prompt', RECORDING.id, {
        recordingId: RECORDING.id,
        text: '## Transcript\n00:00:03 안녕하세요.\n',
        totalSize: { bytes: 96, chars: 32, lines: 2 },
        portion: 1,
        portionCount: 1,
        portionSize: { bytes: 96, chars: 32, lines: 2 },
      }),
      failedCopy('prompt', RECORDING.id, 1, FAILURE),
      failedCopy('prompt', RECORDING.id, 1, CLIPBOARD_FAILURE),
    ];

    const bodies = [
      copyPanel({ recording: null, mode: 'meeting', attempt: NO_COPY_ATTEMPT }).prompt.body,
      copyPanel({
        recording: RECORDING_WITHOUT_TRANSCRIPT,
        mode: 'meeting',
        attempt: NO_COPY_ATTEMPT,
      }).prompt.body,
      ...attempts.map(
        (attempt) => copyPanel({ recording: RECORDING, mode: 'meeting', attempt }).prompt.body,
      ),
    ];

    expect(new Set(bodies.map((body) => body.kind))).toEqual(
      new Set(['loading', 'nothingToCopy', 'notAsked', 'copying', 'copied', 'failed']),
    );
    for (const body of bodies) {
      // `loading`은 아직 아무 상태도 아니다 — 그 자리를 채우는 것은 화면의 로딩 표시다.
      if (body.kind !== 'loading') {
        expectSpeaksInWords(`복사 ${body.kind}`, body);
      }
    }
  });

  it('Export for AI 자리의 여섯 갈래에 전부 문장이 있다', () => {
    const attempts: readonly AiExportAttempt[] = [
      NO_AI_EXPORT_ATTEMPT,
      startedAiExport(RECORDING.id, 'meeting', 1),
      exportedAiRequest({
        file: {
          recordingId: RECORDING.id,
          path: '/data/exports/weekly-sync.md',
          fileName: 'weekly-sync.md',
        },
        totalSize: { bytes: 7_200, chars: 2_400, lines: 61 },
        portion: 1,
        portionCount: 1,
        portionSize: { bytes: 7_200, chars: 2_400, lines: 61 },
      }),
      failedAiExport(RECORDING.id, 1, FAILURE),
    ];

    // `show`는 **만들어진 파일이 놓인 자리를 여는 시도**다 (`phase-prompt/05.6` 성공 기준 2).
    // 여기서는 아직 아무것도 열지 않은 값을 준다 — 이 검사가 보는 것은 여섯 갈래 각각에
    // 문장이 있는가이지, 여는 수단의 상태가 아니다.
    const bodies = [
      manualHandoff({
        recording: null,
        mode: 'meeting',
        copy: NO_COPY_ATTEMPT,
        aiExport: NO_AI_EXPORT_ATTEMPT,
        show: NO_SHOW_FILE_ATTEMPT,
      }).aiExport.body,
      manualHandoff({
        recording: RECORDING_WITHOUT_TRANSCRIPT,
        mode: 'meeting',
        copy: NO_COPY_ATTEMPT,
        aiExport: NO_AI_EXPORT_ATTEMPT,
        show: NO_SHOW_FILE_ATTEMPT,
      }).aiExport.body,
      ...attempts.map(
        (aiExport) =>
          manualHandoff({
            recording: RECORDING,
            mode: 'meeting',
            copy: NO_COPY_ATTEMPT,
            aiExport,
            show: NO_SHOW_FILE_ATTEMPT,
          }).aiExport.body,
      ),
    ];

    expect(new Set(bodies.map((body) => body.kind))).toEqual(
      new Set(['loading', 'nothingToExport', 'notAsked', 'exporting', 'done', 'failed']),
    );
    for (const body of bodies) {
      if (body.kind !== 'loading') {
        expectSpeaksInWords(`export ${body.kind}`, body);
      }
    }
  });

  it('상태를 나타내는 CSS가 색을 얹을 뿐 뜻을 나르지 않는다', () => {
    // 위 검사들이 "문장이 언제나 있다"를 값으로 못박았다. 여기서 보는 것은 그 반대편이다 —
    // `.status--*`가 색 말고 다른 것을 하기 시작하면(예: `display: none`) 화면에서 뜻을
    // 나르는 것이 다시 CSS가 된다.
    const statusModifiers = rules.filter((rule) => /^\.status--[\w-]+$/.test(rule.selector));

    expect(statusModifiers.length, '상태 표시 규칙을 하나도 찾지 못했다').toBeGreaterThan(0);
    for (const rule of statusModifiers) {
      expect(
        rule.declarations.map((declaration) => declaration.property),
        `${rule.selector}가 색 말고 다른 일을 한다`,
      ).toEqual(['color']);
    }
  });

  it('활성 상태가 색 하나로만 갈리지 않는다', () => {
    // 지금 어느 화면인가 · 어느 탭인가 · 어느 mode인가 · 녹음 중인가는 전부 "고른 것"의 표시다.
    // 그 표시가 글자색 하나뿐이면 색을 구분하지 못하는 사람에게는 아무 표시도 없는 것과 같다.
    // 색이 아닌 신호(굵기 · 면 · 밑줄 · 테두리)가 적어도 하나 함께 있어야 한다.
    const NON_COLOR_SIGNAL =
      /^(?:font-weight|text-decoration|border(?:-\w+)?|background-color|outline|opacity)$/;

    const active = rules.filter((rule) => /--(?:active|live|selected|current)\b/.test(rule.selector));

    expect(active.length, '활성 상태 규칙을 하나도 찾지 못했다').toBeGreaterThan(0);
    for (const rule of active) {
      expect(
        rule.declarations.some((declaration) => NON_COLOR_SIGNAL.test(declaration.property)),
        `${rule.selector}가 색 하나로만 활성을 말한다`,
      ).toBe(true);
    }
  });
});
