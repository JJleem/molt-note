// 입력 레벨 · 붕괴 판정 경계 테스트 (Phase 5.7).
//
// 검사 대상은 동작이 아니라 **소스 전체에 대한 규칙**이다. 파일 하나가 새로 생기거나 필드
// 하나가 늘어나는 것만으로 조용히 깨질 수 있는 것들이므로, 개별 모듈 옆의 테스트가 아니라
// 원문을 읽는 이 자리에서 본다 (`tests/screen-boundary.test.ts` ·
// `tests/ipc-boundary.test.ts`의 선례 — `src/`는 브라우저 코드로 타입 검사되어 node:fs를
// 쓸 수 없고, Rust의 `include_str!` 검사는 자기 crate 안쪽만 본다).
//
// 이 Phase가 세운 다섯 가지를 고정한다.
//
//   (a) 레벨을 알리는 경로가 **오디오 샘플 자체**를 화면이나 네트워크로 내보내지 않는다
//       (INV-6 · PRODUCT-SPEC §12 Privacy Boundary · phase-prompt/05.7 Constraints).
//   (b) 붕괴 판정 규칙이 **한 자리에만** 있다 — 임계값과 비율 계산이 `run.rs` · 엔진 ·
//       화면에 복제돼 있지 않다 (ADR-0007 §18.3).
//   (c) 화면 쪽에 dBFS 계산도 레벨 판정 임계값도 없다 — backend가 준 값과 문장을 그대로
//       쓴다 (ADR-0003 §16.3 · §16.4).
//   (d) 캡처 경로가 샘플을 바꾸지 않는다 — 이 Phase는 게인도 정규화도 적용하지 않으므로
//       들어온 샘플이 그대로 파일에 쓰인다 (ADR-0003 §16.5 · phase-prompt/05.7).
//   (e) 새 벤더 고유 개념이 core/domain · payload · frontend 타입에 생기지 않았다 (INV-9).
//
// ## 왜 이것들이 한 파일에 함께 있는가
//
// 2026-09-07에 사람이 51분을 잃은 것은 두 침묵이 겹쳤기 때문이다 — 녹음 중에는 소리가
// 담기지 않는다는 것을 말해 주지 않았고, 전사 뒤에는 붕괴한 결과를 완료라고 말했다.
// 그 두 침묵을 깬 규칙이 **각각 한 자리에** 산다는 것이 이 Phase가 남긴 구조이며,
// 그 규칙이 두 벌이 되는 순간 화면과 저장되는 것이 조용히 갈라진다.
//
// 여기 있는 것은 전부 **이미 만들어진 경로에 대한 검사**다. 제품 코드를 고쳐야만 통과하는
// 검사는 쓰지 않는다.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const path = (relative: string) => fileURLToPath(new URL(relative, import.meta.url));
const read = (file: string) => readFileSync(file, 'utf8');

/** 주어진 디렉터리 아래의 모든 파일 경로 (확장자로 거른다). */
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
 * 줄 앞에 오는 주석을 걷어낸 소스.
 *
 * 규칙을 설명하는 문장이 규칙 위반으로 잡히면, 규칙을 적어 둔 파일이 가장 먼저 실패한다.
 * 찾는 것은 언제나 **코드에 쓰인 것**이다.
 */
function withoutComments(source: string): string {
  return source
    .split('\n')
    .filter((line) => {
      const trimmed = line.trim();
      return !trimmed.startsWith('//') && !trimmed.startsWith('*') && !trimmed.startsWith('/*');
    })
    .join('\n');
}

/** `#[cfg(test)]` 앞까지, 즉 **제품이 실행하는 Rust 코드**만 남긴다. */
function productionRust(source: string): string {
  return withoutComments(source.split('#[cfg(test)]')[0]);
}

/**
 * `pub use ...;` 재수출을 걷어낸 소스.
 *
 * 재수출은 **한 자리에 있는 정의를 가리키는 것**이지 두 번째 정의가 아니다
 * (`src-tauri/src/transcription/mod.rs`). 그것을 복제로 세면 모듈 barrel이 있다는 이유만으로
 * 규칙이 깨졌다고 보고하게 된다 — 여기서 찾는 것은 **같은 규칙의 두 번째 벌**이다.
 */
function withoutReExports(production: string): string {
  return production.replace(/pub use [\s\S]*?;/g, '');
}

/**
 * 붕괴 판정 규칙의 **값과 이름** (ADR-0007 §18.2).
 *
 * 사람이 읽는 문장의 이름(`TRANSCRIPTION_COLLAPSED_NOTICE` 같은 것)은 규칙이 아니다 —
 * 그래서 `COLLAPSED_`로 뭉뚱그리지 않고 임계값 상수의 이름을 그대로 적는다.
 */
const COLLAPSE_RULE = [
  /\b0\.20?\b/,
  /\b0\.50?\b/,
  /MINIMUM_SENTENCES_TO_JUDGE/,
  /COLLAPSED_UNIQUE_RATIO_AT_OR_BELOW/,
  /COLLAPSED_TOP_REPEAT_SHARE_AT_OR_ABOVE/,
];

const rustSources = sourceFiles(path('../src-tauri/src'), /\.rs$/);
const audioSources = sourceFiles(path('../src-tauri/src/audio'), /\.rs$/);
const transcriptionSources = sourceFiles(path('../src-tauri/src/transcription'), /\.rs$/);
const domainSources = sourceFiles(path('../src-tauri/src/domain'), /\.rs$/);
/** 기기 밖으로 나가는 요청을 만드는 자리 전부 — AI · Notion · 파일 내보내기 · 전송 실행. */
const outboundSources = [
  ...sourceFiles(path('../src-tauri/src/ai'), /\.rs$/),
  ...sourceFiles(path('../src-tauri/src/notion'), /\.rs$/),
  ...sourceFiles(path('../src-tauri/src/export'), /\.rs$/),
  ...sourceFiles(path('../src-tauri/src/sync'), /\.rs$/),
];

const frontendSources = sourceFiles(path('../src'), /\.(ts|tsx)$/);
/** 화면 쪽의 **제품 코드**. 옆에 붙은 테스트는 뺀다. */
const frontendProduction = frontendSources.filter((file) => !/\.test\.tsx?$/.test(file));

const COLLAPSE_MODULE = path('../src-tauri/src/transcription/collapse.rs');
const LEVEL_MODULE = path('../src-tauri/src/audio/level.rs');
const CAPTURE_MODULE = path('../src-tauri/src/audio/capture.rs');
const RUN_MODULE = path('../src-tauri/src/transcription/run.rs');
const PAYLOAD_MODULE = path('../src-tauri/src/commands/payload.rs');
const TYPES_MODULE = path('../src/ipc/types.ts');
const RECORDING_VIEW_MODULE = path('../src/screens/recordingView.ts');

const collapseSource = read(COLLAPSE_MODULE);
const captureSource = read(CAPTURE_MODULE);
const payloadSource = read(PAYLOAD_MODULE);
const payloadProduction = productionRust(payloadSource);
const typesSource = read(TYPES_MODULE);

describe('검사가 실제로 원문을 읽었다', () => {
  it('읽을 소스가 비어 있지 않다', () => {
    // 아래 모든 검사가 "목록이 비어서" 통과하는 일이 없게 한다.
    expect(rustSources.length).toBeGreaterThan(10);
    expect(audioSources.length).toBeGreaterThan(3);
    expect(transcriptionSources.length).toBeGreaterThan(3);
    expect(domainSources.length).toBeGreaterThan(0);
    expect(outboundSources.length).toBeGreaterThan(0);
    expect(frontendProduction.length).toBeGreaterThan(10);
    for (const file of [COLLAPSE_MODULE, LEVEL_MODULE, CAPTURE_MODULE, RUN_MODULE]) {
      expect(rustSources, `${file}이 사라졌다`).toContain(file);
    }
    expect(frontendProduction).toContain(RECORDING_VIEW_MODULE);
    expect(frontendProduction).toContain(TYPES_MODULE);
  });
});

/**
 * `pub struct <name> { ... }` 블록의 필드를 `이름: 타입`으로 나열한다.
 *
 * 필드를 **전부 나열해 비교하는 것**이 목적이다. "금지된 이름이 없다"는 새 이름이 생기면
 * 빠져나가지만, 집합을 통째로 고정하면 무엇이 늘어도 여기서 먼저 드러난다.
 */
function rustStructFields(production: string, name: string): string[] {
  const start = production.indexOf(`pub struct ${name} {`);
  expect(start, `${name}을 찾지 못했다`).toBeGreaterThanOrEqual(0);
  const body = production.slice(start, production.indexOf('\n}', start));
  return [...body.matchAll(/pub (\w+):\s*([^,\n]+),/g)].map(
    (matched) => `${matched[1]}: ${matched[2].trim()}`,
  );
}

/** `export interface <name> { ... }` 블록의 필드. 위와 같은 이유로 집합을 통째로 본다. */
function tsInterfaceFields(source: string, name: string): string[] {
  const stripped = withoutComments(source);
  const start = stripped.indexOf(`export interface ${name} {`);
  expect(start, `${name}을 찾지 못했다`).toBeGreaterThanOrEqual(0);
  const body = stripped.slice(start, stripped.indexOf('\n}', start));
  return [...body.matchAll(/readonly (\w+):\s*([^;]+);/g)].map(
    (matched) => `${matched[1]}: ${matched[2].trim()}`,
  );
}

/**
 * (a) 레벨을 알리는 경로가 오디오 샘플 자체를 내보내지 않는다 (INV-6 · §12).
 *
 * INV-6은 오디오가 **기기 밖으로** 나가는 길을 막는다. 입력 레벨은 그 선을 앱 안의 IPC
 * 경계에서도 지킨다 — 화면이 오디오를 받을 이유가 없고, 한 번 webview로 넘어간 것은
 * 그 뒤에 어디로 가는지 이 경계가 알 수 없다.
 *
 * 나가는 것은 **수치 둘 · 판정 하나 · 사람이 읽는 문장 하나**뿐이며, 그 사실을 필드 집합을
 * 통째로 비교해 고정한다. 파형 하나, 샘플 배열 하나가 늘어나는 것만으로 이 선이 무너진다.
 */
describe('(a) 레벨을 알리는 경로에 오디오 샘플이 실리지 않는다 (INV-6 · §12)', () => {
  it('상태 payload에 실리는 것은 상태 · 경과 시간 · 레벨 하나뿐이다', () => {
    expect(rustStructFields(payloadProduction, 'SessionStatusPayload')).toEqual([
      'state: String',
      'elapsed_ms: i64',
      'elapsed_label: String',
      'level: Option<InputLevelPayload>',
    ]);
  });

  it('레벨 payload에 실리는 것은 수치 셋 · 판정 하나 · 문장 하나뿐이다', () => {
    // 2026-09-08에 `meter_fill`이 늘었다. **샘플이 아니라 이미 판정된 수치를 길이로 옮긴
    // 값**이며, 그것을 backend가 내는 이유는 판정 구간을 아는 자리가 하나여야 하기
    // 때문이다 (INV-9). 오디오는 여전히 이 경계를 지나지 않는다 — 아래 검사가 그것을 본다.
    expect(rustStructFields(payloadProduction, 'InputLevelPayload')).toEqual([
      'average_dbfs: f64',
      'peak_dbfs: f64',
      'verdict: String',
      'meter_fill: f64',
      'message: String',
    ]);
  });

  it('command payload 어디에도 오디오 샘플을 담을 타입이 없다', () => {
    // 레벨 payload만이 아니라 **경계 전체**를 본다. 다른 payload에 샘플이 실리면 레벨 쪽이
    // 깨끗한 것은 아무 의미가 없다.
    const sampleShapes = [
      /Vec\s*<\s*i16\s*>/,
      /Vec\s*<\s*f32\s*>/,
      /Vec\s*<\s*u8\s*>/,
      /\[\s*i16\s*\]/,
      /\bsamples\b/,
      /\bwaveform\b/i,
      /\bspectrum\b/i,
      /\bpcm\b/i,
    ];

    for (const shape of sampleShapes) {
      expect(payloadProduction, `payload에 오디오 샘플이 실린다: ${shape}`).not.toMatch(shape);
    }
  });

  it('frontend 계약에도 샘플이 올 자리가 없다', () => {
    expect(tsInterfaceFields(typesSource, 'InputLevel')).toEqual([
      'averageDbfs: number',
      'peakDbfs: number',
      'verdict: InputLevelVerdict',
      'meterFill: number',
      'message: string',
    ]);
    expect(tsInterfaceFields(typesSource, 'SessionStatus')).toEqual([
      'state: SessionState',
      'elapsedMs: number',
      'elapsedLabel: string',
      'level: InputLevel | null',
    ]);
  });

  it('레벨을 재는 자리에 기기 밖으로 나가는 통로가 없다', () => {
    // 레벨은 파일에 쓰이는 샘플 옆에서 만들어진다 (ADR-0003 §16.2). 그 자리에 요청을 만드는
    // 수단이 하나라도 생기면, 오디오에 가장 가까운 코드가 곧 네트워크에 닿는 코드가 된다.
    const outbound = [/\breqwest\b/, /\bureq\b/, /TcpStream/, /https?:\/\//, /\bkeyring\b/];

    for (const file of audioSources) {
      const production = productionRust(read(file));
      for (const shape of outbound) {
        expect(production, `${file}이 밖으로 나간다`).not.toMatch(shape);
      }
    }
  });

  it('밖으로 나가는 요청을 만드는 자리가 입력 레벨을 알지 않는다', () => {
    // 반대 방향도 막는다. AI · Notion · 내보내기 쪽이 레벨 값을 알기 시작하면 그것을 문서에
    // 싣는 한 줄까지의 거리가 한 줄이 된다 — 수치는 오디오가 아니지만, 그 경로가 열리는
    // 순간 다음에 실릴 것이 무엇인지는 이 경계가 정하지 못한다.
    const levelKnowledge = /InputLevel|LevelReading|average_dbfs|peak_dbfs|audio::level/;

    for (const file of outboundSources) {
      expect(productionRust(read(file)), `${file}이 입력 레벨을 안다`).not.toMatch(levelKnowledge);
    }
  });

  it('화면의 레벨 표시가 오디오 파일에 닿지 않는다', () => {
    // 레벨을 그리는 자리가 오디오 파일의 경로나 재생 주소를 알 이유가 없다. 알기 시작하면
    // "레벨을 보여 주기 위해" 오디오를 webview로 끌어오는 길이 열린다.
    const audioAccess = [/audioPath/, /convertFileSrc/, /new Audio\s*\(/, /asset\s*:/i];
    const source = withoutComments(read(RECORDING_VIEW_MODULE));

    for (const shape of audioAccess) {
      expect(source, `recordingView가 오디오에 닿는다: ${shape}`).not.toMatch(shape);
    }
  });
});

/**
 * (b) 붕괴 판정 규칙은 한 자리에만 있다 (ADR-0007 §18.3).
 *
 * `run.rs`가 `collapse::assess(...)`를 **부르는 것은 복제가 아니다.** 복제는 임계값이나
 * 비율 계산이 두 번째 자리에 생기는 것이며, 그때부터 한쪽만 고쳐지는 날이 오고 저장을 막는
 * 판단과 사람에게 보이는 수치가 서로 다른 규칙에서 나오게 된다.
 */
describe('(b) 붕괴 판정 규칙이 한 자리에만 있다 (ADR-0007 §18.3)', () => {
  it('임계값 상수를 정의하는 파일이 정확히 하나다', () => {
    const definers = rustSources.filter((file) =>
      /const\s+(MINIMUM_SENTENCES_TO_JUDGE|COLLAPSED_\w+)/.test(productionRust(read(file))),
    );

    expect(definers).toEqual([COLLAPSE_MODULE]);
  });

  it('그 파일이 세 임계값을 각각 한 번씩만 정의한다', () => {
    const production = productionRust(collapseSource);
    const definitions = [
      'pub const MINIMUM_SENTENCES_TO_JUDGE: usize = 20;',
      'pub const COLLAPSED_UNIQUE_RATIO_AT_OR_BELOW: f64 = 0.20;',
      'pub const COLLAPSED_TOP_REPEAT_SHARE_AT_OR_ABOVE: f64 = 0.50;',
    ];

    for (const definition of definitions) {
      expect(production.split(definition).length - 1, `${definition}`).toBe(1);
    }
  });

  it('세는 것도 나누는 것도 그 파일 하나에서만 일어난다', () => {
    // 판정을 만드는 함수들이다. 두 번째 자리에 같은 이름이 생기면 부르는 쪽마다 다른 규칙을
    // 쓸 수 있게 된다.
    const makers = rustSources.filter((file) =>
      /fn\s+(assess|sentence_key|unique_ratio|top_repeat_share)\s*\(/.test(
        productionRust(read(file)),
      ),
    );

    expect(makers).toEqual([COLLAPSE_MODULE]);
  });

  it('전사 실행 경로와 엔진에 임계값이 복제돼 있지 않다', () => {
    // 판정에 걸리는 자리는 `run.rs`의 저장 직전 하나이며, 그 자리는 **판정을 보고 저장을
    // 그만둘 뿐** 스스로 판정하지 않는다 (ADR-0007 §18.5). 임계값을 다시 적는 것은 물론이고,
    // 상수를 빌려다 자기 자리에서 비교하는 것도 규칙의 두 번째 벌이다.
    for (const file of transcriptionSources.filter((file) => file !== COLLAPSE_MODULE)) {
      const production = withoutReExports(productionRust(read(file)));
      for (const shape of COLLAPSE_RULE) {
        expect(production, `${file}에 판정 규칙이 복제됐다: ${shape}`).not.toMatch(shape);
      }
    }
  });

  it('run.rs가 판정을 스스로 하지 않고 그 모듈에 물어본다', () => {
    // 위 검사가 "숫자가 없다"로 통과하는 것만으로는 부족하다. 판정이 **실제로 일어나는지**를
    // 함께 못박지 않으면, 판정 자체가 사라져도 위 검사는 조용히 통과한다.
    const production = productionRust(read(RUN_MODULE));

    expect(production).toMatch(/collapse::assess\(/);
    expect(production).toMatch(/CollapseVerdict::Collapsed/);
    expect(production).toMatch(/CollapseVerdict::Empty/);
  });

  it('화면 쪽에 붕괴 판정 규칙이 없다', () => {
    // 화면이 받는 것은 §13의 실패 하나와 그 문장이다. 화면이 segment를 세어 다시 판정하기
    // 시작하면, 저장을 막은 판단과 사람이 보는 판단이 서로 다른 자리에서 나온다.
    //
    // **사람이 읽는 문장을 담은 상수는 규칙이 아니다** — `TRANSCRIPTION_COLLAPSED_NOTICE`는
    // 붕괴를 만났을 때 무엇을 하면 되는지를 말할 뿐 무엇이 붕괴인지 정하지 않는다.
    const collapseRule = [/uniqueRatio/i, /topRepeatShare/i, /sentenceCount/i, ...COLLAPSE_RULE];

    for (const file of frontendSources) {
      const source = withoutComments(read(file));
      for (const shape of collapseRule) {
        expect(source, `${file}에 붕괴 판정 규칙이 있다: ${shape}`).not.toMatch(shape);
      }
    }
  });
});

/**
 * (c) 화면 쪽에 dBFS 계산도 판정 임계값도 없다 (ADR-0003 §16.3 · §16.4).
 *
 * `tests/screen-boundary.test.ts`가 같은 사실을 **금지된 모양**(로그 · 풀스케일 · `-36` ·
 * `-60`)으로 지킨다. 여기서 보는 것은 그 반대쪽이다 — 레벨 값을 다루는 자리가 몇 개이고,
 * 그 자리가 수치에 아예 손대지 않으며, 두 쪽의 판정 갈래가 같은 셋이라는 것.
 *
 * 금지 목록은 새 표현이 생기면 빠져나간다. 값을 만지는 자리 자체를 세는 검사는 그러지 않는다.
 */
describe('(c) 화면에 dBFS 계산도 판정 임계값도 없다 (ADR-0003 §16.4)', () => {
  it('레벨 값을 직접 읽는 제품 모듈이 src/ 아래에 하나뿐이다', () => {
    const readers = frontendProduction.filter((file) =>
      /\.level\b/.test(withoutComments(read(file))),
    );

    expect(readers).toEqual([RECORDING_VIEW_MODULE]);
  });

  it('그 모듈이 dBFS 수치를 아예 만지지 않는다', () => {
    // 갈래(`verdict`)와 문장(`message`)만 쓴다. 수치가 이 모듈에 들어오는 순간 그것을
    // 비교하거나 다시 적는 코드가 생길 자리가 만들어진다.
    const source = withoutComments(read(RECORDING_VIEW_MODULE));

    expect(source, 'recordingView가 dBFS를 안다').not.toMatch(/dbfs/i);
    expect(source).toMatch(/kind:\s*level\.verdict/);
    expect(source).toMatch(/text:\s*level\.message/);
  });

  it('화면이 스스로 만드는 문장에는 수치도 단위도 없다', () => {
    // 화면에도 문장이 둘 있다 — 값이 아직 없을 때의 자리와 정지 전 경고다. 그 둘은 *지금
    // 무엇을 해야 하는가*만 말한다. 거기에 숫자가 들어가는 순간 화면이 자기 수치를 갖게 되고,
    // backend가 만든 문장과 나란히 놓여 서로 다른 값을 말할 수 있게 된다.
    const source = read(RECORDING_VIEW_MODULE);
    const sentences = [
      ...source.matchAll(/export const (UNKNOWN_LEVEL_TEXT|WEAK_LEVEL_WARNING) =\s*([\s\S]*?);/g),
    ].map((matched) => matched[2]);

    expect(sentences, '화면이 가진 두 문장을 찾지 못했다').toHaveLength(2);
    for (const sentence of sentences) {
      expect(sentence, `문장에 단위가 있다: ${sentence}`).not.toMatch(/dbfs/i);
      expect(sentence, `문장에 수치가 있다: ${sentence}`).not.toMatch(/\d/);
    }
  });

  it('판정 구간을 정하는 상수가 Rust의 한 파일에만 있다', () => {
    const definers = rustSources.filter((file) =>
      /const\s+(FULL_SCALE|USABLE_AT_OR_ABOVE_DBFS|SILENT_BELOW_DBFS|FLOOR_DBFS)/.test(
        productionRust(read(file)),
      ),
    );

    expect(definers).toEqual([LEVEL_MODULE]);
  });

  it('backend의 판정 이름 셋과 frontend의 갈래 셋이 정확히 같다', () => {
    // 한쪽에 갈래가 하나 늘고 다른 쪽이 모르면, 화면은 알 수 없는 값을 받고도 아무 말도
    // 하지 않는다. 두 목록을 통째로 맞춰 그 어긋남이 조용히 생기지 않게 한다.
    const wire = [...payloadProduction.matchAll(/LevelVerdict::\w+\s*=>\s*"(\w+)"/g)].map(
      (matched) => matched[1],
    );
    expect(wire, 'Rust가 보내는 판정 이름을 찾지 못했다').toHaveLength(3);

    const union = typesSource.match(/export type InputLevelVerdict =([^;]+);/)?.[1] ?? '';
    const declared = [...union.matchAll(/'(\w+)'/g)].map((matched) => matched[1]);

    expect(declared.sort()).toEqual([...wire].sort());
  });

  it('src/ 아래에 RMS나 dBFS를 만드는 산술이 없다', () => {
    // 길이 포맷의 예외(`transcriptView.ts`의 segment timestamp)는 나눗셈과 반올림만 쓴다.
    // 로그 · 거듭제곱 · 제곱근은 그 예외에도 필요 없는 모양이며, 레벨 환산의 모양 그 자체다.
    const levelArithmetic = /Math\s*\.\s*(log10|log2|log|pow|sqrt|hypot)\s*\(/;

    for (const file of frontendSources) {
      expect(withoutComments(read(file)), `${file}에 레벨 산술이 있다`).not.toMatch(
        levelArithmetic,
      );
    }
  });
});

/** `fn <signature>` 뒤부터 다음 최상위 항목 앞까지 — 그 함수의 본문만 남긴다. */
function rustFnBody(production: string, signature: string): string {
  const start = production.indexOf(signature);
  expect(start, `${signature}를 찾지 못했다`).toBeGreaterThanOrEqual(0);
  const rest = production.slice(start + signature.length);
  const end = rest.search(/\n(?:pub )?(?:struct|enum|fn|impl|trait|const) /);
  return rest.slice(0, end < 0 ? rest.length : end);
}

/**
 * (d) 캡처 경로가 샘플을 바꾸지 않는다 (ADR-0003 §16.5 · phase-prompt/05.7).
 *
 * 이 Phase는 **레벨을 알릴 뿐 고치지 않는다.** 마이크 게인을 앱이 조정할지는 성공 기준 3의
 * 측정 결과가 나온 뒤에 내릴 결정이며, 그전에 정규화가 슬쩍 들어가면 두 가지가 무너진다 —
 * 사람이 보는 레벨이 파일에 남은 소리와 달라지고, 측정 자체가 무의미해진다.
 */
describe('(d) 캡처 경로가 샘플을 바꾸지 않는다 (ADR-0003 §16.5)', () => {
  it('파일로 가는 덩어리와 레벨로 가는 덩어리가 받은 그대로의 같은 값이다', () => {
    // 통로에서 꺼낸 덩어리가 **이름 그대로** 파일과 레벨로 간다. 사이에 어떤 변환도 없고,
    // 그 이름이 등장하는 자리도 그 셋뿐이다 — 네 번째 자리가 생기면 여기서 드러난다.
    const drain = rustFnBody(productionRust(captureSource), 'fn drain(');

    const received = drain.match(/Packet::Samples\((\w+)\)\s+if/)?.[1];
    expect(received, '통로에서 꺼낸 덩어리에 이름이 있어야 한다').toBeTruthy();

    expect(drain, '파일로 가는 것이 받은 덩어리가 아니다').toContain(`file.write(&${received})`);
    expect(drain, '레벨로 가는 것이 받은 덩어리가 아니다').toContain(`level.push(&${received})`);
    expect(
      drain.match(new RegExp(`\\b${received}\\b`, 'g')),
      '덩어리가 그 셋 말고 다른 자리에서도 쓰인다',
    ).toHaveLength(3);
  });

  it('캡처 경로 어디에도 게인도 정규화도 리샘플도 없다', () => {
    // 전사 입력(`transcription/audio_input.rs`)은 리샘플과 다운믹스를 한다. 그것은 **파생
    // 버퍼**이며 원본 파일이 아니다 (ADR-0007 §9.2 · INV-1). 녹음이 파일에 닿는 이 경로에는
    // 그런 변환이 하나도 없다는 것이 이 Phase가 지킨 선이다.
    //
    // `gain`은 **낱말 경계로 찾지 않는다.** `apply_gain`처럼 이름의 뒤쪽에 붙는 것이 오히려
    // 흔한 모양이고, 그러면 `\bgain\b`는 그것을 놓친다. 앞이 글자가 아닌 자리만 보면
    // `apply_gain`과 `gain_factor`는 잡히고 `again` 같은 낱말은 잡히지 않는다.
    const changingShapes = [
      /(?<![a-z])gain/i,
      /normaliz/i,
      /amplif/i,
      /boost/i,
      /\brubato\b/,
      /resampl/i,
    ];

    for (const file of audioSources) {
      const production = productionRust(read(file));
      for (const shape of changingShapes) {
        expect(production, `${file}이 샘플을 바꾼다: ${shape}`).not.toMatch(shape);
      }
    }
  });

  it('샘플을 바꿀 수 있는 자리가 캡처 경로 표면에 없다', () => {
    // 바꾸는 코드가 지금 없다는 것과 **바꿀 수 있는 자리가 없다**는 것은 다른 말이다.
    // 가변 슬라이스를 넘겨주는 함수가 하나 생기면 그다음 한 줄은 이 경계 밖에서 쓰인다.
    const mutableSamples = [/&mut\s*\[\s*i16\s*\]/, /&mut\s*Vec\s*<\s*i16\s*>/, /\.iter_mut\s*\(/];

    for (const file of audioSources) {
      const production = productionRust(read(file));
      for (const shape of mutableSamples) {
        expect(production, `${file}에 샘플을 바꿀 자리가 있다: ${shape}`).not.toMatch(shape);
      }
    }
  });

  it('쓰인 샘플을 파일에서 되읽어 대조하는 검사가 캡처 모듈에 남아 있다', () => {
    // 원문을 읽는 이 검사가 말할 수 있는 것은 "바꾸는 코드가 없다"까지다. 들어온 샘플이
    // 실제로 그대로 파일에 있다는 것은 **확정된 파일을 되읽어 비교해야** 알 수 있고, 그
    // 검사는 `capture.rs` 안에 있다 (`samples_in` · `drained`). 그것이 사라지면 이 불변을
    // 동작으로 판정하는 자리가 저장소에서 없어진 것이므로 여기서 먼저 알린다.
    const tests = captureSource.slice(captureSource.indexOf('#[cfg(test)]'));

    expect(tests, '캡처 모듈에 테스트가 없다').not.toBe('');
    expect(tests, '확정된 파일을 되읽는 자리가 없다').toContain('hound::WavReader::open');
    expect(tests, '읽어 온 샘플을 그대로 대조하는 단언이 없다').toMatch(/assert_eq!\(\s*samples,/);
  });
});

/**
 * (e) 새 벤더 고유 개념이 생기지 않았다 (INV-9).
 *
 * 벤더는 바뀐다. 바뀔 때 흔들리는 것이 adapter 하나여야 하며, 그러려면 core/domain ·
 * payload · frontend 타입에 특정 제공자의 이름도, **그 제공자가 만들어 낸 문자열도** 없어야
 * 한다. 붕괴 판정이 후자에서 특히 위태롭다 — 알려진 환각 문장 목록을 갖는 것이 가장 쉬운
 * 구현이고, 그 순간 규칙은 모델이 바뀌는 날 통째로 쓸모없어진다.
 */
describe('(e) 새 벤더 고유 개념이 생기지 않았다 (INV-9)', () => {
  /** `tests/ipc-boundary.test.ts`가 쓰는 것과 **같은 목록**이다. 아래에서 그 사실을 확인한다. */
  const VENDORS = /ollama|llama|openai|gpt-|anthropic|claude|gemini|groq|mistral|huggingface/i;

  it('core/domain에 벤더 고유 이름이 없다', () => {
    for (const file of domainSources) {
      expect(productionRust(read(file)), `${file}에 벤더 이름이 있다`).not.toMatch(VENDORS);
    }
  });

  it('이 Phase가 만든 두 순수 모듈이 어떤 벤더도 알지 않는다', () => {
    for (const file of [COLLAPSE_MODULE, LEVEL_MODULE]) {
      expect(productionRust(read(file)), `${file}에 벤더 이름이 있다`).not.toMatch(VENDORS);
    }
  });

  it('이 Phase가 payload와 frontend 타입에 더한 것이 벤더 중립이다', () => {
    expect(payloadProduction).not.toMatch(VENDORS);
    expect(withoutComments(typesSource)).not.toMatch(VENDORS);
    // 더해진 것이 실제로 그 자리에 있다 — 빈 자리를 보고 통과하지 않게 한다.
    expect(payloadSource).toContain('pub struct InputLevelPayload');
    expect(typesSource).toContain('export interface InputLevel');
  });

  it('붕괴 판정이 특정 모델이 낸 문자열을 알지 않는다', () => {
    // 2026-09-07의 `한글자막 by 한효정`은 그 모델의 학습 데이터에서 나온 잔재다
    // (ADR-0007 §18.1). 판정은 그 문장을 **모른 채** 고유 비율과 반복 점유율만 본다.
    const HALLUCINATION = '한글자막 by 한효정';

    expect(productionRust(collapseSource), '판정이 환각 문장 목록을 갖고 있다').not.toContain(
      HALLUCINATION,
    );
    expect(productionRust(read(RUN_MODULE))).not.toContain(HALLUCINATION);
    expect(payloadProduction).not.toContain(HALLUCINATION);
    for (const file of frontendSources) {
      expect(read(file), `${file}에 모델이 낸 문자열이 있다`).not.toContain(HALLUCINATION);
    }

    // 그런데도 그 문장은 **테스트에는 실재한다** — 실제 관측이 값으로 고정돼 있다는 뜻이며,
    // 위 검사가 "그 문자열이 저장소에서 사라져서" 통과하는 것이 아니라는 근거다.
    expect(collapseSource, '관측된 붕괴가 값으로 고정돼 있지 않다').toContain(HALLUCINATION);
  });

  it('INV-9를 지키는 기존 검사가 그대로 남아 있다', () => {
    // 이 Phase의 요구는 *"`tests/ipc-boundary.test.ts`의 검사가 그대로 통과한다"*이다.
    // 통과 여부는 그 파일이 실행되며 판정하지만, **그 검사가 약해지거나 사라지는 것**은
    // 그 파일 자신이 알릴 수 없다. 그래서 여기서 본다.
    const ipcBoundary = read(path('./ipc-boundary.test.ts'));

    expect(ipcBoundary).toContain("describe('wire 계약에 벤더가 없다 (INV-9)'");
    expect(ipcBoundary).toContain('command payload 타입에 벤더 고유 이름이 없다');
    expect(ipcBoundary).toContain('frontend 타입에 벤더 고유 이름이 없다');

    // 그리고 그쪽의 목록이 위에서 쓴 것과 **글자 그대로 같다.** 한쪽이 느슨해지면 두 검사가
    // 서로 다른 것을 보게 되고, 그때부터 이 파일의 통과는 아무것도 보장하지 않는다.
    const theirs = ipcBoundary.match(/const VENDORS = (\/[^\n]+\/i);/)?.[1];
    expect(theirs, 'ipc-boundary의 벤더 목록을 찾지 못했다').toBe(VENDORS.toString());
  });
});
