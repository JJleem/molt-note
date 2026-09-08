/**
 * 지금 배경에서 무엇이 돌고 있는가 — **화면을 떠나도 보이는 한 줄** (2026-09-08).
 *
 * ## 왜 있는가
 *
 * 전사도 AI 노트도 Notion 전송도 배경 스레드에서 돈다. 그래서 화면을 옮겨도 계속되지만,
 * **그 사실을 볼 자리가 그 화면 안에만 있었다.** 2026-09-08에 사람이 2시간 녹음의 전사를
 * 걸어 두고 다른 화면으로 갔다가, 끝났는지 알 방법이 없어 다시 그 녹음을 찾아 들어가야
 * 했다. 6분이 걸리는 일이고 그동안 앱은 아무 말도 하지 않았다.
 *
 * ## 이 모듈이 하지 않는 것
 *
 * **상태를 만들지 않는다.** 세 값은 backend가 각자 들고 있는 것이고 (`transcription_status`
 * · `ai_note_status` · `notion_sync_status`), 여기서 하는 일은 *지금 돌고 있는 것이
 * 있는가*와 *그것을 뭐라고 부를 것인가* 둘뿐이다.
 *
 * **끝난 것을 말하지 않는다.** `done`도 `failed`도 여기 오지 않는다 — 결과는 그 녹음의
 * 화면이 말하며, 그 자리가 §13의 세 질문에 답하는 자리다. 이 줄이 결과까지 말하면
 * 같은 사실이 두 곳에서 서로 다른 문장으로 살게 된다.
 *
 * **누를 수 없다.** 진행 중이라는 사실만 알린다. 어디로 갈지는 사람이 정한다.
 */
import type { AiNoteStatus, NotionSendStatus, TranscriptionStatus } from '../ipc/types';

/** 배경에서 도는 일 하나. */
export interface RunningWork {
  /** 무엇이 도는가. 화면이 이 문장을 다시 만들지 않는다. */
  readonly text: string;
  /** 어느 녹음의 일인가. 모르면 `null`이다 — 없는 것을 지어내지 않는다. */
  readonly recordingId: string | null;
}

/** 전사가 얼마나 갔는가를 함께 적은 문장. 진행률을 모르면 그 부분이 없다. */
function transcribing(progress: number | null): string {
  if (progress === null) {
    return '전사 중';
  }
  // 내림한다 — 아직 끝나지 않았는데 100%가 보이지 않게 한다 (`transcriptView`와 같은 규칙).
  return `전사 중 ${Math.floor(progress * 100)}%`;
}

/**
 * 지금 도는 일들. 하나도 없으면 빈 배열이다.
 *
 * **순서는 파이프라인 순서다** — 전사 → AI 노트 → Notion. 셋이 동시에 돌 수 있고,
 * 그때 사람이 읽는 순서가 일이 일어나는 순서와 같아야 한다.
 */
export function runningWork(
  transcription: TranscriptionStatus | null,
  aiNote: AiNoteStatus | null,
  notion: NotionSendStatus | null,
): readonly RunningWork[] {
  const running: RunningWork[] = [];

  if (transcription !== null && transcription.state === 'running') {
    running.push({
      text: transcribing(transcription.progress),
      recordingId: transcription.recordingId,
    });
  }
  if (aiNote !== null && aiNote.state === 'running') {
    running.push({ text: 'AI 노트 만드는 중', recordingId: aiNote.recordingId });
  }
  if (notion !== null && notion.state === 'running') {
    running.push({ text: 'Notion으로 보내는 중', recordingId: notion.recordingId });
  }

  return running;
}

/**
 * 이 줄을 화면에 둘 것인가.
 *
 * **도는 것이 없으면 두지 않는다.** 아무 일도 없을 때 "대기 중" 같은 줄을 남겨 두면
 * 화면에 늘 있는 소음이 되고, 정작 무언가 돌 때 그것이 눈에 띄지 않는다.
 */
export function hasRunningWork(work: readonly RunningWork[]): boolean {
  return work.length > 0;
}
