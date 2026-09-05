import type { ReactNode } from 'react';

/**
 * 데이터가 없는 자리 (`phase-prompt/05.5` 요구 9 · PRODUCT-SPEC §13의 태도).
 *
 * **없는 것은 고장이 아니다.** 경고색도 테두리도 두지 않는다 — 무엇이 없는가 한 줄, 무엇을
 * 하면 되는가 한 줄, 그리고 할 수 있는 일이 있다면 그 동작 하나다.
 *
 * 다섯 갈래(녹음 없음 · transcript 없음 · AI Note 없음 · provider 없음 · Notion 미설정)가
 * 전부 이 한 모양을 쓴다. 빈 상태가 화면마다 제각각이지 않게 하는 것이 이 컴포넌트의 이유다.
 *
 * **무엇이 비었는지도, 그때 무엇을 할 수 있는지도 이 컴포넌트가 정하지 않는다.** 문장은
 * 순수 모듈이 만든 값 그대로이며 여기에는 그리는 일만 있다 (§18).
 */
export function EmptyState({
  title,
  body,
  children,
}: {
  title: string;
  /** 무엇을 하면 되는가. 할 말이 없으면 넘기지 않는다. */
  body?: string | null;
  /** 거드는 문장과 동작. 있으면 본문 아래에 그대로 놓인다. */
  children?: ReactNode;
}) {
  return (
    <div className="empty-state" role="status">
      <p className="empty-state__title">{title}</p>
      {body !== undefined && body !== null && <p className="empty-state__body">{body}</p>}
      {children}
    </div>
  );
}
