/**
 * 녹음 화면 (§5.B).
 *
 * 실제 오디오 캡처 · microphone 열거 · Record / Pause / Resume / Stop은 Phase 2의 일이다.
 * 이 화면은 아직 아무것도 녹음하지 않은 idle 상태까지만 보여준다.
 * §19에 따라 녹음 상태와 경과 시간이 화면에서 가장 크고 명확한 요소다.
 */
export function RecordingScreen() {
  return (
    <div className="screen screen--centered">
      <p className="recording__state">Idle</p>
      <p className="recording__elapsed">00:00</p>
      <p className="empty">No microphone selected.</p>
      <p className="hint">Audio capture is not implemented yet.</p>
    </div>
  );
}
