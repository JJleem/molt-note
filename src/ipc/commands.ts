/**
 * 프론트엔드가 부를 수 있는 동작의 전부.
 *
 * `src-tauri/src/lib.rs`가 등록한 스물여덟 개 command와 1:1이며, 그 밖의 경로는 없다 —
 * **임의의 질의를 보낼 수단이 없다.** 저장소를 아는 코드는 Rust 안에만 있고, Notion으로
 * 나가는 요청을 만드는 코드도 마찬가지다 — webview에는 그 통로가 없다
 * (`docs/ADR-0001-local-persistence.md` · `docs/ADR-0009-notion-and-export.md` §5 ·
 * PRODUCT-SPEC §12 · INV-7).
 *
 * 실패는 예외로 흘리지 않고 언제나 {@link Failure}로 만들어 던진다. 화면은 어떤 실패든
 * 같은 모양으로 받아 사용자에게 보여줄 수 있다 (§13).
 *
 * ## command가 아닌 통로가 하나 있다 — 저장된 녹음의 재생
 *
 * 오디오 파일만은 command로 흐르지 않는다 ({@link recordingAudioSource}). 파일 바이트를
 * IPC에 실으면 한 시간짜리 녹음을 통째로 메모리에 올려야 하므로, 재생은 Tauri v2의
 * **asset protocol**을 지난다. 그 통로가 열리는 자리는 녹음 디렉터리 하나뿐이며 그 범위를
 * 정하는 것은 backend다 (`src-tauri/src/lib.rs` · docs/ADR-0006-audio-playback.md).
 * 저장소에 대한 질의는 여전히 아래 command가 전부다.
 */
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { toFailure } from './failure';
import type {
  AiNote,
  AiNoteStatus,
  AiProviderStatus,
  ExportedFile,
  InputDevice,
  MissingAudio,
  NewRecording,
  NoteMode,
  NotionConfirmation,
  NotionConnection,
  NotionSendStatus,
  NotionSync,
  NotionTokenStatus,
  Recording,
  SessionStatus,
  Settings,
  StoppedRecording,
  Transcript,
  TranscriptionStatus,
} from './types';

export type { Failure, FailureKind } from './failure';
export type {
  AiNote,
  AiNoteState,
  AiNoteStatus,
  AiProviderLocality,
  AiProviderState,
  AiProviderStatus,
  CaptureReport,
  ExportedFile,
  InputDevice,
  MeetingNote,
  MissingAudio,
  NewRecording,
  NoteMode,
  NotionConfirmation,
  NotionConnection,
  NotionConnectionState,
  NotionSendState,
  NotionSendStatus,
  NotionSync,
  NotionTokenStatus,
  ProcessingStatus,
  Recording,
  SessionState,
  SessionStatus,
  Settings,
  StoppedRecording,
  StructuredNote,
  StudyNote,
  SummaryNote,
  Transcript,
  TranscriptSegment,
  TranscriptionState,
  TranscriptionStatus,
} from './types';

/** command 하나를 부른다. 거절된 값은 언제나 {@link Failure}로 바꿔 던진다. */
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toFailure(error);
  }
}

/**
 * 저장된 녹음 파일을 webview가 재생할 수 있는 주소로 바꾼다 (Phase 2B 요구사항 7).
 *
 * **command가 아니다.** 파일 바이트는 IPC를 지나지 않고 Tauri v2의 asset protocol을 지난다
 * (`src-tauri/Cargo.toml`의 `protocol-asset` feature · `src-tauri/tauri.conf.json`의
 * `app.security.assetProtocol`). 그래서 긴 녹음도 통째로 메모리에 올리지 않고 재생된다.
 *
 * **어떤 경로가 실제로 열리는지는 이 함수가 정하지 않는다.** 주소를 만드는 일과 그 자리를
 * 여는 일은 갈라져 있고, 여는 쪽은 backend가 녹음 디렉터리 하나로 제한한다
 * (`src-tauri/src/lib.rs`) — 다른 경로를 넣어도 그 파일은 열리지 않는다.
 *
 * 만들어진 주소는 로컬 webview 안에서만 쓰인다. 오디오가 기기 밖으로 나가는 경로는 이 앱에
 * 없다 (PRODUCT-SPEC §12 · INV-6).
 */
export function recordingAudioSource(audioPath: string): string {
  return convertFileSrc(audioPath);
}

/** 저장된 녹음을 최근 것부터 읽는다. 하나도 없으면 빈 배열이다. */
export function listRecordings(): Promise<Recording[]> {
  return call<Recording[]>('list_recordings');
}

/** 녹음 하나를 읽는다. 그런 id가 없으면 `null`이다. */
export function getRecording(recordingId: string): Promise<Recording | null> {
  return call<Recording | null>('get_recording', { recordingId });
}

/**
 * 저장된 Transcript 하나를 **segment까지** 읽는다. 그런 id가 없으면 `null`이다.
 *
 * 화면은 `Recording.currentTranscriptId`가 가리키는 것을 이 함수로 읽는다 (§7.2).
 *
 * **읽기뿐이다.** Transcript를 고치거나 지우는 command는 없다 — Transcript는 immutable이고
 * (§7.1 · INV-2) 저장소가 내놓는 쓰기 경로도 추가 하나뿐이다. 그래서 화면에서 시작한 어떤
 * 동작도 이미 저장된 전사 내용을 바꾸지 못한다.
 */
export function getTranscript(transcriptId: string): Promise<Transcript | null> {
  return call<Transcript | null>('get_transcript', { transcriptId });
}

/** 녹음 하나를 저장하고 저장된 모습을 받는다. */
export function createRecording(recording: NewRecording): Promise<Recording> {
  return call<Recording>('create_recording', { recording });
}

/** 녹음 레코드 하나를 지운다. 지웠으면 `true`, 그런 id가 없었으면 `false`다. */
export function deleteRecording(recordingId: string): Promise<boolean> {
  return call<boolean>('delete_recording', { recordingId });
}

/** 저장된 설정을 읽는다. 저장된 적이 없으면 기본값이 온다. */
export function getSettings(): Promise<Settings> {
  return call<Settings>('get_settings');
}

/** 설정을 저장하고, 저장된 결과를 받는다. */
export function updateSettings(settings: Settings): Promise<Settings> {
  return call<Settings>('update_settings', { settings });
}

/**
 * 고를 수 있는 입력 장치를 표시 순서로 읽는다 (기본 장치가 먼저).
 *
 * 하나도 없으면 빈 배열이다 — 오류가 아니다. 목록은 부를 때마다 새로 만들어지므로
 * 장치가 꽂히거나 빠진 뒤에는 다시 부른다.
 */
export function listInputDevices(): Promise<InputDevice[]> {
  return call<InputDevice[]>('list_input_devices');
}

/**
 * 고른 장치를 열고 녹음을 시작한다.
 *
 * `deviceKey`는 {@link listInputDevices}가 준 `key`다. 이미 녹음 중이면 실패한다 —
 * 진행 중인 녹음이 조용히 버려지지 않는다.
 *
 * **진행 중인 session을 소유하는 것은 backend다.** 이 함수는 핸들을 돌려주지 않으며,
 * 화면이 사라져도 녹음은 계속된다 (R-001 · docs/ADR-0004-recording-session-lifecycle.md).
 * 지금 상태는 {@link captureStatus}로 물어보고, 결과는 {@link stopCapture}가 돌려준다.
 */
export function startCapture(deviceKey: string): Promise<void> {
  return call<void>('start_capture', { deviceKey });
}

/**
 * 녹음을 일시정지한다.
 *
 * 장치도 파일도 열려 있는 채로 남는다 — 이 시점 이후의 소리는 파일에 들어가지 않고,
 * 흐르는 시간도 녹음 길이에 더해지지 않는다. 녹음 중이 아니면 실패한다.
 */
export function pauseCapture(): Promise<void> {
  return call<void>('pause_capture');
}

/**
 * 일시정지한 녹음을 다시 시작한다. **같은 파일에 이어 쓴다.**
 *
 * 일시정지 상태가 아니면 실패한다.
 */
export function resumeCapture(): Promise<void> {
  return call<void>('resume_capture');
}

/**
 * 녹음을 정지한다. **성공은 파일이 확정된 것을 뜻한다** (PRODUCT-SPEC §6의 R-002).
 *
 * 이 함수가 값을 돌려줬다면 네 가지가 모두 성립한 것이다 — 파일이 존재하고, 크기가 유효
 * 최소치를 넘고, 포맷을 알고 있고, Recording 레코드가 저장됐다. 그러지 못하면 실패이며,
 * **그 실패는 확정된 파일이 어디 남아 있는지 함께 말한다** — 어떤 실패 경로도 이미 녹음된
 * audio를 지우지 않는다 (INV-3 · INV-4 · docs/ADR-0004-recording-session-lifecycle.md).
 *
 * `title`은 사용자가 입력한 제목이다. 비어 있거나 없으면 Rust가 저장 시각에서 만든다 —
 * 이름이 없다는 이유로 방금 녹음한 것이 유실되지 않게 한다.
 */
export function stopCapture(title?: string | null): Promise<StoppedRecording> {
  return call<StoppedRecording>('stop_capture', { title: title ?? null });
}

/**
 * **레코드는 있는데 오디오 파일이 없는** 녹음을 읽는다. 하나도 없으면 빈 배열이다.
 *
 * 정지 경로는 이 상태를 만들지 않는다 — 레코드는 파일이 확인된 뒤에만 저장된다. 이 목록은
 * 파일이 앱 밖에서 옮겨지거나 지워졌을 때를 위한 **감지 수단**이며, 부른다고 해서 레코드가
 * 지워지거나 고쳐지지 않는다.
 */
export function listMissingAudio(): Promise<MissingAudio[]> {
  return call<MissingAudio[]>('list_missing_audio');
}

/**
 * 지금 녹음이 어떤 상태이고 얼마나 진행됐는지 물어본다.
 *
 * 진행 중인 녹음이 없으면 `idle`과 `0:00`이 온다 — 오류가 아니다. 경과 시간 문자열은
 * Rust가 만들어 보낸다. **화면이 길이를 계산하지 않는다.**
 */
export function captureStatus(): Promise<SessionStatus> {
  return call<SessionStatus>('capture_status');
}

/**
 * 녹음 하나의 전사를 시작한다. **돌아오는 것은 접수 사실이지 전사 결과가 아니다.**
 *
 * 실제 전사는 backend의 배경 스레드에서 돌므로 이 호출은 바로 끝난다 — 1시간짜리 녹음을 걸어도
 * 화면과 다른 command가 멈추지 않는다 (`phase-prompt/03` 요구 3). 결과는
 * {@link transcriptionStatus}로 물어본다.
 *
 * **진행 중인 전사를 소유하는 것은 backend다.** 이 함수는 핸들을 돌려주지 않으며, 화면이
 * 사라져도 전사는 계속된다 (R-001과 같은 규약).
 *
 * 이미 전사 중이면 실패한다 — 같은 녹음이어도 마찬가지다. 두 번째 요청이 조용히 사라지지
 * 않는다. 여러 녹음을 줄 세우는 큐는 아직 없다 (PRODUCT-SPEC §16 DEFERRED).
 */
export function startTranscription(recordingId: string): Promise<TranscriptionStatus> {
  return call<TranscriptionStatus>('start_transcription', { recordingId });
}

/**
 * 지금 전사가 어떤 상태인지 물어본다. **전사가 도는 동안에도 즉시 답한다.**
 *
 * 아직 아무것도 걸지 않았으면 `idle`이 온다 — 오류가 아니다. 실패했으면 그 {@link Failure}가
 * 그대로 실려 오므로 화면은 무엇이 실패했는지, 원본이 안전한지, 다시 시도할 수 있는지를
 * 그 값에서 읽는다 (§13).
 */
export function transcriptionStatus(): Promise<TranscriptionStatus> {
  return call<TranscriptionStatus>('transcription_status');
}

/**
 * 고른 AI provider가 지금 어떤 상태인지 물어본다 — 무엇을 골랐는지 · 쓸 수 있는지 · 어떤
 * 모델이 있는지 · **로컬인지 외부인지** (`phase-prompt/04` 요구 8 · 9 · 15 · INV-5).
 *
 * **provider를 고르지 않았거나 서버가 응답하지 않는 것은 실패가 아니다** — 그 사실이
 * {@link AiProviderStatus.state}로 온다 (INV-8). 이 호출이 실패하는 경우는 저장소에서 설정을
 * 읽지 못했을 때뿐이며, 그것은 provider와 무관하다.
 *
 * **어디에 어떻게 연결하는지는 화면이 알지 않는다.** 주소도 엔드포인트도 backend 안에만 있고,
 * webview에는 AI 서버로 가는 통로가 없다 (docs/ADR-0008-note-ai-provider.md §5 · INV-9).
 *
 * 서버가 응답하지 않으면 이 호출은 그 시간만큼 걸릴 수 있다 — 그동안에도 다른 command와 화면은
 * 계속 응답한다 (backend가 이 질의를 main thread에서 처리하지 않는다).
 */
export function aiProviderStatus(): Promise<AiProviderStatus> {
  return call<AiProviderStatus>('ai_provider_status');
}

/**
 * 녹음 하나의 AI 노트 생성을 시작한다. **돌아오는 것은 접수 사실이지 노트가 아니다.**
 *
 * 실제 생성은 backend의 배경 스레드에서 돌므로 이 호출은 바로 끝난다 — 로컬 모델이 몇 분을 써도
 * 화면과 다른 command가 멈추지 않는다. 결과는 {@link aiNoteStatus}로 물어보고, 만들어진 노트는
 * {@link getAiNote} · {@link listAiNotes}로 읽는다.
 *
 * **재생성도 이 함수다.** 이미 노트가 있는 녹음에 다시 걸면 노트가 하나 더 생기고 이전 노트는
 * 그대로 남는다 (ADR-0008 §9.2) — 그래서 "다시 만들기"를 위한 별도의 이름이 없다.
 *
 * 이미 생성 중이면 실패한다 — 같은 녹음이어도 마찬가지이며, 두 번째 요청이 조용히 사라지지
 * 않는다. **provider를 고르지 않았다는 이유로는 실패하지 않는다** (INV-8): 그때는 접수된 뒤
 * `failed` 상태에 §13의 `aiProviderNotConfigured`가 실려 온다.
 *
 * `mode`는 만들 노트의 종류다 (§9.5). 세 값 중 하나가 아니면 실패한다.
 */
export function startAiNote(recordingId: string, mode: NoteMode): Promise<AiNoteStatus> {
  return call<AiNoteStatus>('start_ai_note', { recordingId, mode });
}

/**
 * 지금 AI 노트 생성이 어떤 상태인지 물어본다. **생성이 도는 동안에도 즉시 답한다.**
 *
 * 아직 아무것도 걸지 않았으면 `idle`이 온다 — 오류가 아니다. 실패했으면 그 {@link Failure}가
 * 그대로 실려 오므로 화면은 무엇이 실패했는지, 원본이 안전한지, 다시 시도할 수 있는지를 그
 * 값에서 읽는다 (§13).
 */
export function aiNoteStatus(): Promise<AiNoteStatus> {
  return call<AiNoteStatus>('ai_note_status');
}

/**
 * 그 Transcript에서 만들어진 AI 노트를 **만들어진 순서대로** 읽는다. 하나도 없으면 빈 배열이다.
 *
 * 화면은 `Recording.currentTranscriptId`가 가리키는 Transcript의 노트를 이 함수로 읽는다 (§7.2).
 * **이력이 그대로 온다** — 재생성은 대체가 아니라 추가이므로 같은 mode의 노트가 여럿일 수 있고,
 * 그중 무엇을 보여줄지는 화면이 정한다 (ADR-0008 §9.2).
 *
 * **읽기뿐이다.** 노트를 고치거나 지우는 command는 없다 — 저장소가 내놓는 쓰기 경로도 추가
 * 하나뿐이므로, 화면에서 시작한 어떤 동작도 이미 만들어진 노트를 바꾸지 못한다.
 */
export function listAiNotes(transcriptId: string): Promise<AiNote[]> {
  return call<AiNote[]>('list_ai_notes', { transcriptId });
}

/** AI 노트 하나를 읽는다. 그런 id가 없으면 `null`이다. */
export function getAiNote(aiNoteId: string): Promise<AiNote | null> {
  return call<AiNote | null>('get_ai_note', { aiNoteId });
}

/**
 * 녹음 하나를 로컬 Markdown 파일로 내보낸다 (§11 · `phase-prompt/05` 요구 A-1).
 *
 * **돌아온 값은 이미 만들어진 파일을 가리킨다** — 접수 사실이 아니다. 기다릴 모델도 서버도
 * 없으므로 전사·AI 노트처럼 시작하고 상태를 물어보는 규약을 쓰지 않는다.
 *
 * **AI provider를 고르지 않았거나 AI 노트가 하나도 없어도 성공한다** (INV-8). 그때는
 * Transcript와 메타데이터만으로 이루어진 유효한 문서가 나오며, 없는 AI 섹션의 빈 껍데기가
 * 남지 않는다.
 *
 * **이미 있는 파일을 덮어쓰지 않는다.** 같은 이름이 있으면 backend가 번호를 붙이므로
 * ({@link ExportedFile.fileName}), 내보낸 파일을 사용자가 손댔더라도 그것이 사라지지 않는다
 * (docs/ADR-0009-notion-and-export.md §4.3).
 *
 * 아직 전사가 없는 녹음이면 실패한다 — 제목과 길이만 담긴 빈 문서를 만드는 대신 무엇이
 * 필요한지 말한다 (§13). 어떤 실패에서도 녹음 · 전사 · 노트는 그대로다 (INV-3).
 *
 * **어디에 만들어지는지는 화면이 정하지 않는다.** 자리를 아는 것은 backend 하나이며 (INV-10),
 * 그래서 만들어진 파일의 전체 경로가 함께 온다 — 사용자가 파일을 찾지 못하는 상태로 두지 않는다.
 */
export function exportMarkdown(recordingId: string): Promise<ExportedFile> {
  return call<ExportedFile>('export_markdown', { recordingId });
}

/**
 * 녹음 하나를 Notion 페이지로 보내기 시작한다. **돌아오는 것은 접수 사실이지 전송 결과가 아니다.**
 *
 * 실제 전송은 backend의 배경 스레드에서 돌므로 이 호출은 바로 끝난다 — 1시간짜리 transcript를
 * 걸어도 화면과 다른 command가 멈추지 않는다. 결과는 {@link notionSyncStatus}로 물어보고,
 * 디스크에 남은 사실은 {@link getNotionSync}로 읽는다.
 *
 * **진행 중인 전송을 소유하는 것은 backend다** (R-001과 같은 규약). 이미 보내는 중이면
 * 실패한다 — 같은 녹음이어도 마찬가지이며, 두 번째 요청이 조용히 사라지지 않는다. 여러 녹음을
 * 줄 세우는 큐는 아직 없다 (PRODUCT-SPEC §16 DEFERRED).
 *
 * **token이나 부모 페이지를 설정하지 않았다는 이유로는 실패하지 않는다** (INV-8): 그때는 접수된 뒤
 * 무엇이 남았는지가 `failed` 상태에 실려 온다.
 *
 * `confirmation`은 사용자가 무엇을 확인했는가다 (docs/ADR-0009-notion-and-export.md §8.3).
 * **보내지 않으면 아무것도 확인하지 않은 것이며**, 새 페이지가 필요한 상태였다면 전송은 무엇을
 * 확인해야 하는지 말하며 거절된다 — 앱이 대신 고르지 않으므로 사용자가 모르는 사이에 페이지가
 * 둘이 되지 않는다.
 *
 * **어디로 어떻게 보내는지는 화면이 알지 않는다.** 주소도 엔드포인트도 자격증명도 backend 안에만
 * 있고, webview에는 Notion으로 가는 통로가 없다 (PRODUCT-SPEC §12 · INV-7).
 */
export function startNotionSync(
  recordingId: string,
  confirmation?: NotionConfirmation,
): Promise<NotionSendStatus> {
  return call<NotionSendStatus>('start_notion_sync', {
    recordingId,
    confirmation: confirmation ?? null,
  });
}

/**
 * 지금 Notion 전송이 어떤 상태인지 물어본다. **전송이 도는 동안에도 즉시 답한다.**
 *
 * 아직 아무것도 걸지 않았으면 `idle`이 온다 — 오류가 아니다. 실패했으면 그 {@link Failure}가
 * 그대로 실려 오므로 화면은 무엇이 실패했는지, 원본이 안전한지, 다시 시도할 수 있는지를 그
 * 값에서 읽는다 (§13).
 */
export function notionSyncStatus(): Promise<NotionSendStatus> {
  return call<NotionSendStatus>('notion_sync_status');
}

/**
 * 그 녹음의 **저장된** Notion 전송 기록을 읽는다. 보낸 적이 없으면 `null`이다 (오류가 아니다).
 *
 * {@link notionSyncStatus}와 다른 값이다 — 저쪽은 앱이 켜져 있는 동안의 진행 상황이고, 이쪽은
 * 디스크에 남아 있는 사실이다. **부분 전송이 여기서 드러난다** (`sentChunks` · `totalChunks`).
 *
 * **읽기뿐이다.** 전송 기록을 고치거나 지우는 command는 없다.
 */
export function getNotionSync(recordingId: string): Promise<NotionSync | null> {
  return call<NotionSync | null>('get_notion_sync', { recordingId });
}

/**
 * 저장된 token으로 지금 Notion과 말할 수 있는지 확인한다 (§5-D의 connection test).
 *
 * **token을 저장하지 않았거나 부모 페이지를 고르지 않은 것은 실패가 아니다** — 그 사실이
 * {@link NotionConnection.state} · {@link NotionConnection.destinationConfigured}로 온다 (INV-8).
 * 확인하지 못했다면 무엇이 달랐는지가 §13의 {@link Failure}로 실려 오므로, 화면은 token을 다시
 * 넣어야 하는 것과 부모 페이지를 공유해야 하는 것과 네트워크 문제를 구분해 안내할 수 있다.
 *
 * **token 값은 오지 않는다** (INV-7). 오는 것은 저장돼 있다는 사실뿐이다.
 *
 * 서버가 응답하지 않으면 이 호출은 그 시간만큼 걸릴 수 있다 — 그동안에도 다른 command와 화면은
 * 계속 응답한다.
 */
export function checkNotionConnection(): Promise<NotionConnection> {
  return call<NotionConnection>('check_notion_connection');
}

/**
 * integration token을 저장하고 **저장 여부만** 받는다 (INV-7 · ADR-0009 §10).
 *
 * 값이 이 경계를 지나는 방향은 하나다 — 화면에서 backend의 자격증명 저장소로. 돌아오는 길에는
 * 값이 실리지 않으며, 저장된 token을 다시 읽는 command도 없다. 그래서 화면은 값을 들고 있을
 * 이유가 없다 — 넘긴 뒤에는 입력란을 비우면 된다.
 *
 * 빈 값은 저장되지 않는다 (실패한다). 어디에 저장되는지는 화면이 알지 않으며 (INV-10), 저장할
 * 자리가 없는 시스템에서는 파일로 대신 떨어뜨리지 않고 그 사실을 실패로 알린다.
 */
export function saveNotionToken(token: string): Promise<NotionTokenStatus> {
  return call<NotionTokenStatus>('save_notion_token', { token });
}

/**
 * 저장된 integration token을 지우고 **저장 여부만** 받는다.
 *
 * **없던 것을 지우는 것은 실패가 아니다.** 지운 뒤에도 녹음 · 전사 · 노트 · 이미 만들어진
 * Notion 페이지는 그대로다 (INV-3).
 */
export function deleteNotionToken(): Promise<NotionTokenStatus> {
  return call<NotionTokenStatus>('delete_notion_token');
}
