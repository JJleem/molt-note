// 화면 경계 테스트.
//
// 개별 변환 함수의 동작은 각 모듈 옆의 테스트가 본다. 여기서 보는 것은 **소스 전체에 대한
// 두 가지 규칙**이며, 새 파일이 하나 생기는 것만으로 조용히 깨질 수 있는 것들이다.
//
//   1. 길이를 사람이 읽는 형식으로 바꾸는 규칙이 TypeScript에 다시 생기지 않았다.
//      그 규칙은 src-tauri/src/domain/duration.rs 한 곳에만 있고, 화면은 Rust가 보낸
//      durationLabel을 쓴다. 두 벌이 되면 조용히 갈라진다.
//   2. 실패가 console에만 남고 끝나는 경로가 없다 (PRODUCT-SPEC §13).
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

    for (const file of frontendSources) {
      const source = readFileSync(file, 'utf8');
      for (const shape of durationArithmetic) {
        expect(source, `${file}에 길이 포맷 계산이 있다`).not.toMatch(shape);
      }
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

describe('실패는 사용자에게 보인다', () => {
  it('src/ 아래에 실패를 console로 흘려보내는 경로가 없다', () => {
    // console.error로 끝나면 사용자는 아무것도 알지 못한다 (§13).
    for (const file of frontendSources) {
      const source = readFileSync(file, 'utf8');
      expect(source, `${file}에 console 출력이 있다`).not.toMatch(/\bconsole\s*\.\s*\w+\s*\(/);
    }
  });
});
