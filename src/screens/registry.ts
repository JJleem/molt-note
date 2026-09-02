import type { ComponentType } from 'react';
import type { ScreenId } from '../navigation/routes';
import { RecordingsScreen } from './RecordingsScreen';
import { RecordingScreen } from './RecordingScreen';
import { RecordingDetailScreen } from './RecordingDetailScreen';
import { SettingsScreen } from './SettingsScreen';
import type { ScreenProps } from './types';

/**
 * 라우트 → 화면 컴포넌트. 앱은 이 표를 통해서만 화면을 그리므로, 라우트에 등록되지 않은
 * 화면은 렌더링될 수 없다. Record<ScreenId, ...>이라 화면을 추가하면 여기 누락이
 * 컴파일 시점에 드러난다.
 */
export const SCREEN_COMPONENTS: Record<ScreenId, ComponentType<ScreenProps>> = {
  recordings: RecordingsScreen,
  recording: RecordingScreen,
  'recording-detail': RecordingDetailScreen,
  settings: SettingsScreen,
};
