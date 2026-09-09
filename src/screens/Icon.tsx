/**
 * 화면이 쓰는 아이콘 전부 (2026-09-09).
 *
 * ## 그리지 않는다 — 골라 쓴다
 *
 * 처음에는 SVG path를 손으로 적었다가 걷어냈다. 톱니바퀴 하나가 40줄이 되고, 그 40줄이
 * 맞는지 아무도 검증하지 않는다. **아이콘은 이 제품이 잘할 일이 아니다.**
 *
 * 지금은 `lucide-react`를 쓴다. 선 기반이라 이 화면의 화법(굵은 산세리프 · 넉넉한 모서리)과
 * 어울리고, 필요한 것만 번들에 들어간다 (tree-shaking). Google의 Material Symbols도
 * 후보였으나 그쪽은 채운 모양이 기본이라 이 화면에서는 무거워 보인다.
 *
 * **외부로 요청을 보내지 않는다.** 글꼴을 번들한 것과 같은 이유다 — 아이콘 폰트 CDN도,
 * 이미지 주소도 쓰지 않는다. 빌드 시점에 모두 앱 안으로 들어온다.
 *
 * ## 이름은 이 제품의 것이다
 *
 * 화면은 `note` · `record` 처럼 **역할**로 부르고, 그 역할이 어느 아이콘인지는 아래 표
 * 하나가 정한다. 세트를 갈아도 고칠 자리는 이 파일 하나다 — 화면 스무 곳이 벤더의
 * 아이콘 이름을 알고 있으면 그렇게 되지 않는다 (INV-9와 같은 태도).
 *
 * ## 아이콘은 글자를 대신하지 않는다
 *
 * **언제나 글자가 함께 온다** (요구 12). 뜻을 그림 하나로만 나르면 그 뜻을 모르는
 * 사람에게는 아무것도 남지 않는다. 그래서 기본값은 `aria-hidden`이며, 읽어 줄 이름은
 * 옆의 글자가 갖는다.
 */

import {
  AlignLeft,
  AudioLines,
  Check,
  Circle,
  FileText,
  List,
  Mic,
  Pause,
  Play,
  RefreshCw,
  Send,
  Settings,
  Sparkles,
  Square,
  TriangleAlert,
  type LucideIcon,
} from 'lucide-react';

/** 그릴 수 있는 아이콘 이름 전부. **여기 없는 이름은 그릴 수 없다.** */
export type IconName =
  // 사이드바
  | 'list'
  | 'mic'
  | 'settings'
  // 녹음 조작
  | 'record'
  | 'pause'
  | 'play'
  | 'stop'
  // 탭과 내용
  | 'note'
  | 'transcript'
  | 'audio'
  // 내보내기와 상태
  | 'file'
  | 'send'
  | 'check'
  | 'alert'
  | 'refresh';

/** 역할 하나가 아이콘 하나. **화면은 왼쪽만 알고, 오른쪽은 이 파일만 안다.** */
const ICONS: Record<IconName, LucideIcon> = {
  list: List,
  mic: Mic,
  settings: Settings,

  record: Circle,
  pause: Pause,
  play: Play,
  stop: Square,

  note: Sparkles,
  transcript: AlignLeft,
  audio: AudioLines,

  file: FileText,
  send: Send,
  check: Check,
  alert: TriangleAlert,
  refresh: RefreshCw,
};

/**
 * 선이 아니라 **면**으로 그리는 것.
 *
 * 재생·정지·녹음은 조작이고, 조작은 꽉 차 있어야 눌러야 할 것으로 읽힌다.
 * 나머지는 전부 선이다 — 섞이면 어느 것이 조작인지 알 수 없게 된다.
 */
const FILLED: ReadonlySet<IconName> = new Set<IconName>(['record', 'play', 'stop']);

interface IconProps {
  readonly name: IconName;
  /**
   * 이 아이콘이 **혼자서** 뜻을 나를 때만 준다.
   *
   * 옆에 글자가 있으면 주지 않는다 — 같은 말을 두 번 읽어 주게 된다.
   */
  readonly label?: string;
}

/**
 * 아이콘 하나.
 *
 * 크기는 부모의 글자 크기를 따른다 (`1em`). 그래서 버튼 안에서는 버튼 글자만큼,
 * 제목 옆에서는 제목만큼 자란다 — 자리마다 크기를 적지 않아도 된다.
 * 색은 `currentColor`이므로 테마도 상태도 저절로 맞는다.
 */
export function Icon({ name, label }: IconProps) {
  const Drawn = ICONS[name];
  const filled = FILLED.has(name);

  return (
    <Drawn
      className="icon"
      size="1em"
      strokeWidth={filled ? 0 : 1.9}
      fill={filled ? 'currentColor' : 'none'}
      // 글자가 함께 오는 것이 기본이므로 보조기술에는 숨긴다 (요구 12).
      aria-hidden={label === undefined ? true : undefined}
      role={label === undefined ? undefined : 'img'}
      aria-label={label}
      focusable="false"
    />
  );
}
