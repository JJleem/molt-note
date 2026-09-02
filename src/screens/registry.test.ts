// 라우트와 화면 컴포넌트의 연결 테스트.
// 컴포넌트를 import만 하고 렌더링하지 않으므로 DOM이 필요 없다.
// 렌더링 결과가 아니라 "모든 라우트가 실제 컴포넌트로 이어지는가"를 본다.
import { describe, expect, it } from 'vitest';
import { SCREEN_IDS } from '../navigation/routes';
import { RecordingDetailScreen } from './RecordingDetailScreen';
import { RecordingScreen } from './RecordingScreen';
import { RecordingsScreen } from './RecordingsScreen';
import { SettingsScreen } from './SettingsScreen';
import { SCREEN_COMPONENTS } from './registry';

describe('screen registry', () => {
  it('등록된 라우트마다 화면 컴포넌트가 있다', () => {
    for (const screen of SCREEN_IDS) {
      expect(typeof SCREEN_COMPONENTS[screen]).toBe('function');
    }
  });

  it('라우트에 없는 화면이 등록되어 있지 않다', () => {
    expect(Object.keys(SCREEN_COMPONENTS).sort()).toEqual([...SCREEN_IDS].sort());
  });

  it('각 라우트가 그 화면의 컴포넌트로 이어진다', () => {
    expect(SCREEN_COMPONENTS.recordings).toBe(RecordingsScreen);
    expect(SCREEN_COMPONENTS.recording).toBe(RecordingScreen);
    expect(SCREEN_COMPONENTS['recording-detail']).toBe(RecordingDetailScreen);
    expect(SCREEN_COMPONENTS.settings).toBe(SettingsScreen);
  });
});
