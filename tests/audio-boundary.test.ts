// 오디오 경계 테스트.
//
// 검사 대상은 동작이 아니라 **소스 전체에 대한 규칙**이다. 파일 하나가 새로 생기는 것만으로
// 조용히 깨질 수 있는 것이므로 개별 모듈 옆의 테스트가 아니라 여기서 본다.
//
//   1. 실제 오디오 장치를 아는 코드(`cpal`)가 지정된 두 파일 안에만 있다.
//   2. 진행 중인 녹음 session을 화면 컴포넌트가 소유하지 않는다 (R-001).
//   3. 제품 코드에 파일을 지우는 경로가 없다 (INV-3 · INV-4 · R-004).
//   4. 재생을 위해 열어 준 파일 접근 범위가 녹음 디렉터리 하나를 넘지 않는다 (§12 · INV-6).
//
// 1번이 무너지면 그 순간부터 캡처 주변 로직을 마이크 없이 검증할 수 없게 된다
// (PRODUCT-SPEC §18 · §3.1 · docs/ADR-0003-recording-engine.md §5.13).
// 2번이 무너지면 화면을 옮기는 것만으로 녹음이 사라지는 구조가 된다
// (docs/ADR-0004-recording-session-lifecycle.md).
// 3번이 무너지는 것은 한 줄로 충분하다 — 실패 경로에 "정리" 한 줄이 들어가면 사용자가
// 명시적으로 삭제하지 않은 녹음이 사라진다. 그래서 동작이 아니라 소스 전체를 본다.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const path = (relative: string) => fileURLToPath(new URL(relative, import.meta.url));

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

/** 주석 줄을 뺀 소스. 규칙을 설명하는 문장이 규칙 위반으로 잡히지 않게 한다. */
function withoutComments(source: string): string {
  return source
    .split('\n')
    .filter((line) => !line.trim().startsWith('//'))
    .join('\n');
}

const rustSources = sourceFiles(path('../src-tauri/src'), /\.rs$/);
const frontendSources = sourceFiles(path('../src'), /\.(ts|tsx)$/);

/**
 * `cpal`을 알아도 되는 파일.
 *
 * 장치를 **열거하는 자리**와 **여는 자리** 둘뿐이다. 이 목록을 늘리는 것은 경계를 옮기는
 * 결정이므로, 그 결정 없이 조용히 늘어나지 않게 여기 적어 둔다.
 */
const FILES_THAT_MAY_KNOW_CPAL = [
  path('../src-tauri/src/audio/system_devices.rs'),
  path('../src-tauri/src/audio/system_capture.rs'),
];

describe('실제 오디오 장치를 아는 코드는 두 파일 안에만 있다', () => {
  it('src-tauri 아래에서 cpal을 쓰는 파일이 그 둘뿐이다', () => {
    const users = rustSources.filter((file) =>
      /\bcpal\b/.test(withoutComments(readFileSync(file, 'utf8'))),
    );

    expect(users.sort()).toEqual([...FILES_THAT_MAY_KNOW_CPAL].sort());
  });

  it('두 파일이 실제로 존재하고 cpal을 쓴다', () => {
    // 위 검사가 "둘 다 사라졌다"로도 통과하지 않게 한다.
    for (const file of FILES_THAT_MAY_KNOW_CPAL) {
      expect(withoutComments(readFileSync(file, 'utf8')), `${file}`).toMatch(/\bcpal\b/);
    }
  });

  it('frontend는 오디오 장치를 직접 알지 않는다', () => {
    // 브라우저 쪽 캡처 API로 경계가 새는 것도 같은 문제다 (ADR-0003 §7.1).
    const forbidden = /getUserMedia|MediaRecorder|AudioContext|enumerateDevices/;

    for (const file of frontendSources) {
      expect(readFileSync(file, 'utf8'), `${file}가 오디오 장치를 직접 다룬다`).not.toMatch(
        forbidden,
      );
    }
  });
});

/**
 * `#[cfg(test)]` 앞까지, 즉 **제품이 실행하는 코드**만 남긴다.
 *
 * 테스트는 자신이 만든 임시 디렉터리를 치우므로 파일을 지운다. 그것은 사용자의 녹음이
 * 아니며, 이 규칙이 보는 것도 그쪽이 아니다.
 */
function productionRust(source: string): string {
  return withoutComments(source).split('#[cfg(test)]')[0];
}

describe('어떤 실패도 녹음 파일을 지우지 않는다 (INV-3 · INV-4 · R-004)', () => {
  it('제품 코드에 파일·디렉터리를 지우는 호출이 없다', () => {
    // 오디오 파일이 사라지는 유일한 길은 사용자가 파일 관리자에서 직접 지우는 것이다.
    // 앱 안에는 그 경로가 아예 없다 — 레코드 삭제(delete_recording)도 행만 지운다.
    const removal = /remove_file|remove_dir|remove_dir_all|\bunlink\b|OpenOptions[^;]*truncate\(\s*true/;

    for (const file of rustSources) {
      expect(productionRust(readFileSync(file, 'utf8')), `${file}에 파일을 지우는 경로가 있다`).not.toMatch(
        removal,
      );
    }
  });

  it('frontend에도 파일을 지우는 경로가 없다', () => {
    // command 표면에 없으므로 화면이 부를 수 있는 것도 없다. 새 경로가 생기면 여기서 걸린다.
    const forbidden = /removeFile|remove_file|@tauri-apps\/plugin-fs/;

    for (const file of frontendSources) {
      expect(readFileSync(file, 'utf8'), `${file}가 파일을 지운다`).not.toMatch(forbidden);
    }
  });

});

describe('재생 통로는 녹음 디렉터리 하나만 연다 (PRODUCT-SPEC §12 · INV-6)', () => {
  const libSource = readFileSync(path('../src-tauri/src/lib.rs'), 'utf8');
  const tauriConfig = JSON.parse(readFileSync(path('../src-tauri/tauri.conf.json'), 'utf8'));

  it('asset protocol이 켜져 있고, 설정만으로는 아무 자리도 열리지 않는다', () => {
    // 설정의 scope를 비워 두면 열리는 자리는 코드가 명시적으로 허용한 것뿐이다.
    // glob 한 줄이 늘어나는 것만으로 홈 디렉터리가 열리는 일이 없게 한다.
    const assetProtocol = tauriConfig.app.security.assetProtocol;

    expect(assetProtocol.enable).toBe(true);
    expect(assetProtocol.scope).toEqual([]);
  });

  it('코드가 여는 자리는 녹음 디렉터리 하나뿐이다', () => {
    // 앱 데이터 루트도, 사용자 홈도 아니다. 경로는 파일을 쓰는 쪽과 같은 자리에서 온다
    // (AppDataDirectory::recordings_dir · INV-10).
    const opened = [...productionRust(libSource).matchAll(/\.allow_(file|directory)\(([^)]*)\)/g)];

    expect(opened).toHaveLength(1);
    expect(opened[0][2]).toContain('recordings_dir');
    // 하위 디렉터리까지 열지 않는다 — 녹음 파일은 이 디렉터리에 바로 놓인다.
    expect(opened[0][2]).toContain('false');
    expect(libSource).toMatch(/ensure_recordings_dir\(\)/);
  });

  it('다른 어떤 Rust 파일도 접근 범위를 넓히지 않는다', () => {
    const wideners = rustSources.filter((file) =>
      /\.allow_(file|directory)\(/.test(productionRust(readFileSync(file, 'utf8'))),
    );

    expect(wideners).toEqual([path('../src-tauri/src/lib.rs')]);
  });

  it('파일 경로를 재생 주소로 바꾸는 곳은 ipc 모듈뿐이다', () => {
    // 화면이 직접 바꾸기 시작하면 어떤 경로가 webview로 흘러가는지 한 곳에서 볼 수 없게 된다.
    const converters = frontendSources.filter((file) =>
      /\bconvertFileSrc\s*\(/.test(withoutComments(readFileSync(file, 'utf8'))),
    );

    expect(converters).toEqual([path('../src/ipc/commands.ts')]);
  });

  it('frontend에 오디오를 기기 밖으로 보내는 경로가 없다 (INV-6)', () => {
    // 재생은 로컬 webview 안에서 끝난다. 원본 audio는 어떤 경우에도 나가지 않는다.
    const outbound = /\bfetch\s*\(|XMLHttpRequest|WebSocket|sendBeacon|EventSource/;

    for (const file of frontendSources) {
      expect(withoutComments(readFileSync(file, 'utf8')), `${file}가 밖으로 보낸다`).not.toMatch(
        outbound,
      );
    }
  });
});

describe('진행 중인 녹음을 화면이 소유하지 않는다 (R-001)', () => {
  it('앱이 시작될 때 녹음 session을 managed state로 등록한다', () => {
    const libSource = readFileSync(path('../src-tauri/src/lib.rs'), 'utf8');

    expect(libSource).toMatch(/app\.manage\(\s*Recorder::open_for\(app\)\s*\)/);
  });

  it('화면은 session 핸들이 아니라 command와 상태 조회로만 녹음을 다룬다', () => {
    // start가 핸들을 돌려주면 그 핸들의 수명이 곧 녹음의 수명이 된다.
    const commandsSource = readFileSync(path('../src/ipc/commands.ts'), 'utf8');

    expect(commandsSource).toMatch(/export function startCapture\([^)]*\): Promise<void>/);
    expect(commandsSource).toMatch(/export function pauseCapture\(\): Promise<void>/);
    expect(commandsSource).toMatch(/export function resumeCapture\(\): Promise<void>/);
    expect(commandsSource).toMatch(/export function captureStatus\(\): Promise<SessionStatus>/);
  });
});
