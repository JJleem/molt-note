// 전사 청크 분할 경계 테스트 (Phase 5.8).
//
// 검사 대상은 동작이 아니라 **소스 전체에 대한 규칙**이다. 파일 하나가 새로 생기거나 상수
// 하나가 두 번째 자리에 적히는 것만으로 조용히 깨질 수 있는 것들이므로, 개별 모듈 옆의
// 테스트가 아니라 원문을 읽는 이 자리에서 본다 (`tests/level-and-collapse-boundary.test.ts` ·
// `tests/screen-boundary.test.ts`의 선례 — Rust의 `include_str!` 검사는 자기 crate 안쪽만
// 보고, `src/`는 브라우저 코드로 타입 검사되어 node:fs를 쓸 수 없다).
//
// 이 Phase가 세운 여섯 가지를 고정한다.
//
//   (a) 청크 분할 · 겹침 · 오프셋 · 반복 차단 규칙이 **순수 모듈 한 파일에만** 있고
//       `whisper.rs` · `run.rs` · `src/screens/`에 복제되어 있지 않다 (ADR-0007 §20.8).
//   (b) 그 순수 모듈이 파일시스템 · 데이터베이스 · 네트워크 · `whisper_rs`를 알지 않는다
//       (ADR-0007 §20.8 — `collapse.rs` · `parse.rs`가 세운 선례).
//   (c) 센티초 → 밀리초 계수가 여전히 `parse.rs` 한 자리뿐이다 (ADR-0007 §10 · §20.5).
//   (d) `run.rs`의 저장 직전 붕괴 판정 호출이 그대로 있다 — 그리고 **저장보다 앞에** 있다
//       (ADR-0007 §18.5 · §20.6.2 · `phase-prompt/05.8` Constraints).
//   (e) `Cargo.toml`에 새 의존성이 늘지 않았다 (`phase-prompt/05.8` Constraints).
//   (f) 벤더 고유 청킹 개념이 `domain` · `commands/payload.rs` · `src/ipc/types.ts`에 새로
//       생기지 않았다 — **밖에서 보면 전사 하나가 나올 뿐이다** (INV-9 · ADR-0007 §20.8).
//
// ## 왜 이것들이 한 파일에 함께 있는가
//
// 2026-09-05가 72.85분 오디오에서 고유 94.0%를 낸 것은 조건 셋을 함께 바꿨기 때문이고, 그중
// 제품에 없던 하나가 청크 분할이다 (`phase-prompt/05.8` P-2 · P-3). 그 하나를 제품으로 옮기면서
// **무엇을 얻고 무엇을 잃지 않는가**가 이 Phase의 구조다 — 규칙이 한 자리에 살고, 단위 변환의
// 자리가 늘지 않고, 안전망(§18의 붕괴 판정)이 그대로 남고, 청크라는 개념이 경계 밖으로 새어
// 나가지 않는다. 그 넷은 각각 다른 파일에서 깨질 수 있으므로 원문을 통째로 읽는 자리가 필요하다.
//
// ## 이 파일이 판정하지 **않는** 것
//
// **72분 오디오가 읽을 만한 한국어로 전사되는가는 여기서 판정되지 않는다.** 그것은
// `phase-prompt/05.8`의 Human Review 항목이며 사람이 앱으로 실행해 판정한다 — 자동 Gate는
// 실제 whisper도 모델 파일도 오디오도 요구하지 않는다 (PRODUCT-SPEC §18). 합쳐진 결과가
// 저장 경로에서 어떻게 행동하는지는 `src-tauri/tests/transcription_chunking.rs`가 값으로 본다.
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
 * (`src-tauri/src/transcription/mod.rs`가 청킹 모듈의 상수 셋을 그대로 내보낸다).
 * 여기서 찾는 것은 **같은 규칙의 두 번째 벌**이다.
 */
function withoutReExports(production: string): string {
  return production.replace(/pub use [\s\S]*?;/g, '');
}

const CHUNKING_MODULE = path('../src-tauri/src/transcription/chunking.rs');
const COLLAPSE_MODULE = path('../src-tauri/src/transcription/collapse.rs');
const PARSE_MODULE = path('../src-tauri/src/transcription/parse.rs');
const RUN_MODULE = path('../src-tauri/src/transcription/run.rs');
const WHISPER_MODULE = path('../src-tauri/src/transcription/whisper.rs');
const PAYLOAD_MODULE = path('../src-tauri/src/commands/payload.rs');
const TYPES_MODULE = path('../src/ipc/types.ts');
const CARGO_MANIFEST = path('../src-tauri/Cargo.toml');

const rustSources = sourceFiles(path('../src-tauri/src'), /\.rs$/);
const transcriptionSources = sourceFiles(path('../src-tauri/src/transcription'), /\.rs$/);
const commandSources = sourceFiles(path('../src-tauri/src/commands'), /\.rs$/);
const domainSources = sourceFiles(path('../src-tauri/src/domain'), /\.rs$/);
const screenSources = sourceFiles(path('../src/screens'), /\.(ts|tsx)$/);
const frontendSources = sourceFiles(path('../src'), /\.(ts|tsx)$/);

const chunkingProduction = productionRust(read(CHUNKING_MODULE));
const runProduction = productionRust(read(RUN_MODULE));
const whisperProduction = productionRust(read(WHISPER_MODULE));
const payloadProduction = productionRust(read(PAYLOAD_MODULE));
const typesProduction = withoutComments(read(TYPES_MODULE));
const manifest = read(CARGO_MANIFEST);

/**
 * 청킹 규칙의 **값과 이름** (ADR-0007 §20.8).
 *
 * 상수 이름은 `:`까지 붙여 찾는다 — 그래야 값을 **정의하는** 자리만 잡히고 그 값을 빌려다
 * 쓰는 자리는 잡히지 않는다. 숫자 둘은 규칙을 다시 적을 때 반드시 따라 나오는 모양이다
 * (`120 × 16,000 = 1,920,000` 프레임 · `120 × 100 = 12,000` 센티초).
 */
const CHUNK_RULE = [
  /CHUNK_SECONDS:/,
  /CHUNK_OVERLAP_FRAMES:/,
  /MAX_CONSECUTIVE_REPEATS:/,
  /CENTISECONDS_PER_SECOND:/,
  /1_920_000/,
  /\b12_000\b/,
];

describe('검사가 실제로 원문을 읽었다', () => {
  it('읽을 소스가 비어 있지 않다', () => {
    // 아래 모든 검사가 "목록이 비어서" 통과하는 일이 없게 한다.
    expect(rustSources.length).toBeGreaterThan(10);
    expect(transcriptionSources.length).toBeGreaterThan(5);
    expect(commandSources.length).toBeGreaterThan(0);
    expect(domainSources.length).toBeGreaterThan(0);
    expect(screenSources.length).toBeGreaterThan(10);
    expect(frontendSources.length).toBeGreaterThan(10);

    for (const file of [CHUNKING_MODULE, COLLAPSE_MODULE, PARSE_MODULE, RUN_MODULE, WHISPER_MODULE]) {
      expect(rustSources, `${file}이 사라졌다`).toContain(file);
    }
    expect(frontendSources).toContain(TYPES_MODULE);

    // 읽은 것이 실제로 그 모듈인지 확인한다 — 빈 문자열이면 아래 검사는 아무것도 막지 못한다.
    expect(chunkingProduction).toContain('pub fn plan(');
    expect(runProduction).toContain('pub fn transcribe(');
    expect(whisperProduction).toContain('impl TranscriptionEngine for WhisperEngine');
    expect(manifest).toContain('[dependencies]');
  });
});

/**
 * (a) 청킹 규칙은 순수 모듈 한 파일에만 있다 (ADR-0007 §20.8).
 *
 * `whisper.rs`가 `chunking::plan(...)` · `chunking::merge(...)`를 **부르는 것은 복제가 아니다.**
 * 복제는 청크 길이나 오프셋 계산이나 반복 차단 임계값이 두 번째 자리에 생기는 것이며, 그때부터
 * 한쪽만 고쳐지는 날이 온다 — 잘린 자리와 되돌린 자리가 서로 다른 값을 쓰면 timestamp가 전체
 * 시간축에서 조용히 어긋난다. `collapse.rs`가 임계값 셋에 대해 세운 선례 그대로다.
 */
describe('(a) 청킹 규칙이 한 자리에만 있다 (ADR-0007 §20.8)', () => {
  it('세 값을 정의하는 파일이 정확히 하나다', () => {
    const definers = rustSources.filter((file) =>
      /const\s+(CHUNK_SECONDS|CHUNK_OVERLAP_FRAMES|MAX_CONSECUTIVE_REPEATS|CENTISECONDS_PER_SECOND)/.test(
        productionRust(read(file)),
      ),
    );

    expect(definers).toEqual([CHUNKING_MODULE]);
  });

  it('그 파일이 네 값을 각각 한 번씩만 정의한다', () => {
    const definitions = [
      'pub const CHUNK_SECONDS: u32 = 120;',
      'pub const CHUNK_OVERLAP_FRAMES: usize = 0;',
      'pub const MAX_CONSECUTIVE_REPEATS: usize = 3;',
      'const CENTISECONDS_PER_SECOND: u32 = 100;',
    ];

    for (const definition of definitions) {
      expect(chunkingProduction.split(definition).length - 1, `${definition}`).toBe(1);
    }
  });

  it('자르는 것도 되돌리는 것도 차단하는 것도 그 파일 하나에서만 일어난다', () => {
    // 규칙을 만드는 함수들이다. 두 번째 자리에 같은 이름이 생기면 부르는 쪽마다 다른 규칙을
    // 쓸 수 있게 된다. `plan` · `merge`는 전사 밖에도 같은 이름이 있으므로
    // (`sync/run.rs`의 전송 계획) 전사 모듈 안에서 찾고, 청킹 고유의 이름은 저장소 전체에서 찾는다.
    const transcriptionMakers = transcriptionSources.filter((file) =>
      /fn\s+(plan|shift|merge)\s*\(/.test(withoutReExports(productionRust(read(file)))),
    );
    expect(transcriptionMakers).toEqual([CHUNKING_MODULE]);

    const chunkingMakers = rustSources.filter((file) =>
      /fn\s+(block_consecutive_repeats|offset_centiseconds|add_offset)\s*\(/.test(
        withoutReExports(productionRust(read(file))),
      ),
    );
    expect(chunkingMakers).toEqual([CHUNKING_MODULE]);
  });

  it('엔진 · 실행 경로 · 그 밖의 Rust 어디에도 청킹 값이 복제돼 있지 않다', () => {
    // `chunking.rs`의 단위 테스트가 같은 것을 보지만 그쪽은 `include_str!`로 **이름을 적어 둔
    // 다섯 파일**만 본다. 새 파일이 하나 생기면 그 검사는 그런 자리가 생겼다는 것조차 모른다.
    for (const file of rustSources.filter((file) => file !== CHUNKING_MODULE)) {
      const production = withoutReExports(productionRust(read(file)));
      for (const shape of CHUNK_RULE) {
        expect(production, `${file}에 청킹 규칙이 복제됐다: ${shape}`).not.toMatch(shape);
      }
    }
  });

  it('화면 쪽에 청킹 규칙이 없다', () => {
    // 화면이 받는 것은 전사 하나와 §13의 실패다. 화면이 청크를 세거나 오프셋을 더하기 시작하면
    // 저장된 시각과 사람이 보는 시각이 서로 다른 자리에서 나온다.
    //
    // **숫자로 찾지 않는다.** 12,000이라는 값은 화면 쪽에서 얼마든지 다른 뜻으로 나온다
    // (`copyView.test.ts`의 조각 크기가 실제로 그렇다). 청킹 규칙이 화면에 도달할 수 있는 길은
    // 이름뿐이며, 이름 없이 적힌 숫자는 규칙이 아니라 우연이다.
    const chunkRule = [
      /chunkSeconds/i,
      /chunkOverlap/i,
      /chunkLength/i,
      /consecutiveRepeat/i,
      /offsetCentiseconds/i,
      /centisecond/i,
      /CHUNK_SECONDS/,
      /CHUNK_OVERLAP_FRAMES/,
      /MAX_CONSECUTIVE_REPEATS/,
    ];

    for (const file of frontendSources) {
      const source = withoutComments(read(file));
      for (const shape of chunkRule) {
        expect(source, `${file}에 청킹 규칙이 있다: ${shape}`).not.toMatch(shape);
      }
    }
  });

  it('엔진이 스스로 자르지 않고 그 모듈에 물어본다', () => {
    // 위 검사가 "숫자가 없다"로 통과하는 것만으로는 부족하다. 청킹이 **실제로 일어나는지**를
    // 함께 못박지 않으면, 청크 분할 자체가 사라져도 위 검사는 조용히 통과한다 (ADR-0007 §20.8).
    expect(whisperProduction).toMatch(/chunking::plan\(/);
    expect(whisperProduction).toMatch(/chunking::merge\(/);
    expect(whisperProduction, '자르는 구간을 엔진이 스스로 짓는다').toMatch(/chunk\.range\(\)/);
    expect(
      whisperProduction,
      '오프셋을 엔진이 스스로 계산한다',
    ).toMatch(/offset_centiseconds:\s*chunk\.offset_centiseconds/);
  });

  it('전사 실행 경로가 청크라는 것을 아예 알지 않는다', () => {
    // 청킹은 `TranscriptionEngine` 구현 **안쪽**의 일이다 (`phase-prompt/05.8` Constraints).
    // `run.rs`가 청크를 알기 시작하면 밖으로 나가는 계약이 전사 하나가 아니게 된다.
    for (const shape of [/chunking/, /AudioChunk/, /ChunkTranscription/, /RepeatBlocking/]) {
      expect(runProduction, `run.rs가 청크를 안다: ${shape}`).not.toMatch(shape);
    }
  });
});

/**
 * (b) 그 순수 모듈이 바깥 세계를 모른다 (ADR-0007 §20.8).
 *
 * 금지 목록은 새 표현이 생기면 빠져나간다. 그래서 **들어오는 것 전부를 통째로 고정한다** —
 * `use` 하나가 늘어나는 순간 여기서 드러나며, 그것이 이 모듈을 실제 whisper도 모델도 없이
 * 값으로 검증할 수 있게 하는 유일한 근거다 (PRODUCT-SPEC §18).
 */
describe('(b) 청킹 모듈이 바깥 세계를 모른다 (ADR-0007 §20.8)', () => {
  it('이 모듈에 들어오는 것이 값 넷뿐이다', () => {
    const imports = [...chunkingProduction.matchAll(/^use ([^;]+);/gm)].map((matched) =>
      matched[1].replace(/\s+/g, ' '),
    );

    expect(imports).toEqual([
      'std::ops::Range',
      'crate::domain::{Failure, FailureKind}',
      'super::collapse::sentence_key',
      'super::parse::{RawSegment, RawTranscription, TranscriptSegment}',
    ]);
  });

  it('문장 비교 규칙의 두 번째 정의를 만들지 않고 판정 모듈의 것을 부른다', () => {
    // `use super::collapse::sentence_key`가 있다는 것과 **그것을 쓴다**는 것은 다른 말이다
    // (ADR-0007 §20.6.1 — "두 번째 정의를 만들지 않는다. 그 함수를 그대로 부른다").
    expect(chunkingProduction).toMatch(/sentence_key\(&segment\.text\)/);
    expect(chunkingProduction, '문장 정규화를 다시 구현했다').not.toMatch(
      /split_whitespace|to_lowercase|trim\(\)/,
    );
  });

  it('경로 · 저장소 · 네트워크 · 시계 · 엔진 타입이 이 모듈에 없다', () => {
    const outsideWorld = [
      /std::fs/,
      /std::process/,
      /Command::new/,
      /rusqlite/,
      /whisper_rs/,
      /WhisperContext/,
      /reqwest/,
      /\bureq\b/,
      /\bkeyring\b/,
      /https?:\/\//,
      /Instant::now/,
      /SystemTime::now/,
      /crate::db/,
      /crate::platform/,
      /crate::notion/,
      /crate::ai/,
    ];

    for (const shape of outsideWorld) {
      expect(chunkingProduction, `청킹 모듈에 바깥 세계가 들어왔다: ${shape}`).not.toMatch(shape);
    }
  });

  it('설정도 언어 선택도 이 모듈에 도달하지 않는다', () => {
    // 설정 값이 `language`에 베껴 들어갈 경로 자체를 만들지 않는다 (ADR-0007 §20.7 · §17.1.4-3).
    for (const shape of [/LanguageChoice/, /Settings/, /ModelChoice/, /ModelFile/]) {
      expect(chunkingProduction, `청킹 모듈이 설정을 안다: ${shape}`).not.toMatch(shape);
    }
  });
});

/**
 * (c) 센티초 → 밀리초 계수는 여전히 `parse.rs` 한 자리뿐이다 (ADR-0007 §10 · §20.5).
 *
 * 청크 오프셋을 더하는 일은 **같은 단위끼리의 덧셈**이므로 단위 변환이 아니다. 그 구분이
 * 무너지면 §10이 한 자리로 묶어 둔 것이 두 자리가 되고, 저장되는 시각이 어느 쪽 계수로
 * 만들어졌는지 알 수 없게 된다 — 그리고 틀린 시각은 영구히 저장된다 (INV-2).
 */
describe('(c) 단위 변환의 자리가 늘지 않았다 (ADR-0007 §10 · §20.5)', () => {
  it('계수를 정의하는 파일이 여전히 하나다', () => {
    const definers = rustSources.filter((file) =>
      /const\s+MILLISECONDS_PER_CENTISECOND/.test(productionRust(read(file))),
    );

    expect(definers).toEqual([PARSE_MODULE]);
    expect(productionRust(read(PARSE_MODULE)).split('const MILLISECONDS_PER_CENTISECOND: i64 = 10;').length - 1).toBe(1);
  });

  it('청킹 모듈은 밀리초라는 단위를 아예 알지 않는다', () => {
    // 오프셋도 그 합도 **원시 센티초**다 (ADR-0007 §20.5). 이 모듈이 밀리초를 알기 시작하면
    // 어느 값이 원시이고 어느 값이 정규화된 값인지가 코드에서 사라진다.
    for (const shape of [/millisecond/i, /\bstart_ms\b/, /\bend_ms\b/, /MILLISECONDS_PER/]) {
      expect(chunkingProduction, `청킹 모듈이 밀리초를 안다: ${shape}`).not.toMatch(shape);
    }
  });

  it('엔진은 청크가 낸 원시 값을 옮기기만 한다', () => {
    // `whisper.rs`가 timestamp에 손을 대면 단위 변환의 자리가 두 곳이 된다 (ADR-0007 §20.5).
    // 넘기는 것은 엔진이 준 값 그대로여야 한다.
    expect(whisperProduction).toMatch(/start_centiseconds:\s*segment\.start_timestamp\(\)/);
    expect(whisperProduction).toMatch(/end_centiseconds:\s*segment\.end_timestamp\(\)/);
    for (const shape of [/millisecond/i, /\bstart_ms\b/, /\bend_ms\b/]) {
      expect(whisperProduction, `엔진이 밀리초를 안다: ${shape}`).not.toMatch(shape);
    }
  });
});

/**
 * (d) 저장 직전의 붕괴 판정이 그대로 있다 (ADR-0007 §18.5 · §20.6.2).
 *
 * `phase-prompt/05.8` Constraints: *"붕괴 판정을 우회하지 않는다. 청킹이 들어와도 저장 직전
 * 검사는 그대로 통과해야 한다."* 그 검사가 **저장보다 앞에 있다는 것**까지가 규칙이다 —
 * 순서가 뒤집히면 쓸 수 없는 Transcript가 먼저 저장되고, 한 번 저장된 것은 지울 수 없다 (INV-2).
 */
describe('(d) 저장 직전의 붕괴 판정이 그대로 있다 (ADR-0007 §18.5)', () => {
  it('run.rs가 판정을 스스로 하지 않고 그 모듈에 물어본다', () => {
    expect(runProduction).toMatch(/collapse::assess\(/);
    expect(runProduction).toMatch(/CollapseVerdict::Collapsed/);
    expect(runProduction).toMatch(/CollapseVerdict::Empty/);
  });

  it('그 물음이 Transcript를 저장하는 자리보다 앞에 있다', () => {
    const asked = runProduction.indexOf('collapse::assess(');
    const stored = runProduction.indexOf('store::append_transcript(');

    expect(asked, 'collapse::assess를 찾지 못했다').toBeGreaterThanOrEqual(0);
    expect(stored, 'append_transcript를 찾지 못했다').toBeGreaterThanOrEqual(0);
    expect(asked, '판정이 저장 뒤로 밀렸다').toBeLessThan(stored);
  });

  it('붕괴 판정 규칙이 청킹 모듈로 새어 나가지 않았다', () => {
    // 차단은 판정을 대체하지 않는다 (ADR-0007 §20.6.2). 청킹 모듈이 임계값을 알기 시작하면
    // 차단이 판정을 겸할 수 있게 되고, 그 순간 §18이 세는 증거를 차단이 먼저 지운다.
    const collapseRule = [
      /MINIMUM_SENTENCES_TO_JUDGE/,
      /COLLAPSED_UNIQUE_RATIO_AT_OR_BELOW/,
      /COLLAPSED_TOP_REPEAT_SHARE_AT_OR_ABOVE/,
      /CollapseVerdict/,
      /\b0\.20?\b/,
      /\b0\.50?\b/,
    ];

    for (const shape of collapseRule) {
      expect(chunkingProduction, `청킹 모듈에 판정 규칙이 있다: ${shape}`).not.toMatch(shape);
    }
  });
});

/**
 * (e) 새 의존성이 늘지 않았다 (`phase-prompt/05.8` Constraints).
 *
 * *"새 의존성을 들이지 않는다. 9/5 실험은 새 crate 없이 이 결과를 냈다."* 이름 목록을 통째로
 * 고정한다 — "금지된 crate가 없다"는 새 이름이 생기면 빠져나가지만, 집합을 고정하면 무엇이
 * 늘어도 여기서 먼저 드러난다.
 */
describe('(e) 새 의존성이 늘지 않았다 (phase-prompt/05.8 Constraints)', () => {
  /** `[<section>]` 아래의 `<name> = ...` 이름들. 주석과 빈 줄은 건너뛴다. */
  function dependencyNames(section: string): string[] {
    const start = manifest.indexOf(`[${section}]`);
    expect(start, `[${section}]을 찾지 못했다`).toBeGreaterThanOrEqual(0);
    const rest = manifest.slice(start + `[${section}]`.length);
    const end = rest.indexOf('\n[');
    const body = rest.slice(0, end < 0 ? rest.length : end);
    return [...body.matchAll(/^([\w-]+)\s*=/gm)].map((matched) => matched[1]);
  }

  it('제품이 쓰는 crate 목록이 그대로다', () => {
    expect(dependencyNames('dependencies')).toEqual([
      'tauri',
      'serde',
      'serde_json',
      'rusqlite',
      'cpal',
      'hound',
      'whisper-rs',
      'rubato',
      'ureq',
      'keyring',
      'sha2',
    ]);
  });

  /**
   * **2026-09-08에 macOS 전용 섹션이 하나 생겼다** (`ADR-0012` · 온라인 회의 녹음).
   *
   * 여기 있는 넷은 **새로 내려받는 crate가 아니다** — `cpal 0.18` → `coreaudio-rs 0.14`와
   * Tauri가 이미 트리에 갖고 있는 것을 이름으로 부르고 feature를 켤 뿐이며, `Cargo.lock`의
   * checksum이 그대로 유지된다. 그럼에도 **목록은 여전히 닫혀 있어야 한다**: 무엇이 늘면
   * 여기서 먼저 드러난다.
   *
   * ⚠️ 이 검사가 실제로 버그를 잡았다. 처음에 이 섹션을 `[dependencies]` **한가운데** 넣어
   * 그 뒤의 일곱 crate(`hound` · `whisper-rs` · `ureq` …)가 전부 macOS 전용이 돼 있었고,
   * Windows(Phase 6)에서 빌드가 깨졌을 것이다.
   */
  it('macOS 전용 의존성 목록도 닫혀 있다', () => {
    expect(dependencyNames(`target.'cfg(target_os = "macos")'.dependencies`)).toEqual([
      'objc2-core-audio',
      'objc2-core-audio-types',
      'objc2',
      'objc2-foundation',
      'objc2-core-foundation',
    ]);
  });

  it('빌드 의존성도 그대로이고 테스트 전용 의존성은 생기지 않았다', () => {
    expect(dependencyNames('build-dependencies')).toEqual(['tauri-build']);
    // 이 Task가 더한 통합 테스트는 이미 있는 crate만 쓴다 — 검사를 위해 새 crate를 들이지 않는다.
    expect(manifest, 'dev-dependencies가 생겼다').not.toContain('[dev-dependencies]');
  });

  it('청크 분할 때문에 엔진 feature가 바뀌지 않았다', () => {
    // Metal은 TASK-069가 켠 것이고 이 Phase가 건드리는 자리가 아니다 (ADR-0007 §17.2).
    expect(manifest).toContain('whisper-rs = { version = "0.16", features = ["metal"] }');
  });
});

/**
 * (f) 벤더 고유 청킹 개념이 경계 밖으로 나가지 않았다 (INV-9 · ADR-0007 §20.8).
 *
 * 벤더는 바뀐다. 바뀔 때 흔들리는 것이 adapter 하나여야 하며, 그러려면 core/domain · payload ·
 * frontend 타입에 특정 제공자의 이름도, **그 제공자를 위해 만든 실행 개념도** 없어야 한다.
 * 청크는 후자다 — 120초로 자르는 것은 whisper를 그 오디오에 돌리기 위한 사정이며, 엔진이
 * 바뀌면 사라질 수 있는 개념이다. 밖에서 보면 전사 하나가 나올 뿐이다.
 */
describe('(f) 새 벤더 고유 청킹 개념이 생기지 않았다 (INV-9)', () => {
  /** `tests/ipc-boundary.test.ts`가 쓰는 것과 **같은 목록**이다. 아래에서 그 사실을 확인한다. */
  const VENDORS = /ollama|llama|openai|gpt-|anthropic|claude|gemini|groq|mistral|huggingface/i;

  /** 소스에 실제로 쓰인, `chunk`가 들어가는 식별자 전부. */
  function chunkNames(production: string): string[] {
    const found = [...production.matchAll(/\b\w*[Cc]hunks?\w*\b/g)].map((matched) => matched[0]);
    return [...new Set(found)].sort();
  }

  it('core/domain에 있는 chunk는 Notion 전송 조각뿐이다', () => {
    // `sent_chunks` · `total_chunks`는 ADR-0009가 만든 **전송** 개념이며 이 Phase보다 앞선다.
    // 전사 청크가 도메인에 생기면 그 목록이 늘어나므로 여기서 드러난다.
    const names = domainSources.flatMap((file) => chunkNames(productionRust(read(file))));

    expect([...new Set(names)].sort()).toEqual(['sent_chunks', 'total_chunks']);
  });

  it('command payload에 있는 chunk도 그 둘뿐이다', () => {
    expect(chunkNames(payloadProduction)).toEqual(['sent_chunks', 'total_chunks']);
  });

  it('frontend 계약에 있는 chunk도 그 둘뿐이다', () => {
    expect(chunkNames(typesProduction)).toEqual(['sentChunks', 'totalChunks']);
  });

  it('경계 셋 어디에도 벤더 이름도 엔진 고유 단위도 없다', () => {
    for (const file of domainSources) {
      const production = productionRust(read(file));
      expect(production, `${file}에 벤더 이름이 있다`).not.toMatch(VENDORS);
      expect(production, `${file}에 엔진 고유 단위가 있다`).not.toMatch(/centisecond/i);
    }
    expect(payloadProduction).not.toMatch(VENDORS);
    expect(payloadProduction, 'payload에 엔진 고유 단위가 있다').not.toMatch(/centisecond/i);
    expect(typesProduction).not.toMatch(VENDORS);
    expect(typesProduction, 'frontend 계약에 엔진 고유 단위가 있다').not.toMatch(/centisecond/i);
  });

  it('INV-9를 지키는 기존 검사가 그대로 남아 있다', () => {
    // `phase-prompt/05.8` Constraints: *"`tests/ipc-boundary.test.ts`의 검사가 그대로 통과해야
    // 한다."* 통과 여부는 그 파일이 실행되며 판정하지만, **그 검사가 약해지거나 사라지는 것**은
    // 그 파일 자신이 알릴 수 없다. 그래서 여기서 본다.
    const ipcBoundary = read(path('./ipc-boundary.test.ts'));

    expect(ipcBoundary).toContain("describe('wire 계약에 벤더가 없다 (INV-9)'");
    expect(ipcBoundary).toContain('command payload 타입에 벤더 고유 이름이 없다');
    expect(ipcBoundary).toContain('frontend 타입에 벤더 고유 이름이 없다');

    // 그리고 그쪽의 목록이 위에서 쓴 것과 **글자 그대로 같다.**
    const theirs = ipcBoundary.match(/const VENDORS = (\/[^\n]+\/i);/)?.[1];
    expect(theirs, 'ipc-boundary의 벤더 목록을 찾지 못했다').toBe(VENDORS.toString());
  });
});
