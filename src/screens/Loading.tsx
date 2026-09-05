/**
 * 오래 걸리는 동작이 얼어붙은 것처럼 보이지 않게 하는 한 줄 (`phase-prompt/05.5` 요구 9).
 *
 * 고리 하나와 **언제나 함께 오는 글자**다. 고리만으로는 무엇을 기다리는지 알 수 없고,
 * 움직임을 원하지 않는 사람에게는 그 고리가 멈춘 채로 남는다 (`prefers-reduced-motion`) —
 * 그래서 뜻을 나르는 것은 언제나 글자 쪽이며, 색도 움직임도 거들 뿐이다 (요구 12).
 *
 * **무엇을 기다리는가는 이 컴포넌트가 정하지 않는다.** 문장은 순수 모듈이 만든 값 그대로이며
 * 여기에는 그리는 일만 있다 (§18).
 *
 * `live`는 이 줄이 **나중에 나타나는** 자리에 쓴다 — 화면을 처음 그릴 때부터 있는 줄은
 * 알릴 것이 없고, 상태가 바뀌면서 생긴 줄만 소리로도 전해야 한다.
 */
export function Loading({ text, live = false }: { text: string; live?: boolean }) {
  return (
    <p className="loading" aria-live={live ? 'polite' : undefined}>
      {/* 장식이 아니라 '아직 진행 중'이라는 사실이다. 뜻은 옆의 글자가 이미 말한다. */}
      <span className="spinner" aria-hidden="true" />
      {text}
    </p>
  );
}
