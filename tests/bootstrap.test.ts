// Bootstrap baseline test.
//
// 제품 기능이 아니라 개발 baseline이 온전한지 확인한다.
// Phase 1이 시작되기 전에 이미 깨질 수 있는 것들만 본다:
// 프로젝트 식별자, 버전 정합성, Tauri 설정이 실제로 존재하고 파싱되는지.
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const read = (p: string) => JSON.parse(readFileSync(new URL(p, import.meta.url), 'utf8'));

const pkg = read('../package.json');
const tauriConf = read('../src-tauri/tauri.conf.json');

describe('bootstrap baseline', () => {
  it('package.json이 이 프로젝트를 식별한다', () => {
    expect(pkg.name).toBe('molt-note');
  });

  it('tauri.conf.json이 파싱되고 macOS 번들 식별자를 가진다', () => {
    expect(tauriConf.identifier).toBe('com.moltnote.app');
    expect(tauriConf.productName).toBeTruthy();
  });

  it('frontend 버전과 Tauri 앱 버전이 어긋나지 않는다', () => {
    // 둘이 갈라지면 배포 산출물의 버전이 조용히 틀어진다.
    expect(tauriConf.version).toBe(pkg.version);
  });

  it('Tauri build가 vite 산출물 디렉터리를 가리킨다', () => {
    expect(tauriConf.build.frontendDist).toBe('../dist');
  });
});
