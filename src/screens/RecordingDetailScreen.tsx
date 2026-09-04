import { useCallback, useEffect, useRef, useState } from 'react';
import {
  aiNoteStatus,
  aiProviderStatus,
  getRecording,
  getTranscript,
  listAiNotes,
  listMissingAudio,
  recordingAudioSource,
  startAiNote,
  startTranscription,
  transcriptionStatus,
} from '../ipc/commands';
import type {
  AiNote,
  AiNoteStatus,
  AiProviderStatus,
  NoteMode,
  Recording,
  Transcript,
  TranscriptionStatus,
} from '../ipc/types';
import {
  aiNoteTab,
  aiNoteTrouble,
  type AiNoteTabView,
  type AiNoteTrouble,
  type NoteSection,
  type NoteView,
} from './aiNoteView';
import { FailureNotice } from './FailureNotice';
import {
  LOADING_RECORDING_DETAIL,
  MISSING_AUDIO_NOTICE,
  failedRecordingDetail,
  loadedRecordingDetail,
  type RecordingDetailView,
} from './recordingDetailView';
import {
  LOADING_TRANSCRIPT_TAB,
  transcriptTab,
  transcriptTrouble,
  type TranscriptLine,
  type TranscriptTabView,
  type TranscriptTrouble,
} from './transcriptView';
import type { ScreenProps } from './types';

// §5.C의 세 탭. AI Note가 없어도 화면은 정상 동작해야 한다 (INV-8).
const TABS = ['AI Note', 'Transcript', 'Recording'] as const;
type Tab = (typeof TABS)[number];

/** 레코드는 있는데 파일이 없을 때 Recording 탭이 말하는 것. */
const NO_AUDIO_TEXT = 'No audio file yet.';

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

  const recordingId = route.screen === 'recording-detail' ? route.recordingId : null;

  /** 마지막으로 본 전사 상태. 전사가 **끝나는 순간**을 알아보는 데 쓴다. */
  const lastLiveState = useRef<TranscriptionStatus['state'] | null>(null);
  /** 마지막으로 본 노트 생성 상태. 생성이 **끝나는 순간**을 알아보는 데 쓴다. */
  const lastAiState = useRef<AiNoteStatus['state'] | null>(null);

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

  useEffect(() => {
    // 노트를 만드는 동안에만 되풀이해 물어본다. 상태 조회는 생성을 기다리지 않으므로
    // (`NoteGenerator::status`) 이 되풀이가 화면을 멎게 하지 않는다.
    if (!generatingNote) {
      return;
    }
    const timer = setInterval(refreshAiNote, AI_NOTE_REFRESH_MS);
    return () => clearInterval(timer);
  }, [generatingNote, refreshAiNote]);

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

  /** provider 상태를 다시 물어본다. **실패의 재시도가 아니라 확인이다** (INV-8). */
  const recheckProvider = () => {
    setAiTrouble(null);
    setProviderAttempt((count) => count + 1);
  };

  // 대상 recording 없이 이 화면에 도달하는 것도 정상 상태다.
  if (recordingId === null) {
    return (
      <div className="screen">
        <p className="empty">No recording selected.</p>
        <button type="button" className="action" onClick={goBack}>
          Back
        </button>
      </div>
    );
  }

  if (view.kind === 'loading') {
    return (
      <div className="screen">
        <p className="hint">Loading recording…</p>
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
        <button type="button" className="action" onClick={goBack}>
          Back
        </button>
      </div>
    );
  }

  if (view.kind === 'notFound') {
    return (
      <div className="screen">
        <p className="empty">This recording is no longer in the list.</p>
        <p className="hint">{view.recordingId}</p>
        <button type="button" className="action" onClick={goBack}>
          Back
        </button>
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
          <audio className="detail__audio" controls preload="metadata" src={view.audioSource} />
        </div>
      ) : (
        // 파일이 없다는 사실을 보여줄 뿐 아무것도 지우지 않는다 (INV-3 · INV-4).
        <div className="detail__player detail__player--missing" role="status">
          <p className="detail__missing">Audio file not found</p>
          <p className="hint">{MISSING_AUDIO_NOTICE}</p>
          <p className="detail__path">{view.audioPath}</p>
        </div>
      )}

      <div className="tabs" role="tablist">
        {TABS.map((name) => (
          <button
            key={name}
            type="button"
            role="tab"
            aria-selected={tab === name}
            className={tab === name ? 'tabs__tab tabs__tab--active' : 'tabs__tab'}
            onClick={() => setTab(name)}
          >
            {name}
          </button>
        ))}
      </div>

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
          tab={aiView}
          trouble={aiTrouble}
          onMode={setNoteMode}
          onGenerate={beginAiNote}
          onRecheck={recheckProvider}
        />
      )}
    </div>
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

      {tab.kind === 'loading' && <p className="hint">Loading transcript…</p>}

      {tab.kind === 'none' && (
        <>
          <p className="empty">{tab.text}</p>
          <button
            type="button"
            className="action"
            onClick={() => onTranscribe(tab.start.recordingId)}
          >
            {tab.start.label}
          </button>
        </>
      )}

      {(tab.kind === 'pending' || tab.kind === 'running') && (
        <>
          {/* 상태가 바뀌는 것은 소리로도 알린다. 화면은 이 동안에도 멎지 않는다. */}
          <p className="hint" aria-live="polite">
            {tab.text}
          </p>
          {/* 새 전사가 도는 동안에도 이미 있던 Transcript는 그대로 보인다 (§7.1 · INV-2). */}
          <TranscriptLines lines={tab.kept} />
        </>
      )}

      {tab.kind === 'done' && (
        <>
          <p className="hint">
            {[tab.language, tab.engine, tab.model].filter((fact) => fact !== null).join(' · ')}
          </p>
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
          {/* 모델이 없어서 실패한 경우는 먼저 할 일이 다르다. */}
          {tab.resolution !== null && <p className="hint">{tab.resolution}</p>}
          <button
            type="button"
            className="action"
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
 * AI Note 탭 (§5 C · §9 · `phase-prompt/04` 요구 4 · 12 · 14 · 15).
 *
 * 일곱 상태가 서로 다른 모습을 갖는다. 어느 상태인지 정하는 규칙은 여기 없고
 * {@link aiNoteTab}에 있다 — 이 컴포넌트는 그리기만 한다.
 *
 * **AI가 꺼져 있는 상태를 {@link FailureNotice}로 그리지 않는다** (INV-8 · §13). 그 자리에는
 * `role="alert"`도 없고 `Failure`도 없다 — 담담한 사실 몇 줄과, 다시 확인해 볼 수단뿐이다.
 */
function AiNoteTab({
  tab,
  trouble,
  onMode,
  onGenerate,
  onRecheck,
}: {
  tab: AiNoteTabView;
  trouble: AiNoteTrouble | null;
  onMode: (mode: NoteMode) => void;
  onGenerate: (recordingId: string, mode: NoteMode) => void;
  onRecheck: () => void;
}) {
  const { body } = tab;

  return (
    <section className="note">
      {/* 요청이 거절된 사실은 노트 상태를 덮지 않고 그 옆에 남는다 (§13). */}
      {trouble !== null && <FailureNotice failure={trouble.failure} headline={trouble.headline} />}

      {/* 무엇을 만들 것인가 (§9.5). 지금 바꿀 수 있는지는 화면이 아니라 값이 말한다. */}
      <div className="note__modes" role="group" aria-label="Note mode">
        {tab.modes.map((choice) => (
          <button
            key={choice.mode}
            type="button"
            className={choice.selected ? 'note__mode note__mode--active' : 'note__mode'}
            aria-pressed={choice.selected}
            disabled={!tab.modeSelectable}
            title={choice.sections}
            onClick={() => onMode(choice.mode)}
          >
            {choice.label}
          </button>
        ))}
      </div>

      {/* 전사 텍스트가 이 기기를 떠나는가 (§12 · INV-5). audio는 어느 쪽이든 나가지 않는다. */}
      {tab.provider !== null && <p className="hint">{tab.provider.label}</p>}

      {body.kind === 'loading' && <p className="hint">Loading AI note…</p>}

      {body.kind === 'disabled' && (
        // 경고가 아니다. AI 기능이 비활성이라는 사실을 담담히 알린다 (INV-8 · §13).
        <div className="note__off" role="status">
          <p className="empty">{body.notice.headline}</p>
          <p className="hint">{body.notice.text}</p>
          <p className="hint">{body.notice.resolution}</p>
          {/* 이 상태가 막지 않는 것 — 재생도 Transcript도 그대로다. */}
          <p className="hint">{body.notice.unaffectedNotice}</p>
          {body.notice.recheck !== null && (
            <button type="button" className="action" onClick={onRecheck}>
              {body.notice.recheck.label}
            </button>
          )}
        </div>
      )}

      {body.kind === 'noTranscript' && (
        // 재료가 아직 없는 것도 실패가 아니다 (§7.2).
        <div role="status">
          <p className="empty">{body.text}</p>
          <p className="hint">{body.hint}</p>
        </div>
      )}

      {body.kind === 'none' && (
        <>
          <p className="empty">{body.text}</p>
          <button
            type="button"
            className="action"
            onClick={() => onGenerate(body.generate.recordingId, body.generate.mode)}
          >
            {body.generate.label}
          </button>
        </>
      )}

      {body.kind === 'generating' && (
        <>
          <p className="hint" aria-live="polite">
            {body.text}
          </p>
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
            className="action"
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
            className="action"
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
function TranscriptLines({ lines }: { lines: readonly TranscriptLine[] }) {
  if (lines.length === 0) {
    return null;
  }

  return (
    <ol className="transcript__lines">
      {lines.map((line) => (
        <li key={`${line.startLabel}-${line.endLabel}-${line.text}`} className="transcript__line">
          <span className="transcript__time">{line.rangeLabel}</span>
          <span className="transcript__text">{line.text}</span>
        </li>
      ))}
    </ol>
  );
}
