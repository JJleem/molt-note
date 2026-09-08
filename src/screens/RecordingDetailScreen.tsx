import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  aiNoteStatus,
  aiProviderStatus,
  exportAiRequest,
  exportMarkdown,
  getAiPrompt,
  getNotionSync,
  getRecording,
  getTranscript,
  getTranscriptText,
  listAiNotes,
  listMissingAudio,
  notionSyncStatus,
  recordingAudioSource,
  showSavedFile,
  startAiNote,
  startNotionSync,
  startTranscription,
  transcriptionStatus,
} from '../ipc/commands';
import type {
  AiNote,
  AiNoteStatus,
  AiProviderStatus,
  NoteMode,
  NotionConfirmation,
  NotionSendStatus,
  NotionSync,
  Recording,
  Transcript,
  TranscriptionStatus,
} from '../ipc/types';
// 이 앱에서 clipboard에 쓰는 유일한 경로 (R-4 · INV-10). 실패는 던져지지 않고 값으로 온다.
import { copyText } from '../platform/clipboard';
import {
  NO_AI_EXPORT_ATTEMPT,
  aiNoteTabLayout,
  exportedAiRequest,
  failedAiExport,
  manualHandoff,
  startedAiExport,
  type AiExportAction,
  type AiExportAttempt,
  type AiExportItemView,
  type AiNoteTabLayout,
  type ManualHandoffView,
} from './aiHandoffView';
import {
  aiNoteTab,
  aiNoteTrouble,
  type AiNoteTabView,
  type AiNoteTrouble,
  type NoteSection,
  type NoteView,
} from './aiNoteView';
import {
  NO_COPY_ATTEMPT,
  copiedText,
  failedCopy,
  startedCopy,
  type CopyAction,
  type CopyAttempt,
  type CopyItemView,
} from './copyView';
import { EmptyState } from './EmptyState';
import {
  NO_EXPORT_ATTEMPT,
  exportPanel,
  exportedFile,
  failedExport,
  type ExportAction,
  type ExportAttempt,
  type ExportPanelView,
} from './exportView';
import { FailureNotice } from './FailureNotice';
import { Loading } from './Loading';
import {
  notionPanel,
  notionTrouble,
  type NotionPanelView,
  type NotionSendAction,
  type NotionTrouble,
} from './notionSyncView';
import {
  LOADING_RECORDING_DETAIL,
  MISSING_AUDIO_NOTICE,
  failedRecordingDetail,
  loadedRecordingDetail,
  type RecordingDetailView,
} from './recordingDetailView';
// 만들어진 파일이 놓인 자리를 여는 규칙 (R-4). 어느 OS의 파일 관리자인지는 backend가 안다.
import {
  NO_SHOW_FILE_ATTEMPT,
  failedShowFile,
  showingFile,
  type ShowFileAction,
  type ShowFileAttempt,
  type ShowFileView,
} from './savedFileView';
import {
  LOADING_TRANSCRIPT_TAB,
  transcriptTab,
  transcriptTrouble,
  type TranscriptLine,
  type TranscriptTabView,
  type TranscriptTrouble,
  transcriptParagraphs,
  matchingParagraphs,
  searchNotice,
} from './transcriptView';
import type { ScreenProps } from './types';

// §5.C의 세 탭. AI Note가 없어도 화면은 정상 동작해야 한다 (INV-8).
//
// **값과 표시 이름을 가른다.** 값은 `id`·`aria-controls`가 쓰는 안정적인 식별자이므로
// 화면에 보이는 말이 바뀌어도 따라 바뀌지 않는다.
const TABS = ['AI Note', 'Transcript', 'Recording'] as const;
type Tab = (typeof TABS)[number];

/** 탭에 보이는 이름. */
const TAB_LABEL: Record<Tab, string> = {
  'AI Note': 'AI 노트',
  Transcript: '전사',
  Recording: '녹음',
};

/** 레코드는 있는데 파일이 없을 때 Recording 탭이 말하는 것. */
const NO_AUDIO_TEXT = '아직 오디오 파일이 없다.';

/**
 * 탭 하나와 그 내용을 잇는 식별자 (요구 12).
 *
 * 탭 이름 자체에서 만든다 — 목록과 따로 관리되는 두 번째 표가 생기면 탭이 하나 늘 때 조용히
 * 어긋난다. **탭 구성을 바꾸지 않으므로** 여기서 만드는 것은 이름의 다른 표기일 뿐이다.
 */
const slug = (name: Tab) => name.toLowerCase().replace(/\s+/g, '-');
const tabId = (name: Tab) => `tab-${slug(name)}`;
const panelId = (name: Tab) => `panel-${slug(name)}`;

/**
 * 전사가 도는 동안 상태를 다시 물어보는 간격(밀리초).
 *
 * 화면이 진행 상황을 만들어 내지 않으므로 보이는 것은 언제나 backend가 마지막으로 말해 준
 * 값이다. 녹음 화면의 경과 시간보다 느리게 물어보는 이유는 이 값이 초 단위로 바뀌지 않기
 * 때문이다 — 바뀌는 것은 상태 하나뿐이다.
 */
const TRANSCRIPTION_REFRESH_MS = 1_000;

/**
 * 노트를 만드는 동안 상태를 다시 물어보는 간격(밀리초).
 *
 * 전사보다 느리게 물어본다 — 로컬 모델이 노트 하나를 쓰는 데 걸리는 시간은 초 단위가 아니고
 * (ADR-0008 §16.2), 바뀌는 것은 상태 하나뿐이기 때문이다.
 */
const AI_NOTE_REFRESH_MS = 2_000;

/**
 * Notion으로 보내는 동안 상태를 다시 물어보는 간격(밀리초).
 *
 * 노트 생성과 같은 간격이다 — 요청 하나하나가 왕복이고 속도 제한 때문에 기다리는 구간도 있어
 * (docs/ADR-0009-notion-and-export.md §9.2) 초 단위로 바뀌는 값이 아니다.
 */
const NOTION_REFRESH_MS = 2_000;

/**
 * Recording Detail 화면 (§5.C).
 *
 * 세 가지를 읽는다 — 이 녹음(`get_recording`), **레코드는 있는데 파일이 없는 녹음의 목록**
 * (`list_missing_audio`), 그리고 current Transcript(`get_transcript` · §7.2)다. 파일이 있는지를
 * 화면이 직접 보지 않는 이유는 하나다: 파일시스템을 아는 코드는 Rust 안에만 있다
 * (PRODUCT-SPEC §12 · ADR-0001). 그 목록은 **알리기만 한다** — 부른다고 해서 레코드가
 * 지워지거나 고쳐지지 않는다 (INV-4).
 *
 * 응답을 화면 상태로 바꾸는 규칙은 {@link loadedRecordingDetail} · {@link failedRecordingDetail} ·
 * {@link transcriptTab}에 있다. 여기에는 그리는 일만 있다 — 그래서 재생 경로 네 갈래와
 * 전사 상태 다섯 갈래가 DOM 없이 판정된다 (§18).
 *
 * ## 진행 중인 전사를 이 컴포넌트가 소유하지 않는다
 *
 * 전사는 backend의 배경 스레드에서 돌고 (`src-tauri/src/commands/transcriber.rs`), 화면은
 * `transcription_status`로 **물어본다** — 녹음 화면이 `capture_status`를 물어보는 것과 같은
 * 규약이다 (R-001). 그래서 화면이 unmount돼도 전사는 이어지고, 1시간짜리 녹음을 걸어 둔
 * 동안에도 이 화면은 멎지 않는다. unmount가 하는 일은 되풀이 조회를 멈추는 것뿐이다.
 *
 * ## 기록을 밖으로 꺼내는 문 둘 (§10 · §11)
 *
 * `Export Markdown`과 `Send to Notion`은 탭 위에 나란히 있다 — 어느 탭을 보고 있든 이 녹음의
 * Notion 상태가 보여야 하기 때문이다 (`phase-prompt/05` 요구 9). 여기서도 판단은 순수 모듈에
 * 있다 ({@link exportPanel} · {@link notionPanel}): **AI provider 없이 · AI 노트 없이 내보낼 수
 * 있다는 것**과 **누르면 무슨 일이 일어나는가**가 DOM 없이 판정된다.
 *
 * 내보내기는 상태를 물어보는 규약을 쓰지 않는다 — `export_markdown`이 이미 만들어진 파일을
 * 돌려주기 때문이다. Notion 전송은 전사·노트 생성과 같은 규약이다: backend가 소유하고 화면은
 * 물어본다.
 *
 * 재생은 `<audio controls>` 하나다. 파일 바이트는 IPC를 지나지 않고 asset protocol을 지나며
 * (`recordingAudioSource` · docs/ADR-0006-audio-playback.md), 그 주소는 로컬 webview 안에서만
 * 쓰인다 (INV-6). **실제 재생 음질은 자동으로 판정되지 않는다 — 사람이 확인하는 항목이다**
 * (`phase-prompt/02-reliable-recording.md`의 Human Review).
 */
export function RecordingDetailScreen({ route, goBack }: ScreenProps) {
  const [tab, setTab] = useState<Tab>('Transcript');
  const [view, setView] = useState<RecordingDetailView>(LOADING_RECORDING_DETAIL);
  /** 레코드 그대로. 상세 표시는 {@link view}가 하고, 전사 상태 판단에는 이 값이 필요하다. */
  const [record, setRecord] = useState<Recording | null>(null);
  /** current Transcript (§7.2). 아직 읽지 못했거나 없으면 `null`이다. */
  const [currentTranscript, setCurrentTranscript] = useState<Transcript | null>(null);
  /** backend가 마지막으로 알려준 전사 상태. 아직 물어보지 못했으면 `null`이다. */
  const [live, setLive] = useState<TranscriptionStatus | null>(null);
  /** 거절된 요청 하나. 전사 자체의 실패와 다른 자리에 놓인다 (§13). */
  const [trouble, setTrouble] = useState<TranscriptTrouble | null>(null);
  /** 다시 읽은 횟수. 늘어나면 다시 읽는다. */
  const [attempt, setAttempt] = useState(0);

  /** 고른 AI provider의 지금 상태. 아직 물어보지 못했으면 `null`이다 (INV-8). */
  const [aiProvider, setAiProvider] = useState<AiProviderStatus | null>(null);
  /** current Transcript에서 만들어진 노트 전부. 아직 읽지 못했으면 `null`이다. */
  const [aiNotes, setAiNotes] = useState<readonly AiNote[] | null>(null);
  /** backend가 마지막으로 알려준 노트 생성 상태. 아직 물어보지 못했으면 `null`이다. */
  const [aiLive, setAiLive] = useState<AiNoteStatus | null>(null);
  /** 사용자가 고른 mode (§9.5). 고르는 것은 화면의 일이고 만드는 것은 backend의 일이다. */
  const [noteMode, setNoteMode] = useState<NoteMode>('meeting');
  /** 거절된 AI 관련 요청 하나. 노트 생성 자체의 실패와 다른 자리에 놓인다 (§13). */
  const [aiTrouble, setAiTrouble] = useState<AiNoteTrouble | null>(null);
  /** provider 상태를 다시 물어본 횟수. 늘어나면 다시 물어본다. */
  const [providerAttempt, setProviderAttempt] = useState(0);

  /** 이 화면이 건 Markdown export 한 번 (§11). 상태를 물어보는 규약을 쓰지 않는다. */
  const [exportAttempt, setExportAttempt] = useState<ExportAttempt>(NO_EXPORT_ATTEMPT);
  /**
   * 이 화면이 건 복사 한 번 (요구 A-1 · A-2). 두 복사 자리 중 하나에만 해당한다.
   *
   * **AI provider와 아무 상관이 없다** — 이 값이 만들어지는 경로에 provider가 들어오지 않는다
   * (MH-1 · MH-2).
   */
  const [copyAttempt, setCopyAttempt] = useState<CopyAttempt>(NO_COPY_ATTEMPT);
  /** 이 화면이 건 Export for AI 한 번 (요구 3). Markdown export와 다른 자리, 다른 상태다. */
  const [aiExportAttempt, setAiExportAttempt] = useState<AiExportAttempt>(NO_AI_EXPORT_ATTEMPT);
  /**
   * 이 화면이 건 **자리 열기** 한 번 (`phase-prompt/05.6` 성공 기준 2 · R-4).
   *
   * 두 export 자리가 이 값 하나를 함께 본다 — 가리키는 것이 녹음이 아니라 **파일 경로**이므로,
   * 어느 자리의 결과인지는 그 경로가 말한다 (`savedFileView.ts`의 `showFile`).
   */
  const [showAttempt, setShowAttempt] = useState<ShowFileAttempt>(NO_SHOW_FILE_ATTEMPT);
  /** 디스크에 남아 있는 Notion 전송 기록. 아직 읽지 못했거나 보낸 적이 없으면 `null`이다. */
  const [notionSync, setNotionSync] = useState<NotionSync | null>(null);
  /** backend가 마지막으로 알려준 Notion 전송 상태. 아직 물어보지 못했으면 `null`이다. */
  const [notionLive, setNotionLive] = useState<NotionSendStatus | null>(null);
  /** 거절된 Notion 관련 요청 하나. 전송 자체의 실패와 다른 자리에 놓인다 (§13). */
  const [notionIssue, setNotionIssue] = useState<NotionTrouble | null>(null);

  const recordingId = route.screen === 'recording-detail' ? route.recordingId : null;

  /** 마지막으로 본 전사 상태. 전사가 **끝나는 순간**을 알아보는 데 쓴다. */
  const lastLiveState = useRef<TranscriptionStatus['state'] | null>(null);
  /** 마지막으로 본 노트 생성 상태. 생성이 **끝나는 순간**을 알아보는 데 쓴다. */
  const lastAiState = useRef<AiNoteStatus['state'] | null>(null);
  /** 마지막으로 본 Notion 전송 상태. 전송이 **끝나는 순간**을 알아보는 데 쓴다. */
  const lastNotionState = useRef<NotionSendStatus['state'] | null>(null);

  /** 저장된 값을 다시 읽는다. 화면을 로딩으로 되돌리지 않는다 — 보고 있던 것이 사라지지 않게. */
  const reload = useCallback(() => setAttempt((count) => count + 1), []);

  /** 사용자가 다시 시도했다. 이때는 화면을 로딩으로 되돌린다. */
  const retry = () => {
    setView(LOADING_RECORDING_DETAIL);
    setTrouble(null);
    reload();
  };

  /**
   * 지금 전사가 어떤 상태인지 backend에 물어본다.
   *
   * 화면이 상태를 만들어 내지 않는 자리다. 전사가 끝난 것을 본 순간에는 저장된 값을 다시
   * 읽는다 — 그때 비로소 새 Transcript와 `transcriptionStatus`가 저장돼 있기 때문이다.
   */
  const refreshTranscription = useCallback(() => {
    transcriptionStatus().then(
      (next) => {
        setLive(next);
        const finished = next.state === 'done' || next.state === 'failed';
        if (finished && lastLiveState.current === 'running' && next.recordingId === recordingId) {
          reload();
        }
        lastLiveState.current = next.state;
      },
      // 실패를 console에만 남기지 않는다. 화면 상태가 된다 (§13).
      (error: unknown) => setTrouble(transcriptTrouble('status', error)),
    );
  }, [recordingId, reload]);

  useEffect(() => {
    if (recordingId === null) {
      return;
    }
    // 응답이 오기 전에 화면을 떠났다면 그 응답으로 상태를 바꾸지 않는다.
    let current = true;

    // 셋 중 하나라도 답하지 못하면 실패 상태가 된다 — 파일이 그 자리에 있는지 모르는 채로
    // "재생할 수 있다"고 말하지 않는다. Transcript는 레코드가 가리킬 때만 읽는다 (§7.2).

    getRecording(recordingId)
      .then((recording) =>
        Promise.all([
          Promise.resolve(recording),
          listMissingAudio(),
          recording !== null && recording.currentTranscriptId !== null
            ? getTranscript(recording.currentTranscriptId)
            : Promise.resolve(null),
        ]),
      )
      .then(
        ([recording, missingAudio, currentTranscript]) => {
          if (!current) {
            return;
          }
          setRecord(recording);
          setCurrentTranscript(currentTranscript);
          setView(loadedRecordingDetail(recordingId, recording, missingAudio, recordingAudioSource));
        },
        (error: unknown) => {
          if (current) setView(failedRecordingDetail(error));
        },
      );

    return () => {
      current = false;
    };
  }, [recordingId, attempt]);

  // 화면을 열 때 한 번 물어본다 — 이 화면에 오기 전에 걸어 둔 전사가 돌고 있을 수 있다.
  useEffect(() => {
    refreshTranscription();
  }, [refreshTranscription]);

  /**
   * 고른 AI provider가 지금 어떤 상태인지 물어본다 (INV-8).
   *
   * **이 조회의 실패도 화면의 다른 부분을 막지 않는다.** 실패는 AI Note 탭 안의 알림 하나가
   * 되고, 녹음 · 재생 · Transcript 탭은 그대로 동작한다 — 그것이 이 Phase의 성공 기준 2다.
   */
  useEffect(() => {
    let current = true;
    aiProviderStatus().then(
      (next) => {
        if (current) setAiProvider(next);
      },
      (error: unknown) => {
        if (current) setAiTrouble(aiNoteTrouble('provider', error));
      },
    );
    return () => {
      current = false;
    };
  }, [providerAttempt]);

  /**
   * current Transcript에서 만들어진 노트를 읽는다 (§7.2 · ADR-0008 §9.2).
   *
   * **레코드를 읽는 경로와 갈라 두었다.** 노트를 읽지 못하는 것이 녹음 상세를 못 읽는 것으로
   * 번지면 AI 하나 때문에 재생과 Transcript가 함께 막힌다 (INV-8).
   */
  const noteTranscriptId = record?.currentTranscriptId ?? null;
  useEffect(() => {
    // 가리키는 Transcript가 없으면 읽을 것도 없다. 그 사실은 상태로 만들지 않고 아래에서
    // 그대로 읽는다 — 없는 것을 "빈 목록을 읽어 왔다"로 저장하면 두 사실이 섞인다.
    if (noteTranscriptId === null) {
      return;
    }
    let current = true;
    listAiNotes(noteTranscriptId).then(
      (next) => {
        if (current) setAiNotes(next);
      },
      (error: unknown) => {
        if (current) setAiTrouble(aiNoteTrouble('notes', error));
      },
    );
    return () => {
      current = false;
    };
  }, [noteTranscriptId, attempt]);

  /**
   * 지금 노트 생성이 어떤 상태인지 backend에 물어본다.
   *
   * 전사와 같은 규약이다 — 화면이 상태를 만들어 내지 않는다. 생성이 끝난 것을 본 순간에는
   * 저장된 값을 다시 읽는다: 그때 비로소 새 노트와 `aiStatus`가 저장돼 있기 때문이다.
   */
  const refreshAiNote = useCallback(() => {
    aiNoteStatus().then(
      (next) => {
        setAiLive(next);
        const finished =
          next.state === 'done' || next.state === 'failed' || next.state === 'noTranscript';
        if (finished && lastAiState.current === 'running' && next.recordingId === recordingId) {
          reload();
        }
        lastAiState.current = next.state;
      },
      (error: unknown) => setAiTrouble(aiNoteTrouble('status', error)),
    );
  }, [recordingId, reload]);

  // 화면을 열 때 한 번 물어본다 — 이 화면에 오기 전에 걸어 둔 생성이 돌고 있을 수 있다.
  useEffect(() => {
    refreshAiNote();
  }, [refreshAiNote]);

  /**
   * 이 녹음의 **저장된** Notion 전송 기록을 읽는다 (§7 · ADR-0009 §8.4).
   *
   * 레코드를 읽는 경로와 갈라 두었다 — 전송 기록을 읽지 못하는 것이 녹음 상세를 못 읽는 것으로
   * 번지면 Notion 하나 때문에 재생과 Transcript가 함께 막힌다 (INV-8). 보낸 적이 없으면 `null`이
   * 오며 그것은 오류가 아니다.
   */
  useEffect(() => {
    if (recordingId === null) {
      return;
    }
    let current = true;
    getNotionSync(recordingId).then(
      (next) => {
        if (current) setNotionSync(next);
      },
      (error: unknown) => {
        if (current) setNotionIssue(notionTrouble('sync', error));
      },
    );
    return () => {
      current = false;
    };
  }, [recordingId, attempt]);

  /**
   * 지금 Notion 전송이 어떤 상태인지 backend에 물어본다.
   *
   * 전사 · 노트 생성과 같은 규약이다 — 화면이 상태를 만들어 내지 않는다. 전송이 끝난 것을 본
   * 순간에는 저장된 값을 다시 읽는다: 그때 비로소 새 `notion_syncs` 행과 `notionStatus`가
   * 저장돼 있기 때문이다.
   */
  const refreshNotion = useCallback(() => {
    notionSyncStatus().then(
      (next) => {
        setNotionLive(next);
        const finished = next.state === 'done' || next.state === 'failed';
        if (finished && lastNotionState.current === 'running' && next.recordingId === recordingId) {
          reload();
        }
        lastNotionState.current = next.state;
      },
      (error: unknown) => setNotionIssue(notionTrouble('status', error)),
    );
  }, [recordingId, reload]);

  // 화면을 열 때 한 번 물어본다 — 이 화면에 오기 전에 걸어 둔 전송이 돌고 있을 수 있다.
  useEffect(() => {
    refreshNotion();
  }, [refreshNotion]);

  // 레코드를 아직 읽지 못한 것은 "전사가 없다"가 아니다 — 그 둘을 접지 않는다.
  const transcriptView =
    record === null ? LOADING_TRANSCRIPT_TAB : transcriptTab(record, currentTranscript, live);
  const transcribing = transcriptView.kind === 'running' || transcriptView.kind === 'pending';

  useEffect(() => {
    // 전사가 도는 동안에만 되풀이해 물어본다. 상태 조회는 전사를 기다리지 않으므로
    // (`Transcriber::status`) 이 되풀이가 화면을 멎게 하지 않는다.
    if (!transcribing) {
      return;
    }
    const timer = setInterval(refreshTranscription, TRANSCRIPTION_REFRESH_MS);
    return () => clearInterval(timer);
  }, [transcribing, refreshTranscription]);

  // 어느 상태가 되든 이 탭은 재생과 Transcript 탭을 막지 않는다 (INV-8). 판단은 전부
  // aiNoteTab에 있고 여기에는 없다.
  const aiView = aiNoteTab({
    recording: record,
    transcript: currentTranscript,
    // 가리키는 Transcript가 없으면 노트도 없다 (§7.2) — 읽지 못한 것과 다른 사실이다.
    notes: noteTranscriptId === null ? [] : aiNotes,
    provider: aiProvider,
    live: aiLive,
    mode: noteMode,
  });
  const generatingNote = aiView.body.kind === 'generating';

  // AI Note 탭의 두 줄 (요구 6). **아래 줄은 provider를 보지 않는다** — `manualHandoff`의
  // 입력에 provider를 담을 자리가 없으므로, provider가 하나도 없어도 세 동작이 그대로 가능하다
  // (MH-1 · MH-2). 위 줄의 상태는 `aiNoteTab`이 만든 값 그대로 실린다 (MH-8). 위계를 정하는
  // 규칙은 여기 없고 `aiHandoffView.ts`에 있다.
  const aiLayout = aiNoteTabLayout(
    aiView,
    manualHandoff({
      recording: record,
      mode: noteMode,
      copy: copyAttempt,
      aiExport: aiExportAttempt,
      show: showAttempt,
    }),
  );

  useEffect(() => {
    // 노트를 만드는 동안에만 되풀이해 물어본다. 상태 조회는 생성을 기다리지 않으므로
    // (`NoteGenerator::status`) 이 되풀이가 화면을 멎게 하지 않는다.
    if (!generatingNote) {
      return;
    }
    const timer = setInterval(refreshAiNote, AI_NOTE_REFRESH_MS);
    return () => clearInterval(timer);
  }, [generatingNote, refreshAiNote]);

  // 내보내기와 Notion 전송. 어느 쪽도 재생 · Transcript · AI Note 탭을 막지 않으며, 판단은
  // 전부 exportPanel · notionPanel에 있고 여기에는 없다.
  const exportView = exportPanel({
    recording: record,
    // 가리키는 Transcript가 없으면 노트도 없다 (§7.2) — 읽지 못한 것과 다른 사실이다.
    notes: noteTranscriptId === null ? [] : aiNotes,
    attempt: exportAttempt,
    show: showAttempt,
  });
  const notionView = notionPanel({ recording: record, sync: notionSync, live: notionLive });
  const sendingToNotion = notionView.body.kind === 'sending';

  useEffect(() => {
    // 보내는 동안에만 되풀이해 물어본다. 상태 조회는 전송을 기다리지 않으므로
    // (`NotionSender::status`) 이 되풀이가 화면을 멎게 하지 않는다.
    if (!sendingToNotion) {
      return;
    }
    const timer = setInterval(refreshNotion, NOTION_REFRESH_MS);
    return () => clearInterval(timer);
  }, [sendingToNotion, refreshNotion]);

  /**
   * 이 화면에서 전사를 시작한다 — 처음이든 실패한 뒤든 같은 동작이다 (요구 2 · 7).
   *
   * 돌아오는 것은 접수 사실이지 전사 결과가 아니다. 거절되면 그 사실이 화면에 남는다 —
   * 조용히 사라지지 않는다.
   */
  const beginTranscription = (id: string) => {
    setTrouble(null);
    startTranscription(id).then(
      (next) => {
        setLive(next);
        lastLiveState.current = next.state;
      },
      (error: unknown) => setTrouble(transcriptTrouble('start', error)),
    );
  };

  /**
   * 이 화면에서 노트를 만든다 — 처음이든, 다시 만드는 것이든, 실패한 뒤든 같은 동작이다
   * (요구 12 · ADR-0008 §9.2). 다시 만들어도 이전 노트는 지워지지 않고 하나 더 생긴다.
   *
   * 돌아오는 것은 접수 사실이지 노트가 아니다. 거절되면 그 사실이 화면에 남는다 —
   * 조용히 사라지지 않는다.
   */
  const beginAiNote = (id: string, mode: NoteMode) => {
    setAiTrouble(null);
    startAiNote(id, mode).then(
      (next) => {
        setAiLive(next);
        lastAiState.current = next.state;
      },
      (error: unknown) => setAiTrouble(aiNoteTrouble('start', error)),
    );
  };

  /**
   * 이 녹음을 로컬 Markdown 파일로 내보낸다 (§11 · 요구 A-1).
   *
   * 돌아오는 것은 **이미 만들어진 파일**이지 접수 사실이 아니다 — 기다릴 모델도 서버도 없다.
   * 실패해도 녹음 · 전사 · 노트는 그대로이며 (INV-3), 그 사실은 실패 상태가 들고 있다.
   */
  const beginExport = (id: string) => {
    setExportAttempt({ kind: 'running', recordingId: id });
    exportMarkdown(id).then(
      (file) => setExportAttempt(exportedFile(file)),
      // 실패를 console에만 남기지 않는다. 화면 상태가 된다 (§13).
      (error: unknown) => setExportAttempt(failedExport(id, error)),
    );
  };

  /**
   * 이 녹음의 프롬프트나 전사 텍스트를 clipboard에 올린다 (요구 A-1 · A-2 · 5).
   *
   * 두 경계를 차례로 지난다 — 문자열을 만드는 것은 backend이고 (`get_ai_prompt` ·
   * `get_transcript_text`), 그것을 clipboard에 쓰는 것은 `platform/clipboard`다. **어느 쪽이
   * 거절해도 사용자에게는 "복사되지 않았다"는 하나의 사건이며**, 무엇 때문이었는지는 순수
   * 모듈이 가른다 (`copyView`의 `CopyFailureCause`). 실패를 console로 흘려보내지 않는다 (§13).
   *
   * **AI provider가 들어오지 않는다** — 두 command 어디에도 provider 인자가 없으므로 provider가
   * 없다는 이유로 거절당할 수단이 없다 (MH-1 · MH-2 · INV-8).
   */
  const beginCopy = (action: CopyAction) => {
    const { target, recordingId: id, mode, portion } = action;
    setCopyAttempt(startedCopy(target, id, portion));
    // 프롬프트에는 mode가 실려 있고 전사에는 없다 — 그 구분은 값이 들고 있다. 몇 번째 조각을
    // 가져올지도 값이 들고 있다 (`phase-prompt/05.6` 성공 기준 4).
    const taken = mode === null ? getTranscriptText(id, portion) : getAiPrompt(id, mode, portion);
    taken.then(
      (value) =>
        // `copyText`는 예외를 던지지 않는다. 결과가 값으로 온다 (ADR-0010 §7).
        copyText(value.text).then((result) =>
          setCopyAttempt(
            result.ok
              ? // 크기와 자리를 그대로 들고 간다 — 화면이 그것을 다시 세지 않는다.
                copiedText(target, id, value)
              : failedCopy(target, id, portion, result.failure),
          ),
        ),
      (error: unknown) => setCopyAttempt(failedCopy(target, id, portion, error)),
    );
  };

  /**
   * 이 녹음을 **AI에게 줄 문서 하나**로 내보낸다 (요구 3 · ADR-0010 §5.2).
   *
   * Markdown export와 같은 규약이다 — 돌아오는 것은 이미 만들어진 파일이며, 기다릴 서버도
   * 모델도 없다. **clipboard를 전혀 쓰지 않으므로 복사가 거절되는 환경에서도 이 길은 남는다**
   * (§7.5). 실패해도 녹음 · 전사 · 노트 · 이미 내보낸 파일은 그대로다 (INV-3 · MH-7).
   *
   * **문서가 나뉘면 한 번에 조각 하나다** (`phase-prompt/05.6` 성공 기준 4). 몇 번째를 쓸지는
   * 누른 동작이 들고 있고, 돌아온 값이 그것이 몇 번째였는지 말한다 — 이 자리가 세지 않는다.
   */
  const beginAiExport = (action: AiExportAction) => {
    const { recordingId: id, mode, portion } = action;
    setAiExportAttempt(startedAiExport(id, mode, portion));
    exportAiRequest(id, mode, portion).then(
      (written) => setAiExportAttempt(exportedAiRequest(written)),
      (error: unknown) => setAiExportAttempt(failedAiExport(id, portion, error)),
    );
  };

  /**
   * 방금 만들어진 파일이 **놓인 자리를 연다** (`phase-prompt/05.6` 성공 기준 2 · R-4).
   *
   * 경로를 보여 주는 것은 그대로다 — **이것은 그 대체가 아니라 추가다.** 무엇을 열어도 되는지
   * 정하는 것은 backend이며 (`src-tauri/src/commands/saved_file.rs`), 이 화면은 backend가 준
   * 경로를 그대로 되돌려 줄 뿐 다른 경로를 지어내지 않는다.
   *
   * 열지 못하면 그 사실이 화면 상태가 된다 — console로 흘려보내지 않으며 (§13), 그때에도 파일과
   * 경로는 그대로다 (INV-3). 열렸으면 상태를 되돌린다: 이 자리에 남길 사실이 없다.
   */
  const beginShowFile = (action: ShowFileAction) => {
    const { path } = action;
    setShowAttempt(showingFile(path));
    showSavedFile(path).then(
      () => setShowAttempt(NO_SHOW_FILE_ATTEMPT),
      (error: unknown) => setShowAttempt(failedShowFile(path, error)),
    );
  };

  /**
   * 이 녹음을 Notion으로 보내기 시작한다 (§10 · ADR-0009 §8).
   *
   * 돌아오는 것은 접수 사실이지 전송 결과가 아니다. **`confirmation`을 화면이 스스로 고르지
   * 않는다** — 값은 사용자가 누른 버튼이 들고 있던 것 그대로이며, 그래서 확인 없이 페이지가
   * 하나 더 생기는 경로가 없다 (§8.3). 거절되면 그 사실이 화면에 남는다.
   */
  const beginNotionSend = (id: string, confirmation: NotionConfirmation) => {
    setNotionIssue(null);
    startNotionSync(id, confirmation).then(
      (next) => {
        setNotionLive(next);
        lastNotionState.current = next.state;
      },
      (error: unknown) => setNotionIssue(notionTrouble('start', error)),
    );
  };

  /** provider 상태를 다시 물어본다. **실패의 재시도가 아니라 확인이다** (INV-8). */
  const recheckProvider = () => {
    setAiTrouble(null);
    setProviderAttempt((count) => count + 1);
  };

  // 대상 recording 없이 이 화면에 도달하는 것도 정상 상태다.
  if (recordingId === null) {
    return (
      <div className="screen">
        <EmptyState title="No recording selected.">
          <button type="button" className="btn btn--secondary" onClick={goBack}>
            Back
          </button>
        </EmptyState>
      </div>
    );
  }

  if (view.kind === 'loading') {
    return (
      <div className="screen">
        <Loading text="Loading recording…" />
      </div>
    );
  }

  if (view.kind === 'failed') {
    return (
      <div className="screen">
        <FailureNotice
          failure={view.failure}
          headline="This recording could not be read."
          onRetry={retry}
        />
        <button type="button" className="btn btn--secondary" onClick={goBack}>
          Back
        </button>
      </div>
    );
  }

  if (view.kind === 'notFound') {
    return (
      <div className="screen">
        {/* 목록에서 사라진 것도 실패가 아니다 (INV-3 · INV-4). 어느 레코드였는지는 남긴다. */}
        <EmptyState title="This recording is no longer in the list.">
          <p className="t-caption">{view.recordingId}</p>
          <button type="button" className="btn btn--secondary" onClick={goBack}>
            Back
          </button>
        </EmptyState>
      </div>
    );
  }

  const { recording } = view;

  return (
    <div className="screen">
      <p className="detail__title">{recording.title}</p>
      <p className="hint">
        {recording.recordedAtLabel} · {recording.durationLabel}
      </p>

      {view.kind === 'playable' ? (
        <div className="detail__player">
          {/* 파일은 asset protocol로 흐른다. 주소를 만드는 것은 ipc 모듈의 일이다. */}
          <audio
            className="detail__audio"
            controls
            preload="metadata"
            aria-label={`Play ${recording.title}`}
            src={view.audioSource}
          />
        </div>
      ) : (
        // 파일이 없다는 사실을 보여줄 뿐 아무것도 지우지 않는다 (INV-3 · INV-4).
        <div className="detail__player detail__player--missing" role="status">
          <p className="detail__missing">오디오 파일을 찾지 못함</p>
          <p className="hint">{MISSING_AUDIO_NOTICE}</p>
          <p className="detail__path">{view.audioPath}</p>
        </div>
      )}

      {/* 기록을 이 앱 밖으로 꺼내는 문 둘 (§10 · §11). 탭 위에 있으므로 어느 탭을 보고
          있든 Notion 상태가 보인다 (요구 9). */}
      <section className="share">
        <ExportPanel panel={exportView} onExport={beginExport} onShowFile={beginShowFile} />
        <NotionPanel panel={notionView} trouble={notionIssue} onSend={beginNotionSend} />
      </section>

      {/* 탭 셋은 그대로다 — 이 Task가 바꾼 것은 여백과 활성 표시, 그리고 어느 탭이 어느
          내용을 여는지 보조기술에도 이어 준 것뿐이다 (요구 10 · 12). */}
      <div className="tabs" role="tablist">
        {TABS.map((name) => (
          <button
            key={name}
            type="button"
            role="tab"
            id={tabId(name)}
            aria-selected={tab === name}
            aria-controls={panelId(name)}
            className={tab === name ? 'tabs__tab tabs__tab--active' : 'tabs__tab'}
            onClick={() => setTab(name)}
          >
            {TAB_LABEL[name]}
          </button>
        ))}
      </div>

      <div
        className="tabs__panel"
        role="tabpanel"
        id={panelId(tab)}
        aria-labelledby={tabId(tab)}
        tabIndex={0}
      >
        {tab === 'Transcript' && (
          <TranscriptTab tab={transcriptView} trouble={trouble} onTranscribe={beginTranscription} />
        )}

        {tab === 'Recording' && (
          <p className="hint">
            {view.kind === 'playable'
              ? `${view.audioFormat} · ${recording.durationLabel}`
              : NO_AUDIO_TEXT}
          </p>
        )}

        {tab === 'AI Note' && (
          <AiNoteTab
            layout={aiLayout}
            trouble={aiTrouble}
            onMode={setNoteMode}
            onGenerate={beginAiNote}
            onRecheck={recheckProvider}
            onCopy={beginCopy}
            onExportForAi={beginAiExport}
            onShowFile={beginShowFile}
          />
        )}
      </div>
    </div>
  );
}

/**
 * Markdown export 자리 (§5 C · §11 · `phase-prompt/05` 요구 A-1~3).
 *
 * 여섯 상태가 서로 다른 모습을 갖는다. 어느 상태인지 정하는 규칙은 여기 없고
 * {@link exportPanel}에 있다 — 이 컴포넌트는 그리기만 한다.
 *
 * **AI에 대한 참조가 하나도 없다** (INV-8). 만들어진 파일의 **전체 경로는 그대로 보이고**,
 * 이제 그 자리를 여는 수단이 그 옆에 하나 더 있다 (`phase-prompt/05.6` 성공 기준 2 · R-4) —
 * 경로만으로는 파일에 도달하지 못한다는 것이 실사용에서 드러났기 때문이다. 대체가 아니라
 * 추가이며, 그 판정도 이 컴포넌트가 아니라 `savedFileView.ts`에 있다.
 */
function ExportPanel({
  panel,
  onExport,
  onShowFile,
}: {
  panel: ExportPanelView;
  onExport: (recordingId: string) => void;
  onShowFile: (action: ShowFileAction) => void;
}) {
  const { body, contents } = panel;

  return (
    <section className="share__panel">
      <h2 className="share__title">마크다운</h2>

      {body.kind === 'loading' && <Loading text="Loading…" />}

      {body.kind === 'nothingToExport' && (
        // 재료가 아직 없는 것은 실패가 아니다 (§7.2).
        <EmptyState title={body.text} body={body.hint} />
      )}

      {body.kind === 'ready' && (
        <>
          <p className="hint">{body.text}</p>
          <ExportButton action={body.start} onExport={onExport} />
        </>
      )}

      {/* 파일을 만드는 동안 이 자리가 멎은 것처럼 보이지 않게 한다 (요구 9). */}
      {body.kind === 'exporting' && <Loading text={body.text} live />}

      {body.kind === 'done' && (
        <div className="share__done" role="status">
          <p className="share__headline">{body.file.headline}</p>
          <p className="share__file">{body.file.fileName}</p>
          {/* 어디에 만들어졌는가 (§4.1). **이 줄은 그대로 남는다** — 여는 수단은 그 대체가
              아니라 추가다 (성공 기준 2). */}
          <p className="share__path">{body.file.path}</p>
          {/* 그 자리를 연다 (R-4). 경로를 알아도 걸어 들어갈 수 없는 자리가 있다. */}
          <ShowFileControl show={body.file.show} onShowFile={onShowFile} />
          <p className="hint">{body.text}</p>
          <ExportButton action={body.again} onExport={onExport} />
        </div>
      )}

      {body.kind === 'failed' && (
        <>
          {/* 무엇이 실패했는지 · 원본은 안전한지 · 다시 시도할 수 있는지 (§13). */}
          <FailureNotice failure={body.failure} headline={body.headline} />
          {/* 앱은 아무것도 지우지 않았다 (INV-3). */}
          <p className="hint">{body.preservedNotice}</p>
          {body.resolution !== null && <p className="hint">{body.resolution}</p>}
          <ExportButton action={body.retry} onExport={onExport} />
        </>
      )}

      {/* 무엇이 파일에 들어가는가 · 오디오는 복사되지 않는다 (§11 · INV-6). */}
      <div className="share__contents">
        <p className="share__contents-title">{contents.headline}</p>
        <ul className="share__items">
          {contents.items.map((item) => (
            <li key={item} className="share__item">
              {item}
            </li>
          ))}
        </ul>
        <p className="hint">{contents.noteText}</p>
        <p className="hint">{contents.audioNotice}</p>
      </div>
    </section>
  );
}

/**
 * 만들어진 파일이 **놓인 자리를 여는** 수단 (`phase-prompt/05.6` 성공 기준 2 · R-4).
 *
 * Markdown export 자리와 Export for AI 자리가 **같은 컴포넌트를 쓴다** — 두 자리에서 같은 일을
 * 하므로 그리는 방식이 갈라질 이유가 없다. 어느 상태인지 정하는 규칙은 여기 없고
 * `savedFileView.ts`의 `showFile`에 있다: 이 컴포넌트는 그리기만 한다.
 *
 * **경로 줄을 대체하지 않는다.** 이것은 그 옆에 놓이며, 열지 못했을 때 사용자에게 남는 길이
 * 바로 그 경로라는 사실도 값으로 온다 ({@link ShowFileView.trouble}).
 */
function ShowFileControl({
  show,
  onShowFile,
}: {
  show: ShowFileView;
  onShowFile: (action: ShowFileAction) => void;
}) {
  return (
    <div className="share__show">
      <button
        type="button"
        className="btn btn--secondary"
        disabled={show.showing}
        onClick={() => onShowFile(show.action)}
      >
        {show.action.label}
      </button>
      {/* 여는 중이라는 것도, 이 자리가 무엇인지도 문장으로 있다 (요구 12의 규약). */}
      <p className="hint">{show.text}</p>

      {show.trouble !== null && (
        <>
          {/* 무엇이 실패했는지 · 원본은 안전한지 · 다시 시도할 수 있는지 (§13). */}
          <FailureNotice failure={show.trouble.failure} headline={show.trouble.headline} />
          {/* 파일도 저장된 것도 그대로다 (INV-3). */}
          <p className="hint">{show.trouble.preservedNotice}</p>
          {/* 그래도 도달할 길이 있다 — 위에 그대로 있는 경로다. */}
          <p className="hint">{show.trouble.resolution}</p>
        </>
      )}
    </div>
  );
}

function ExportButton({
  action,
  onExport,
}: {
  action: ExportAction;
  onExport: (recordingId: string) => void;
}) {
  return (
    <button
      type="button"
      className="btn btn--secondary"
      onClick={() => onExport(action.recordingId)}
    >
      {action.label}
    </button>
  );
}

/**
 * Notion 전송 자리 (§5 C · §10 · `phase-prompt/05` 요구 9 · 12 · 13).
 *
 * 일곱 상태가 서로 다른 모습을 갖는다. 어느 상태인지 정하는 규칙은 여기 없고
 * {@link notionPanel}에 있다 — 이 컴포넌트는 그리기만 한다.
 *
 * **누르기 전에 무슨 일이 일어나는지가 버튼 옆에 있다** ({@link NotionSendAction.outcomeText} ·
 * ADR-0009 §8.5). 그리고 **확인이 필요한 상태를 {@link FailureNotice}로 그리지 않는다** —
 * 아무것도 하지 않고 거절된 요청이며, 사용자가 할 일은 고르는 것뿐이다.
 */
function NotionPanel({
  panel,
  trouble,
  onSend,
}: {
  panel: NotionPanelView;
  trouble: NotionTrouble | null;
  onSend: (recordingId: string, confirmation: NotionConfirmation) => void;
}) {
  const { body, contents, status } = panel;

  return (
    <section className="share__panel">
      <h2 className="share__title">
        Notion
        {/* 이 녹음의 Notion 상태 (§7 · 요구 9). 목록과 같은 규칙으로 만들어진 값이다. */}
        {status !== null && <span className="status share__status">{status.text}</span>}
      </h2>

      {/* 요청이 거절된 사실은 전송 상태를 덮지 않고 그 옆에 남는다 (§13). */}
      {trouble !== null && <FailureNotice failure={trouble.failure} headline={trouble.headline} />}

      {body.kind === 'loading' && <Loading text="Loading…" />}

      {body.kind === 'nothingToSend' && <EmptyState title={body.text} body={body.hint} />}

      {body.kind === 'ready' && (
        <>
          <p className="empty">{body.text}</p>
          <SendButton action={body.send} onSend={onSend} />
        </>
      )}

      {/* 왕복이 여럿이라 오래 걸린다 (§9.2). 어디까지 갔는지가 고리 옆에 함께 있다. */}
      {body.kind === 'sending' && (
        <>
          <Loading text={body.text} live />
          {body.progress !== null && <p className="hint">{body.progress.text}</p>}
        </>
      )}

      {body.kind === 'sent' && (
        <div className="share__done" role="status">
          <p className="share__headline">{body.headline}</p>
          {body.pageId !== null && <p className="share__path">{body.pageId}</p>}
          {body.syncedAt !== null && <p className="hint">{body.syncedAt}</p>}
          {body.progress !== null && <p className="hint">{body.progress.text}</p>}
          {/* 또 보내면 무슨 일이 일어나는가 — **누르기 전에** 적혀 있다 (§8.3). */}
          <SendButton action={body.again} onSend={onSend} />
        </div>
      )}

      {body.kind === 'needsConfirmation' && (
        // 경고가 아니다. 아무것도 하지 않고 거절된 요청이며, 사용자가 고를 차례다 (§8.5).
        <div className="share__confirm" role="status">
          <p className="share__headline">{body.headline}</p>
          <p className="hint">{body.text}</p>
          {body.progress !== null && <p className="hint">{body.progress.text}</p>}
          <p className="hint">{body.preservedNotice}</p>
          <SendButton action={body.confirm} onSend={onSend} />
        </div>
      )}

      {body.kind === 'failed' && (
        <>
          {/* 무엇이 실패했는지 · 원본은 안전한지 · 다시 시도할 수 있는지 (§13). */}
          {body.failure !== null && (
            <FailureNotice failure={body.failure} headline={body.headline} />
          )}
          {body.failure === null && <p className="failure__headline">{body.headline}</p>}
          {/* 부분 전송이었다면 그 사실이 드러난다 (§8.4 · 요구 12). */}
          {body.progress !== null && <p className="hint">{body.progress.text}</p>}
          {/* 녹음도 전사도 노트도, 이미 Notion에 있는 것도 그대로다 (INV-3). */}
          <p className="hint">{body.preservedNotice}</p>
          {body.resolution !== null && <p className="hint">{body.resolution}</p>}
          <SendButton action={body.retry} onSend={onSend} />
        </>
      )}

      {/* 무엇이 전송되는가 · 오디오는 나가지 않는다 (INV-5 · INV-6 · 요구 13). */}
      <div className="share__contents">
        <p className="share__contents-title">{contents.headline}</p>
        <ul className="share__items">
          {contents.items.map((item) => (
            <li key={item} className="share__item">
              {item}
            </li>
          ))}
        </ul>
        <p className="hint">{contents.audioNotice}</p>
      </div>
    </section>
  );
}

/**
 * 보내는 버튼 하나.
 *
 * **어떤 확인을 싣는지는 이 컴포넌트가 정하지 않는다** — 값은 {@link NotionSendAction}에 있고,
 * 그래서 확인 없이 새 페이지를 만드는 버튼이 실수로 만들어질 수 없다 (ADR-0009 §8.3).
 */
function SendButton({
  action,
  onSend,
}: {
  action: NotionSendAction;
  onSend: (recordingId: string, confirmation: NotionConfirmation) => void;
}) {
  return (
    <>
      {/* 누르면 무슨 일이 일어나는가 — 버튼보다 먼저 읽힌다 (§8.5 · 요구 8). */}
      <p className="hint">{action.outcomeText}</p>
      <button
        type="button"
        className="btn btn--secondary"
        onClick={() => onSend(action.recordingId, action.confirmation)}
      >
        {action.label}
      </button>
    </>
  );
}

/**
 * Transcript 탭 (§5 C · `phase-prompt/03` 요구 6 · 7).
 *
 * 다섯 상태가 서로 다른 모습을 갖는다. 어느 상태인지 정하는 규칙은 여기 없고
 * {@link transcriptTab}에 있다 — 이 컴포넌트는 그리기만 한다.
 */
function TranscriptTab({
  tab,
  trouble,
  onTranscribe,
}: {
  tab: TranscriptTabView;
  trouble: TranscriptTrouble | null;
  onTranscribe: (recordingId: string) => void;
}) {
  return (
    <section className="transcript">
      {/* 요청이 거절된 사실은 전사 상태를 덮지 않고 그 옆에 남는다 (§13). */}
      {trouble !== null && <FailureNotice failure={trouble.failure} headline={trouble.headline} />}

      {tab.kind === 'loading' && <Loading text="Loading transcript…" />}

      {tab.kind === 'none' && (
        // 빈 상태 둘 — 아직 전사가 없다. 여기서 할 수 있는 일 하나가 그 아래에 있다.
        <EmptyState title={tab.text}>
          <button
            type="button"
            className="btn btn--primary"
            onClick={() => onTranscribe(tab.start.recordingId)}
          >
            {tab.start.label}
          </button>
        </EmptyState>
      )}

      {(tab.kind === 'pending' || tab.kind === 'running') && (
        <>
          {/* 상태가 바뀌는 것은 소리로도 알린다. 화면은 이 동안에도 멎지 않으며, 오래
              걸리는 동안 그 사실이 고리와 글자로 함께 보인다 (요구 9). */}
          <Loading
            text={
              tab.kind === 'running' && tab.progressLabel !== null
                ? `${tab.text} · ${tab.progressLabel}`
                : tab.text
            }
            live
          />
          {/* **나오는 대로 보여 준다** (2026-09-07). 72분짜리 녹음이 도는 동안 사람이 아무것도
              보지 못한 채 수십 분을 기다리던 자리다. 이것은 저장된 Transcript가 아니라
              미리보기이므로, 그 사실을 감추지 않고 함께 적는다. */}
          {tab.kind === 'running' && tab.partial.length > 0 && (
            <>
              <p className="hint">
                Live preview — not saved yet. Some lines may change or be dropped when the
                transcription finishes.
              </p>
              <TranscriptLines lines={tab.partial} />
            </>
          )}
          {/* 새 전사가 도는 동안에도 이미 있던 Transcript는 그대로 보인다 (§7.1 · INV-2). */}
          <TranscriptLines lines={tab.kept} />
        </>
      )}

      {tab.kind === 'done' && (
        <>
          <p className="hint">
            {[tab.language, tab.engine, tab.model].filter((fact) => fact !== null).join(' · ')}
          </p>
          {/* 이 전사에 얼마나 걸렸는가 (phase-prompt/05.6 성공 기준 3). 문장은 Rust가 만든
              것을 그대로 쓴다. **재지 않은 옛 Transcript에서는 이 줄이 아예 없다** —
              모르는 것을 0초라고 말하지 않는다. */}
          {tab.transcriptionLabel !== null && (
            <p className="hint">Transcribed in {tab.transcriptionLabel}</p>
          )}
          {/* 같은 녹음을 다시 전사한다 (2026-09-08). 이 자리에 수단이 없어서 저장소에
              레코드를 직접 넣어 우회한 적이 있다. **잃는 것이 없다는 사실을 버튼 옆에
              둔다** — 그 말이 없으면 누르기를 망설이고, 그러면 우회가 다시 생긴다. */}
          <button
            type="button"
            className="btn btn--secondary"
            onClick={() => onTranscribe(tab.redo.recordingId)}
          >
            {tab.redo.label}
          </button>
          <p className="hint">{tab.redoNotice}</p>
          <TranscriptLines lines={tab.lines} />
        </>
      )}

      {tab.kind === 'failed' && (
        <>
          {/* 무엇이 실패했는지 · 원본은 안전한지 · 다시 시도할 수 있는지 (§13). */}
          {tab.failure !== null && (
            <FailureNotice failure={tab.failure} headline={tab.headline} />
          )}
          {tab.failure === null && <p className="failure__headline">{tab.headline}</p>}
          {/* 앱은 아무것도 지우지 않았다 (INV-1 · INV-2 · INV-3). */}
          <p className="hint">{tab.preservedNotice}</p>
          {/* 모델이 없거나 · 쓸 수 없거나 · 전사가 붕괴한 경우는 먼저 할 일이 다르다.
              어느 갈래가 어떤 문장을 갖는지 정하는 것은 `transcriptView`이며, 여기서는
              있으면 그린다. */}
          {tab.resolution !== null && <p className="hint">{tab.resolution}</p>}
          <button
            type="button"
            className="btn btn--secondary"
            onClick={() => onTranscribe(tab.retry.recordingId)}
          >
            {tab.retry.label}
          </button>
          <TranscriptLines lines={tab.kept} />
        </>
      )}
    </section>
  );
}

/**
 * AI Note 탭 (§5 C · §9 · `phase-prompt/04` 요구 4 · 12 · 14 · 15 ·
 * `phase-prompt/05.5` 요구 6).
 *
 * **노트를 얻는 길이 둘이고, 둘은 위아래로 놓인다** — 위는 연결된 provider가 여기서 쓰는 것,
 * 아래는 사용자가 이미 쓰는 AI에 가져가는 것이다. **아래 줄은 provider가 하나도 없어도 완전히
 * 쓸 수 있다** (MH-1 · MH-2). 어느 줄이 어떤 상태인지 정하는 규칙은 여기 없고
 * {@link aiNoteTab} · {@link manualHandoff} · {@link aiNoteTabLayout}에 있다 — 이 컴포넌트는
 * 그리기만 한다.
 *
 * 세 mode는 두 줄 위에 있다 (§9.5). 프롬프트와 AI-ready 문서도 같은 mode로 만들어지므로 그
 * 선택은 어느 한 줄의 것이 아니며, **provider가 없다는 이유로 잠기지 않는다.**
 */
function AiNoteTab({
  layout,
  trouble,
  onMode,
  onGenerate,
  onRecheck,
  onCopy,
  onExportForAi,
  onShowFile,
}: {
  layout: AiNoteTabLayout;
  trouble: AiNoteTrouble | null;
  onMode: (mode: NoteMode) => void;
  onGenerate: (recordingId: string, mode: NoteMode) => void;
  onRecheck: () => void;
  onCopy: (action: CopyAction) => void;
  onExportForAi: (action: AiExportAction) => void;
  onShowFile: (action: ShowFileAction) => void;
}) {
  return (
    <section className="note">
      {/* 요청이 거절된 사실은 노트 상태를 덮지 않고 그 옆에 남는다 (§13). */}
      {trouble !== null && <FailureNotice failure={trouble.failure} headline={trouble.headline} />}

      {/* 무엇을 만들 것인가 (§9.5). 지금 바꿀 수 있는지는 화면이 아니라 값이 말한다. */}
      <div className="note__modes" role="group" aria-label="Note mode">
        {layout.modes.map((choice) => (
          <button
            key={choice.mode}
            type="button"
            className={choice.selected ? 'note__mode note__mode--active' : 'note__mode'}
            aria-pressed={choice.selected}
            disabled={!layout.modeSelectable}
            title={choice.sections}
            onClick={() => onMode(choice.mode)}
          >
            {choice.label}
          </button>
        ))}
      </div>

      <AutomaticNote row={layout.automatic} onGenerate={onGenerate} onRecheck={onRecheck} />

      {/* 위 줄은 조건이 아니다 — 둘 중 하나를 고르는 것이다. */}
      <p className="note__or">{layout.orText}</p>

      <ManualHandoff
        manual={layout.manual}
        onCopy={onCopy}
        onExportForAi={onExportForAi}
        onShowFile={onShowFile}
      />
    </section>
  );
}

/**
 * 위 줄 — 연결된 provider가 여기서 노트를 쓴다 (§9 · `phase-prompt/04`).
 *
 * 일곱 상태가 서로 다른 모습을 갖는다. **이 Phase가 바꾼 것은 이 줄이 놓이는 자리와 그 위의
 * 제목 두 줄뿐이며, 상태 표현은 그대로다** (MH-8).
 *
 * **AI가 꺼져 있는 상태를 {@link FailureNotice}로 그리지 않는다** (INV-8 · §13). 그 자리에는
 * `role="alert"`도 없고 `Failure`도 없다 — 담담한 사실 몇 줄과, 다시 확인해 볼 수단과,
 * **그것이 선택이라는 사실**뿐이다 (요구 6).
 */
function AutomaticNote({
  row,
  onGenerate,
  onRecheck,
}: {
  row: AiNoteTabLayout['automatic'];
  onGenerate: (recordingId: string, mode: NoteMode) => void;
  onRecheck: () => void;
}) {
  const tab: AiNoteTabView = row.tab;
  const { body } = tab;

  return (
    <section className="note__row">
      <h3 className="note__row-title">{row.heading}</h3>
      <p className="hint">{row.text}</p>
      {/* provider가 준비되지 않은 것은 오류도 설정 요구도 아니다 — 아래 줄이 그대로 있다
          (INV-8 · 요구 6). 그 사실이 상태보다 먼저 읽힌다. */}
      {row.optionalNotice !== null && <p className="hint">{row.optionalNotice}</p>}

      {/* 전사 텍스트가 이 기기를 떠나는가 (§12 · INV-5). audio는 어느 쪽이든 나가지 않는다. */}
      {tab.provider !== null && <p className="hint">{tab.provider.label}</p>}

      {body.kind === 'loading' && <Loading text="Loading AI note…" />}

      {body.kind === 'disabled' && (
        // 빈 상태 셋 — provider가 없다. **경고가 아니다.** AI 기능이 비활성이라는 사실을
        // 담담히 알리며, 아래 줄은 이 상태에서도 그대로 쓸 수 있다 (INV-8 · §13 · MH-1).
        <div className="note__off">
          <EmptyState title={body.notice.headline} body={body.notice.text}>
            <p className="hint">{body.notice.resolution}</p>
            {/* 이 상태가 막지 않는 것 — 재생도 Transcript도 그대로다. */}
            <p className="hint">{body.notice.unaffectedNotice}</p>
            {body.notice.recheck !== null && (
              <button type="button" className="btn btn--secondary" onClick={onRecheck}>
                {body.notice.recheck.label}
              </button>
            )}
          </EmptyState>
        </div>
      )}

      {body.kind === 'noTranscript' && (
        // 재료가 아직 없는 것도 실패가 아니다 (§7.2).
        <EmptyState title={body.text} body={body.hint} />
      )}

      {body.kind === 'none' && (
        // 빈 상태 넷 — 아직 노트가 없다. 여기서 할 수 있는 일 하나가 그 아래에 있다.
        <EmptyState title={body.text}>
          <button
            type="button"
            className="btn btn--primary"
            onClick={() => onGenerate(body.generate.recordingId, body.generate.mode)}
          >
            {body.generate.label}
          </button>
        </EmptyState>
      )}

      {body.kind === 'generating' && (
        <>
          {/* 로컬 모델이 노트 하나를 쓰는 데 걸리는 시간은 초 단위가 아니다 (§16.2) —
              그동안 화면이 얼어붙은 것처럼 보이지 않게 한다 (요구 9). */}
          <Loading text={body.text} live />
          {/* 새 생성이 도는 동안에도 이미 있던 노트는 그대로 보인다 (ADR-0008 §9.2 · INV-2). */}
          {body.kept !== null && <NoteDocument note={body.kept} />}
        </>
      )}

      {body.kind === 'ready' && (
        <>
          <NoteDocument note={body.note} />
          {/* 다시 만들어도 지금 보고 있는 노트는 지워지지 않는다 (요구 12). */}
          <button
            type="button"
            className="btn btn--secondary"
            onClick={() => onGenerate(body.regenerate.recordingId, body.regenerate.mode)}
          >
            {body.regenerate.label}
          </button>
        </>
      )}

      {body.kind === 'failed' && (
        <>
          {/* 무엇이 실패했는지 · 원본은 안전한지 · 다시 시도할 수 있는지 (§13). */}
          {body.failure !== null && (
            <FailureNotice failure={body.failure} headline={body.headline} />
          )}
          {body.failure === null && <p className="failure__headline">{body.headline}</p>}
          {/* 녹음도 Transcript도 기존 노트도 그대로다 (INV-1 · INV-2 · INV-3). */}
          <p className="hint">{body.preservedNotice}</p>
          {/* 모델이 없어서 실패한 경우처럼 먼저 할 일이 다른 갈래가 있다. */}
          {body.resolution !== null && <p className="hint">{body.resolution}</p>}
          <button
            type="button"
            className="btn btn--secondary"
            onClick={() => onGenerate(body.retry.recordingId, body.retry.mode)}
          >
            {body.retry.label}
          </button>
          {body.kept !== null && <NoteDocument note={body.kept} />}
        </>
      )}
    </section>
  );
}

/**
 * 아래 줄 — 이미 쓰고 있는 AI로 가져간다 (`phase-prompt/05.5` 요구 3 · 6 · A-1 · A-2).
 *
 * 세 자리가 나란히 있고, **셋 다 AI provider 없이 동작한다** (MH-1 · MH-2). 어느 자리가 어떤
 * 상태인지 정하는 규칙은 여기 없고 {@link manualHandoff}에 있다 — 이 컴포넌트는 그리기만 한다.
 *
 * **앱이 아무 데도 보내지 않는다** (MH-3). 나가는 행위의 주체는 사람이며, 그 사실이 세 자리
 * 위에 한 줄로 있다.
 */
function ManualHandoff({
  manual,
  onCopy,
  onExportForAi,
  onShowFile,
}: {
  manual: ManualHandoffView;
  onCopy: (action: CopyAction) => void;
  onExportForAi: (action: AiExportAction) => void;
  onShowFile: (action: ShowFileAction) => void;
}) {
  return (
    <section className="note__row">
      <h3 className="note__row-title">{manual.heading}</h3>
      <p className="hint">{manual.text}</p>
      {/* provider가 없어도 된다 · 아무것도 나가지 않는다 (MH-1 · MH-2 · MH-3 · INV-6). */}
      <p className="hint">{manual.noProviderNotice}</p>
      <p className="hint">{manual.localNotice}</p>

      <div className="note__handoff">
        <CopyItem item={manual.copy.prompt} onCopy={onCopy} />
        <CopyItem item={manual.copy.transcript} onCopy={onCopy} />
        <AiExportItem
          item={manual.aiExport}
          onExportForAi={onExportForAi}
          onShowFile={onShowFile}
        />
      </div>
    </section>
  );
}

/**
 * 복사 자리 하나 (요구 A-1 · A-2 · 5).
 *
 * **복사됐다는 것도 실패했다는 것도 문장으로 있다** — 색 하나로만 말하지 않는다 (§7.5 ·
 * 요구 12). 실패했을 때는 다시 시도할 수단과, clipboard를 쓰지 않는 다른 길(Export for AI)이
 * 함께 남는다.
 */
function CopyItem({
  item,
  onCopy,
}: {
  item: CopyItemView;
  onCopy: (action: CopyAction) => void;
}) {
  const { body } = item;

  return (
    <div className="note__handoff-item">
      <p className="note__handoff-title">{item.label}</p>

      {body.kind === 'loading' && <Loading text="Loading…" />}

      {body.kind === 'nothingToCopy' && (
        // 재료가 아직 없는 것은 실패가 아니다 (§7.2 · MH-5).
        <EmptyState title={body.text} body={body.hint} />
      )}

      {body.kind === 'notAsked' && (
        <>
          <p className="hint">{body.text}</p>
          <CopyButton action={body.start} onCopy={onCopy} />
        </>
      )}

      {body.kind === 'copying' && <Loading text={body.text} live />}

      {body.kind === 'copied' && (
        <div role="status">
          {/* 색이 아니라 이 문장이 복사됐다고 말한다 (§7.5 · 요구 12). 다른 완료 표시와
              같은 모양이며, 여기에만 쓰이는 색을 따로 두지 않는다. */}
          <p className="share__headline">{body.headline}</p>
          {/* 몇 번째 중 몇 번째인가 · 얼마나 큰가 (`phase-prompt/05.6` 성공 기준 4). */}
          <p className="hint">{body.portion.label}</p>
          <p className="hint">{body.size.label}</p>
          {body.size.tooLongNotice !== null && <p className="hint">{body.size.tooLongNotice}</p>}
          <p className="hint">{body.text}</p>
          {/* 나머지를 마저 가져가는 수단. 남은 조각이 없으면 이 자리도 없다. */}
          {body.next !== null && <CopyButton action={body.next} onCopy={onCopy} />}
          <CopyButton action={body.again} onCopy={onCopy} />
        </div>
      )}

      {body.kind === 'failed' && (
        <>
          {/* 무엇이 실패했는지 · 원본은 안전한지 · 다시 시도할 수 있는지 (§13). */}
          <FailureNotice failure={body.failure} headline={body.headline} />
          {/* 복사는 읽기만 한다 — 아무것도 달라지지 않았다 (INV-3 · MH-7). */}
          <p className="hint">{body.preservedNotice}</p>
          {body.resolution !== null && <p className="hint">{body.resolution}</p>}
          <CopyButton action={body.retry} onCopy={onCopy} />
          {/* clipboard가 막혀도 남는 길 (§7.5). 실패에는 언제나 있다. */}
          <p className="hint">
            {body.alternative.label} — {body.alternative.text}
          </p>
        </>
      )}
    </div>
  );
}

function CopyButton({
  action,
  onCopy,
}: {
  action: CopyAction;
  onCopy: (action: CopyAction) => void;
}) {
  return (
    <button type="button" className="btn btn--secondary" onClick={() => onCopy(action)}>
      {action.label}
    </button>
  );
}

/**
 * Export for AI 자리 (요구 3).
 *
 * **clipboard를 전혀 쓰지 않는다** — 복사가 거절되는 환경에서도 이 길은 그대로 남는다
 * (ADR-0010 §7.5). 만들어진 파일의 **전체 경로는 그대로 보이고**, 이제 그 자리를 여는 수단이
 * 그 옆에 하나 더 있다 (`phase-prompt/05.6` 성공 기준 2 · R-4) — 파일을 AI 채팅에 첨부하려면
 * 그 파일에 실제로 도달할 수 있어야 하기 때문이다. 대체가 아니라 추가다.
 */
function AiExportItem({
  item,
  onExportForAi,
  onShowFile,
}: {
  item: AiExportItemView;
  onExportForAi: (action: AiExportAction) => void;
  onShowFile: (action: ShowFileAction) => void;
}) {
  const { body } = item;

  return (
    <div className="note__handoff-item">
      <p className="note__handoff-title">{item.label}</p>

      {body.kind === 'loading' && <Loading text="Loading…" />}

      {body.kind === 'nothingToExport' && <EmptyState title={body.text} body={body.hint} />}

      {body.kind === 'notAsked' && (
        <>
          <p className="hint">{body.text}</p>
          <AiExportButton action={body.start} onExportForAi={onExportForAi} />
        </>
      )}

      {body.kind === 'exporting' && <Loading text={body.text} live />}

      {body.kind === 'done' && (
        <div className="share__done" role="status">
          <p className="share__headline">{body.headline}</p>
          {/* 이 파일이 문서의 몇 번째인가 · 문서 전체가 얼마나 큰가 (성공 기준 4). */}
          <p className="hint">{body.portion.label}</p>
          <p className="hint">{body.size.label}</p>
          {body.size.tooLongNotice !== null && <p className="hint">{body.size.tooLongNotice}</p>}
          <p className="share__file">{body.fileName}</p>
          {/* 어디에 만들어졌는가 (§4.1). **이 줄은 그대로 남는다** — 여는 수단은 그 대체가
              아니라 추가다 (성공 기준 2). */}
          <p className="share__path">{body.path}</p>
          {/* 그 자리를 연다 (R-4). 첨부하려면 파일에 실제로 도달할 수 있어야 한다. */}
          <ShowFileControl show={body.show} onShowFile={onShowFile} />
          <p className="hint">{body.text}</p>
          {/* 나머지를 마저 파일로 꺼내는 수단. 있던 파일은 그대로다 (ADR-0009 §4.3). */}
          {body.next !== null && (
            <AiExportButton action={body.next} onExportForAi={onExportForAi} />
          )}
          <AiExportButton action={body.again} onExportForAi={onExportForAi} />
        </div>
      )}

      {body.kind === 'failed' && (
        <>
          {/* 무엇이 실패했는지 · 원본은 안전한지 · 다시 시도할 수 있는지 (§13). */}
          <FailureNotice failure={body.failure} headline={body.headline} />
          {/* 앱은 아무것도 지우지 않았다 (INV-3 · MH-7). */}
          <p className="hint">{body.preservedNotice}</p>
          {body.resolution !== null && <p className="hint">{body.resolution}</p>}
          <AiExportButton action={body.retry} onExportForAi={onExportForAi} />
        </>
      )}
    </div>
  );
}

function AiExportButton({
  action,
  onExportForAi,
}: {
  action: AiExportAction;
  onExportForAi: (action: AiExportAction) => void;
}) {
  return (
    <button type="button" className="btn btn--secondary" onClick={() => onExportForAi(action)}>
      {action.label}
    </button>
  );
}

/**
 * 노트 하나를 **구조 그대로** 그린다 (§9.3 · INV-9).
 *
 * 여기에 Markdown도, provider가 준 문자열도 없다. 그리는 것은 {@link NoteSection}의 목록이며
 * 그 목록을 만드는 규칙은 `aiNoteView.ts`에 있다 — 이 컴포넌트는 문단과 항목을 놓기만 한다.
 * 같은 구조를 Phase 5의 Markdown · Notion renderer가 다시 소비한다.
 */
function NoteDocument({ note }: { note: NoteView }) {
  return (
    <article className="note__document">
      {note.sections.map((section) => (
        <NoteSectionBlock key={section.title} section={section} />
      ))}
      {/* 어떤 provider · 모델 · promptVersion으로, 언제, 어떤 Transcript version에서 (§7.3). */}
      <p className="note__provenance">
        {note.modeLabel} · {note.provenance.label}
      </p>
    </article>
  );
}

/** 섹션 하나. 문단과 항목은 다른 모양으로 놓인다 (§9.5). */
function NoteSectionBlock({ section }: { section: NoteSection }) {
  return (
    <section className="note__section">
      <h3 className="note__section-title">{section.title}</h3>
      {section.kind === 'text' && section.text !== null && (
        <p className="note__text">{section.text}</p>
      )}
      {section.kind === 'list' && section.items.length > 0 && (
        <ul className="note__items">
          {section.items.map((item, index) => (
            <li key={`${section.title}-${index}`} className="note__item">
              {item}
            </li>
          ))}
        </ul>
      )}
      {/* 비어 있는 것은 실제 결과다. 오류로 그리지 않는다 (ADR-0008 §7.3). */}
      {section.emptyText !== null && <p className="hint">{section.emptyText}</p>}
    </section>
  );
}

/** segment 목록. 시작·종료 timestamp가 문장과 함께 보인다 (요구 6). */
/**
 * 전사를 **읽을 수 있는 형태로** 그린다 (2026-09-08).
 *
 * 2시간 회의가 segment 2,825개로 나왔고, 그것을 한 줄씩 쌓으면 로그이지 글이 아니다.
 * 묶는 규칙은 여기 없다 — `transcriptParagraphs`가 값으로 판정하고, 이 컴포넌트는 그린다.
 *
 * 찾는 칸은 문단이 여러 개일 때만 나온다. 미리보기처럼 짧은 자리에 검색을 두면 소음이다.
 */
function TranscriptLines({ lines }: { lines: readonly TranscriptLine[] }) {
  const [query, setQuery] = useState('');

  const paragraphs = useMemo(() => transcriptParagraphs(lines), [lines]);
  const shown = useMemo(() => matchingParagraphs(paragraphs, query), [paragraphs, query]);
  const notice = searchNotice(query, shown.length, paragraphs.length);

  if (paragraphs.length === 0) {
    return null;
  }

  return (
    <>
      {paragraphs.length > 1 && (
        <label className="field transcript__search" htmlFor="transcript-search">
          <span className="field__label">전사에서 찾기</span>
          <input
            id="transcript-search"
            type="search"
            className="input"
            placeholder="낱말을 입력한다"
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
          />
        </label>
      )}
      {/* 없으면 없다고 말한다 — 빈 화면만 남으면 전사가 사라진 줄 안다. */}
      {notice !== null && <p className="hint">{notice}</p>}

      <ol className="transcript__lines">
        {shown.map((paragraph) => (
          <li key={`${paragraph.startMs}-${paragraph.text.slice(0, 24)}`} className="transcript__line">
            <span className="transcript__time">{paragraph.startLabel}</span>
            <span className="transcript__text">{paragraph.text}</span>
          </li>
        ))}
      </ol>
    </>
  );
}
