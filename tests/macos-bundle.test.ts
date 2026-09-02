// macOS 번들 선언 테스트.
//
// 검사 대상은 제품 기능이 아니라 **macOS packaging 선언**이다 (docs/PRODUCT-SPEC.md §14.3,
// docs/ADR-0002-macos-microphone-usage-description.md).
// NSMicrophoneUsageDescription이 올바른 파일(src-tauri/Info.plist)에 실제 문구와 함께
// 존재하는지, 그리고 잘못된 위치(tauri.conf.json)에 들어가지 않았는지만 본다.
//
// 이 테스트는 번들된 .app 안에서 Tauri CLI 생성값과 실제로 병합되는지는 검증하지 않는다.
// `npm run tauri build`는 Gate가 아니므로 그것은 사람 확인 항목으로 남는다.
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const readText = (p: string) => readFileSync(new URL(p, import.meta.url), 'utf8');

const infoPlist = readText('../src-tauri/Info.plist');
const tauriConfText = readText('../src-tauri/tauri.conf.json');

/**
 * plist의 <key>NAME</key> 바로 뒤에 오는 <string> 값을 읽는다.
 * 이 검사 하나 때문에 plist 파서 의존성을 새로 들이지 않는다.
 */
function plistString(source: string, key: string): string | null {
  const pattern = new RegExp(`<key>\\s*${key}\\s*</key>\\s*<string>([\\s\\S]*?)</string>`);
  const matched = source.match(pattern);
  return matched ? matched[1].trim() : null;
}

describe('macOS 번들 선언', () => {
  it('NSMicrophoneUsageDescription이 src-tauri/Info.plist에 존재한다', () => {
    expect(plistString(infoPlist, 'NSMicrophoneUsageDescription')).not.toBeNull();
  });

  it('권한 설명 문구가 비어 있지 않다', () => {
    // 빈 문자열이면 macOS 권한 프롬프트에 아무 설명도 뜨지 않는다.
    const usage = plistString(infoPlist, 'NSMicrophoneUsageDescription') ?? '';
    expect(usage.length).toBeGreaterThan(0);
    expect(usage).toMatch(/\S/);
  });

  it('NSMicrophoneUsageDescription을 tauri.conf.json에 잘못 넣지 않았다', () => {
    // tauri.conf.json 키가 아니다 (§14.3). 여기에 있으면 조용히 무시되고 번들에 반영되지 않는다.
    expect(tauriConfText).not.toContain('NSMicrophoneUsageDescription');
  });
});
