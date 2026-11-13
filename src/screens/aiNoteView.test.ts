// AI Note 탭 상태 변환 테스트.
//
// 일곱 경로를 전부 본다 — **아직 읽지 못함 · AI 비활성 · 전사 없음 · 노트 없음 · 생성 중 ·
// 완료 · 실패**. Ollama도 모델도 네트워크도 DOM도 필요하지 않다 (PRODUCT-SPEC §18 ·
// phase-prompt/04 요구 16).
//
// 이 테스트가 지키는 것이 셋 더 있다:
//   1. 렌더링이 `Structured Note → UI` 한 방향으로만 흐른다 (§9.3). 섹션의 이름과 순서는
//      §9.5의 표에서 오며, provider가 준 Markdown 문자열이 화면 값에 들어올 자리가 없다.
//   2. **provider 미설정·미연결이 실패로 그려지지 않는다** (INV-8 · §13) — 그 상태의 값에는
//      `failure`가 아예 없고, 같은 상황에서 Transcript 탭과 재생 경로는 그대로 동작한다.
//   3. 실패가 아무것도 잃지 않는다 (INV-1 · INV-2 · INV-3) — 이미 있던 노트는 실패 상태에서도
//      그대로 남고, 이 모듈에는 그것을 지우는 함수 자체가 없다.
import { describe, expect, it } from 'vitest';
import type { Failure, FailureKind } from '../ipc/failure';
import type {
  ActionItemV2,
  AiNote,
  AiNoteStatus,
  AiProviderStatus,
  MeetingNote,
  MeetingNoteV2,
  PreviousActionItems,
  Recording,
  StudyNote,
  SummaryNote,
  Topic,
  Transcript,
} from '../ipc/types';
import {
  AI_DISABLED_HEADLINE,
  AI_NOTE_FAILED_HEADLINE,
  AI_NOTE_PRESERVED_NOTICE,
  AI_UNAFFECTED_NOTICE,
  EMPTY_SECTION_TEXT,
  NOTE_MODES,
  NO_AI_NOTE_TEXT,
  NO_TRANSCRIPT_INPUT_TEXT,
  UNKNOWN_AI_NOTE_NOTICE,
  aiNoteTab,
  aiNoteTrouble,
  latestNote,
  noteModeChoices,
  noteSections,
  noteView,
  type AiNoteInput,
} from './aiNoteView';
import { loadedRecordingDetail } from './recordingDetailView';
import { transcriptTab } from './transcriptView';

/** 저장소가 돌려주는 모양 그대로의 레코드. */
function recording(overrides: Partial<Recording> = {}): Recording {
  return {
    id: 'r-1',
    title: '3DGS Study #04',
    createdAt: '2026-09-01T09:12:00.000Z',
    updatedAt: '2026-09-01T09:12:00.000Z',
    durationMs: 3_151_000,
    durationLabel: '52:31',
    audioPath: '/recordings/r-1.wav',
    audioFormat: 'wav',
    microphone: 'MacBook Microphone',
    currentTranscriptId: 't-1',
    transcriptionStatus: 'done',
    aiStatus: 'none',
    notionStatus: 'none',
    ...overrides,
  };
}

function transcript(overrides: Partial<Transcript> = {}): Transcript {
  return {
    id: 't-1',
    recordingId: 'r-1',
    language: 'ko',
    segments: [{ startMs: 134_000, endMs: 141_000, text: '그러면 이번에는 PLY 먼저 변환하고' }],
    rawText: '그러면 이번에는 PLY 먼저 변환하고',
    createdAt: '2026-09-03T04:50:26.000Z',
    engine: 'whisper.cpp',
    model: 'ggml-base.bin',
    transcriptionMs: null,
    transcriptionLabel: null,
    ...overrides,
  };
}

/** 고른 provider가 쓸 수 있는 상태. 벤더 이름은 provider 자신이 말한 값이다 (INV-9). */
function provider(overrides: Partial<AiProviderStatus> = {}): AiProviderStatus {
  return {
    state: 'ready',
    providerId: 'ollama',
    providerName: 'Ollama',
    locality: 'local',
    models: ['llama3.1:8b'],
    failure: null,
    ...overrides,
  };
}

/** provider를 고르지 않은 상태. **기본이자 정상 상태다** (INV-8). */
function noProvider(): AiProviderStatus {
  return {
    state: 'notConfigured',
    providerId: null,
    providerName: null,
    locality: null,
    models: [],
    failure: null,
  };
}

function liveStatus(overrides: Partial<AiNoteStatus> = {}): AiNoteStatus {
  return { state: 'idle', recordingId: null, mode: null, aiNoteId: null, failure: null, ...overrides };
}

function failure(kind: FailureKind, overrides: Partial<Failure> = {}): Failure {
  return {
    kind,
    message: 'AI provider에 닿지 못했다.',
    detail: null,
    sourceDataSafe: true,
    retryable: true,
    ...overrides,
  };
}

const MEETING: MeetingNote = {
  mode: 'meeting',
  schemaVersion: 1,
  overview: '이번 주 변환 작업 범위를 정했다.',
  keyDiscussions: ['PLY와 SOG 변환 순서'],
  decisions: ['PLY를 먼저 변환한다'],
  actionItems: ['SOG 변환 결과 확인'],
  openQuestions: [],
};

const PREV_ACTIONS: PreviousActionItems = {
  done: ['CI 파이프라인 구축'],
  changed: [],
  inProgress: ['API 문서화'],
  noProgress: [],
  notMentioned: [],
};

const EMPTY_PREV_ACTIONS: PreviousActionItems = {
  done: [],
  changed: [],
  inProgress: [],
  noProgress: [],
  notMentioned: [],
};

const TOPICS: readonly Topic[] = [
  {
    title: '배포 전략',
    groups: [
      { heading: '현재 상태', items: ['스테이징 환경 구축 완료'] },
      { heading: '결정 사항', items: ['블루-그린 배포 적용'] },
    ],
  },
  {
    title: '코드 리뷰 프로세스',
    groups: [
      { heading: '문제점', items: ['리뷰 대기 시간이 길다'] },
    ],
  },
];

const ACTION_ITEMS_V2: readonly ActionItemV2[] = [
  { task: '배포 스크립트 작성', detail: '', owner: '김철수', due: '11/20', period: '' },
  { task: '리뷰 가이드 초안', detail: '', owner: '이영희', due: '11/22', period: '' },
];

const MEETING_V2: MeetingNoteV2 = {
  mode: 'meeting',
  schemaVersion: 2,
  meetingTitle: '주간 기술 회의',
  meetingKind: '정기 회의',
  attendees: ['김철수', '이영희'],
  reliabilityNotice: 'AI 요약이며 원문 확인이 필요하다.',
  keySummary: ['배포 전략을 블루-그린으로 결정', '코드 리뷰 대기 시간 개선 논의'],
  previousActionItems: PREV_ACTIONS,
  topics: TOPICS,
  actionItems: ACTION_ITEMS_V2,
  needsConfirmation: ['배포 일정 최종 확인'],
};

const STUDY: StudyNote = {
  mode: 'study',
  schemaVersion: 2,
  overview: '3D Gaussian Splatting의 표현 방식을 정리했다.',
  keyConcepts: ['Gaussian primitive'],
  importantDetails: ['SH 계수는 색을 방향에 따라 표현한다'],
  questions: ['압축률과 화질의 균형은?'],
  thingsToStudy: ['SOG 포맷 명세'],
  referencesMentioned: [],
};

const SUMMARY: SummaryNote = {
  mode: 'summary',
  schemaVersion: 2,
  shortSummary: '변환 파이프라인 순서를 정하고 확인 항목을 나눴다.',
  keyPoints: ['PLY 먼저', 'SOG 확인'],
};

/** 저장된 노트 하나. provenance 다섯 값이 전부 있다 (§7.3). */
function aiNote(overrides: Partial<AiNote> = {}): AiNote {
  return {
    id: 'n-1',
    recordingId: 'r-1',
    transcriptId: 't-1',
    mode: 'meeting',
    note: MEETING,
    provider: 'ollama',
    model: 'llama3.1:8b',
    promptVersion: 'meeting/2026-09-03',
    generatedAt: '2026-09-03T05:10:00.000Z',
    ...overrides,
  };
}

/** 기본 입력 — provider가 준비됐고 전사가 있고 노트는 아직 없다. */
function input(overrides: Partial<AiNoteInput> = {}): AiNoteInput {
  return {
    recording: recording(),
    transcript: transcript(),
    notes: [],
    provider: provider(),
    live: null,
    mode: 'meeting',
    ...overrides,
  };
}

describe('mode 선택 (§9.5)', () => {
  it('세 mode가 표시 순서대로 있다', () => {
    expect(NOTE_MODES).toEqual(['meeting', 'study', 'summary']);
    expect(noteModeChoices('meeting').map((choice) => choice.label)).toEqual([
      '회의록',
      '학습 노트',
      '요약',
    ]);
  });

  it('고른 것 하나만 selected다', () => {
    const chosen = noteModeChoices('study');
    expect(chosen.filter((choice) => choice.selected).map((choice) => choice.mode)).toEqual([
      'study',
    ]);
  });

  it('고르기 전에 무엇이 나오는지 알 수 있다 (§9.5의 섹션)', () => {
    const [meeting, , summary] = noteModeChoices('meeting');
    expect(meeting.sections).toBe(
      '핵심 요약 · 지난 액션 아이템 · 주제별 섹션 · 액션 아이템 · 확인 필요',
    );
    expect(summary.sections).toBe('짧은 요약 · 핵심');
  });

  it('세 mode를 골라 생성할 수 있다 — 고른 mode가 그대로 동작에 실린다', () => {
    for (const mode of NOTE_MODES) {
      const view = aiNoteTab(input({ mode }));
      expect(view.modeSelectable).toBe(true);
      expect(view.body.kind).toBe('none');
      if (view.body.kind !== 'none') {
        throw new Error('생성할 수 있는 상태여야 한다');
      }
      expect(view.body.generate.mode).toBe(mode);
      expect(view.body.generate.recordingId).toBe('r-1');
    }
  });
});

describe('Structured Note → UI (§9.3 · §9.5)', () => {
  it('Meeting은 §9.5의 다섯 섹션을 그 순서로 만든다', () => {
    expect(noteSections(MEETING).map((section) => section.title)).toEqual([
      '개요',
      '논의한 것',
      '정해진 것',
      '할 일',
      '남은 질문',
    ]);
  });

  it('Study는 여섯 섹션, Summary는 두 섹션이다', () => {
    expect(noteSections(STUDY).map((section) => section.title)).toEqual([
      '개요',
      '핵심 개념',
      '중요한 세부',
      '질문',
      '더 공부할 것',
      '언급된 자료',
    ]);
    expect(noteSections(SUMMARY).map((section) => section.title)).toEqual([
      '짧은 요약',
      '핵심',
    ]);
  });

  it('문단과 항목이 다른 종류로 남는다 — 화면이 문단을 목록처럼 그리지 않게', () => {
    const [overview, discussions] = noteSections(MEETING);
    expect(overview.kind).toBe('text');
    expect(overview.text).toBe('이번 주 변환 작업 범위를 정했다.');
    expect(overview.items).toEqual([]);

    expect(discussions.kind).toBe('list');
    expect(discussions.text).toBeNull();
    expect(discussions.items).toEqual(['PLY와 SOG 변환 순서']);
  });

  it('빈 섹션은 실패가 아니라 비었다고 말한다 (ADR-0008 §7.3)', () => {
    const openQuestions = noteSections(MEETING)[4];
    expect(openQuestions.items).toEqual([]);
    expect(openQuestions.emptyText).toBe(EMPTY_SECTION_TEXT);
    // 비었다는 사실이지 실패가 아니다.
    expect(openQuestions).not.toHaveProperty('failure');
  });

  it('내용이 있는 섹션에는 emptyText가 없다', () => {
    expect(noteSections(SUMMARY)[1].emptyText).toBeNull();
  });

  it('provider가 준 문자열 한 덩어리가 화면 값에 남지 않는다', () => {
    // 화면이 받는 것은 섹션의 목록이며, Markdown 본문 같은 단일 문자열 필드가 없다.
    const view = noteView(aiNote());
    expect(view.sections).toHaveLength(5);
    expect(view).not.toHaveProperty('content');
    expect(view).not.toHaveProperty('markdown');
    expect(view).not.toHaveProperty('rawText');
  });
});

describe('V2 Meeting → noteSections (AC-4-1 · AC-4-2 · AC-4-3)', () => {
  it('V2 fixture가 기준 형식의 섹션 순서를 만든다 (AC-4-1)', () => {
    const sections = noteSections(MEETING_V2);
    expect(sections.map((s) => s.title)).toEqual([
      '핵심 요약',
      '지난 액션 아이템',
      '배포 전략',
      '코드 리뷰 프로세스',
      '액션 아이템',
      '확인 필요',
    ]);
  });

  it('핵심 요약은 list 섹션이다', () => {
    const sections = noteSections(MEETING_V2);
    expect(sections[0].kind).toBe('list');
    expect(sections[0].items).toEqual(MEETING_V2.keySummary);
  });

  it('지난 액션 아이템은 grouped 섹션이다 — 비지 않은 분류만 있다', () => {
    const sections = noteSections(MEETING_V2);
    const prev = sections[1];
    expect(prev.kind).toBe('grouped');
    expect(prev.groups.map((g) => g.heading)).toEqual(['완료', '진행 중']);
    expect(prev.groups[0].items).toEqual(['CI 파이프라인 구축']);
    expect(prev.groups[1].items).toEqual(['API 문서화']);
  });

  it('주제별 섹션은 grouped이고 groups에 소제목·항목이 있다', () => {
    const sections = noteSections(MEETING_V2);
    const deployTopic = sections[2];
    expect(deployTopic.kind).toBe('grouped');
    expect(deployTopic.title).toBe('배포 전략');
    expect(deployTopic.groups).toHaveLength(2);
    expect(deployTopic.groups[0].heading).toBe('현재 상태');
    expect(deployTopic.groups[0].items).toEqual(['스테이징 환경 구축 완료']);
  });

  it('액션 아이템에 담당과 기한이 들어간다', () => {
    const sections = noteSections(MEETING_V2);
    const ai = sections.find((s) => s.title === '액션 아이템')!;
    expect(ai.kind).toBe('list');
    expect(ai.items[0]).toContain('김철수');
    expect(ai.items[0]).toContain('11/20');
    expect(ai.items[1]).toContain('이영희');
  });

  it('V1 Meeting fixture가 기존과 동일한 5섹션을 만든다 — 회귀 없음 (AC-4-2)', () => {
    const sections = noteSections(MEETING);
    expect(sections.map((s) => s.title)).toEqual([
      '개요',
      '논의한 것',
      '정해진 것',
      '할 일',
      '남은 질문',
    ]);
    expect(sections[0].kind).toBe('text');
    expect(sections[0].text).toBe('이번 주 변환 작업 범위를 정했다.');
  });

  it('previousActionItems가 전부 비면 해당 섹션이 결과에 없다 (AC-4-3)', () => {
    const v2Empty: MeetingNoteV2 = { ...MEETING_V2, previousActionItems: EMPTY_PREV_ACTIONS };
    const sections = noteSections(v2Empty);
    const titles = sections.map((s) => s.title);
    expect(titles).not.toContain('지난 액션 아이템');
    // 나머지 순서는 유지된다.
    expect(titles[0]).toBe('핵심 요약');
    expect(titles[1]).toBe('배포 전략');
  });
});

// ── AC-8-1: ~25KB V2 fixture의 앱 noteSections 통과 (E2E 검증의 TS 구간) ──────

describe('large V2 Meeting fixture → noteSections (AC-8-1 · AC-8-2)', () => {
  // 2시간 분량 회의를 모사하는 큰 fixture — Rust 쪽 meeting_v2_e2e.rs와 같은 구조.
  const LARGE_PREV: PreviousActionItems = {
    done: ['RFP 초안 완성 — 마케팅팀 검토 대기 중 (김철수)', '스테이징 환경 API 배포 완료 (박지민)', 'QA 테스트 시나리오 문서 v1 작성 완료 (정하늘)'],
    changed: ['일정: 디자인 시스템 전환 11월 → 12월로 연기', '범위: 모바일 앱 리뉴얼을 v2에서 빼고 v3로 이동'],
    inProgress: ['프론트엔드 성능 최적화 작업 중', '사용자 인터뷰 진행 중 — 12/20명 완료', 'CI/CD 파이프라인 개선 50% 진행'],
    noProgress: ['서버 이관 사전 조사 — 담당자 부재', '접근성 감사 — 외부 업체 계약 지연'],
    notMentioned: ['테스트 환경 분리 구축 계획', '기술 블로그 작성'],
  };

  const LARGE_TOPICS: readonly Topic[] = [
    { title: 'Q4 제품 로드맵 점검', groups: [
      { heading: '일정 확정', items: ['MVP 출시 9월 30일 확정', '베타 테스트 10월 15일~'] },
      { heading: '리소스 배분', items: ['프론트엔드 성능 최적화 우선', '외주 디자이너 10월 투입'] },
    ]},
    { title: '디자인 시스템 v2 전환', groups: [
      { heading: '', items: ['디자인 토큰 표준화 2주 추가 필요', 'Storybook 문서화 60% 완료'] },
      { heading: '마이그레이션', items: ['화면별 전환 진행', '공존 기간 최대 3주', '가이드 다음 주 공유'] },
    ]},
    { title: '백엔드 API 리팩토링', groups: [
      { heading: '성과', items: ['REST→GraphQL 전환 완료', '응답 시간 30% 개선'] },
      { heading: '남은 작업', items: ['레거시 REST 11월 말 폐기', 'subscription 11월 구현'] },
      { heading: '우려', items: ['N+1 쿼리 문제 발견 — DataLoader 필요'] },
    ]},
    { title: 'QA 자동화 전략', groups: [
      { heading: '현황', items: ['커버리지 72%, E2E 15개만 자동화', 'Playwright 도입 검토 중'] },
      { heading: '목표', items: ['12월 말 90% 달성', '회귀 테스트 45분→20분'] },
    ]},
    { title: 'VOC 분석', groups: [
      { heading: '분석 결과', items: ['153건: 기능 요청 67, 버그 42, UX 31, 기타 13'] },
      { heading: '우선순위', items: ['상위 20건 선정', '심각도1 버그 3건 핫픽스'] },
    ]},
    { title: '인프라 및 DevOps', groups: [
      { heading: '클라우드 이관', items: ['AWS→GCP 이관 Q1으로 연기'] },
      { heading: 'CI/CD', items: ['빌드 12분→8분 캐싱 전략', '환경 설정 IaC 계획'] },
    ]},
  ];

  const LARGE_ACTIONS: readonly ActionItemV2[] = Array.from({ length: 16 }, (_, i) => ({
    task: `액션 아이템 ${i + 1}: 구체적 할 일`,
    detail: `상세 설명 ${i + 1}`,
    owner: ['김철수', '이영희', '박지민', '최민수', '정하늘'][i % 5],
    due: `11/${13 + (i % 15)}`,
    period: i < 8 ? '이번 주' : '다음 주',
  }));

  const LARGE_V2: MeetingNoteV2 = {
    mode: 'meeting',
    schemaVersion: 2,
    meetingTitle: '2026년 4분기 사업 전략 및 로드맵 정기 검토 회의',
    meetingKind: '정기 위클리 (사업 계획 확정 및 일정 조율)',
    attendees: ['김철수 (PM)', '이영희 (디자인 리드)', '박지민 (백엔드)', '최민수 (프론트엔드)', '정하늘 (QA)'],
    reliabilityNotice: '1시간 53분 분량의 녹음을 자동 전사(STT) 후 AI가 정리한 회의록입니다.',
    keySummary: [
      '9월 출시 확정 — MVP를 9월 말까지',
      '디자인 시스템 v2 전환 — 11월까지 전면 적용',
      '백엔드 API 리팩토링 완료 — 응답 30% 개선',
      'QA 자동화 커버리지 확대 — 12월 말 90%',
      '고객 피드백 상위 20건 11월 스프린트 반영',
      '인프라 이관 내년 Q1로 연기',
      '시니어 백엔드 채용 12월까지',
    ],
    previousActionItems: LARGE_PREV,
    topics: LARGE_TOPICS,
    actionItems: LARGE_ACTIONS,
    needsConfirmation: [
      '프로젝트 알파 명칭 — 프로젝트 앱타일 수도 있음',
      '5,000만원 정확한지 — 5천만원과 5백만원 구분 불명확',
      '이영희 리드 — 영희 씨와 이 리드가 같은 사람인지 추정',
      '베타 테스트 10월 15일 — 10월 5일로도 들림',
      'Playwright — 플레이라이트 등 여러 형태로 전사됨',
      'AWS 비용 850만원 — 팔백오십과 팔십오 혼동 가능',
    ],
  };

  it('큰 V2 fixture가 noteSections를 통과하고 올바른 섹션 구조를 만든다 (AC-8-1)', () => {
    const sections = noteSections(LARGE_V2);

    // 기본 섹션이 있다.
    expect(sections.length).toBeGreaterThanOrEqual(10);

    // 첫 섹션은 핵심 요약.
    expect(sections[0].title).toBe('핵심 요약');
    expect(sections[0].kind).toBe('list');
    expect(sections[0].items).toHaveLength(7);
  });

  it('섹션 순서가 기준 형식을 따른다: 핵심 요약 → 지난 액션 아이템 → topics → 액션 아이템 → 확인 필요 (AC-8-2)', () => {
    const titles = noteSections(LARGE_V2).map((s) => s.title);

    expect(titles[0]).toBe('핵심 요약');
    expect(titles[1]).toBe('지난 액션 아이템');
    // topics 6개
    expect(titles[2]).toBe('Q4 제품 로드맵 점검');
    expect(titles[3]).toBe('디자인 시스템 v2 전환');
    expect(titles[4]).toBe('백엔드 API 리팩토링');
    expect(titles[5]).toBe('QA 자동화 전략');
    expect(titles[6]).toBe('VOC 분석');
    expect(titles[7]).toBe('인프라 및 DevOps');
    // 마지막 둘
    expect(titles[8]).toBe('액션 아이템');
    expect(titles[9]).toBe('확인 필요');
  });

  it('previousActionItems의 비지 않은 분류 5개가 모두 grouped 섹션에 있다', () => {
    const sections = noteSections(LARGE_V2);
    const prev = sections[1];
    expect(prev.kind).toBe('grouped');
    expect(prev.groups.map((g) => g.heading)).toEqual([
      '완료', '변경됨', '진행 중', '진전 없음', '언급 없음',
    ]);
    expect(prev.groups[0].items).toHaveLength(3);
    expect(prev.groups[1].items).toHaveLength(2);
  });

  it('액션 아이템 16+개가 list 섹션에 전부 있다', () => {
    const sections = noteSections(LARGE_V2);
    const ai = sections.find((s) => s.title === '액션 아이템')!;
    expect(ai.kind).toBe('list');
    expect(ai.items.length).toBeGreaterThanOrEqual(16);
  });

  it('확인 필요 항목 6개가 list 섹션에 전부 있다', () => {
    const sections = noteSections(LARGE_V2);
    const nc = sections.find((s) => s.title === '확인 필요')!;
    expect(nc.kind).toBe('list');
    expect(nc.items).toHaveLength(6);
  });
});

describe('provenance (§7.3 · 요구 11 · 13)', () => {
  it('다섯 값이 전부 화면 값에 있다', () => {
    const { provenance } = noteView(aiNote());
    expect(provenance.provider).toBe('ollama');
    expect(provenance.model).toBe('llama3.1:8b');
    expect(provenance.promptVersion).toBe('meeting/2026-09-03');
    expect(provenance.generatedAt).toBe('2026-09-03T05:10:00.000Z');
    expect(provenance.transcriptId).toBe('t-1');
  });

  it('한 줄 표시가 다섯 값을 그대로 담는다 — 시각을 새로 만들지 않는다', () => {
    const { provenance } = noteView(aiNote());
    for (const fact of ['ollama', 'llama3.1:8b', 'meeting/2026-09-03', 't-1']) {
      expect(provenance.label).toContain(fact);
    }
    // 저장된 ISO 텍스트 그대로다. 표기 규칙이 두 벌이 되지 않는다.
    expect(provenance.label).toContain('2026-09-03T05:10:00.000Z');
  });

  it('생성된 노트를 볼 때 provenance가 함께 온다', () => {
    const view = aiNoteTab(input({ notes: [aiNote()] }));
    if (view.body.kind !== 'ready') {
      throw new Error('노트를 볼 수 있는 상태여야 한다');
    }
    expect(view.body.note.provenance.model).toBe('llama3.1:8b');
    expect(view.body.note.sections[0].title).toBe('개요');
  });
});

describe('가장 최근 노트 고르기 (ADR-0008 §9.2)', () => {
  const older = aiNote({ id: 'n-1', generatedAt: '2026-09-03T05:10:00.000Z' });
  const newer = aiNote({ id: 'n-2', generatedAt: '2026-09-03T06:00:00.000Z' });

  it('같은 mode의 노트가 여럿이면 마지막 것이다 — 저장된 순서가 곧 만들어진 순서다', () => {
    expect(latestNote([older, newer], 't-1', 'meeting')?.id).toBe('n-2');
  });

  it('다른 mode의 노트를 대신 보여주지 않는다', () => {
    expect(latestNote([older, newer], 't-1', 'study')).toBeNull();
  });

  it('다른 Transcript version의 노트를 이 Transcript의 것으로 보여주지 않는다 (§7.3)', () => {
    const otherVersion = aiNote({ id: 'n-3', transcriptId: 't-2' });
    expect(latestNote([older, otherVersion], 't-1', 'meeting')?.id).toBe('n-1');
  });

  it('재생성해도 이전 노트가 목록에서 사라지지 않는다 (INV-2)', () => {
    const notes = [older, newer];
    aiNoteTab(input({ notes }));
    expect(notes).toHaveLength(2);
    expect(notes[0]).toBe(older);
  });
});

describe('아직 읽지 못한 상태', () => {
  it('레코드를 읽기 전에는 판단하지 않는다', () => {
    expect(aiNoteTab(input({ recording: null })).body.kind).toBe('loading');
  });

  it('provider 상태를 묻기 전에는 판단하지 않는다 — idle로 접지 않는다', () => {
    expect(aiNoteTab(input({ provider: null })).body.kind).toBe('loading');
  });

  it('레코드가 가리키는 Transcript를 아직 읽지 못했으면 loading이다', () => {
    expect(aiNoteTab(input({ transcript: null })).body.kind).toBe('loading');
  });

  it('저장된 노트를 아직 읽지 못했으면 loading이다 — "노트 없음"과 다른 사실이다', () => {
    expect(aiNoteTab(input({ notes: null })).body.kind).toBe('loading');
  });
});

describe('AI 기능이 비활성인 상태 (INV-8 · §13)', () => {
  it('provider 미설정은 오류가 아니라 담담한 상태다', () => {
    const view = aiNoteTab(input({ provider: noProvider() }));
    expect(view.body.kind).toBe('disabled');
    if (view.body.kind !== 'disabled') {
      throw new Error('비활성 상태여야 한다');
    }
    expect(view.body.notice.state).toBe('notConfigured');
    expect(view.body.notice.headline).toBe(AI_DISABLED_HEADLINE);
    // 실패로 그릴 재료 자체를 주지 않는다.
    expect(view.body).not.toHaveProperty('failure');
    expect(view.body.notice).not.toHaveProperty('failure');
  });

  it('연결되지 않은 것도 실패가 아니라 비활성이다', () => {
    const view = aiNoteTab(
      input({
        provider: provider({ state: 'unavailable', models: [], failure: failure('aiProviderUnreachable') }),
      }),
    );
    expect(view.body.kind).toBe('disabled');
    if (view.body.kind !== 'disabled') {
      throw new Error('비활성 상태여야 한다');
    }
    expect(view.body.notice.state).toBe('unavailable');
    // provider가 준 failure를 화면 상태로 옮기지 않는다 — 이것은 실패 표현이 아니다.
    expect(view.body).not.toHaveProperty('failure');
    expect(view.body.notice.providerName).toBe('Ollama');
  });

  it('모델이 하나도 없는 것도 비활성이다 (요구 8)', () => {
    const view = aiNoteTab(input({ provider: provider({ state: 'noModels', models: [] }) }));
    if (view.body.kind !== 'disabled') {
      throw new Error('비활성 상태여야 한다');
    }
    expect(view.body.notice.state).toBe('noModels');
    expect(view.body.notice.resolution).toContain('모델을 설치');
  });

  it('어떤 provider 상태에서도 실패로 그려지지 않는다', () => {
    for (const state of ['notConfigured', 'unavailable', 'noModels'] as const) {
      const view = aiNoteTab(input({ provider: provider({ state, models: [] }) }));
      expect(view.body.kind).toBe('disabled');
      expect(view.body.kind).not.toBe('failed');
    }
  });

  it('무엇이 막히지 않는지 함께 말한다 (INV-8)', () => {
    const view = aiNoteTab(input({ provider: noProvider() }));
    if (view.body.kind !== 'disabled') {
      throw new Error('비활성 상태여야 한다');
    }
    expect(view.body.notice.unaffectedNotice).toBe(AI_UNAFFECTED_NOTICE);
    expect(view.body.notice.resolution).toContain('설정');
  });

  it('고른 provider가 없으면 다시 확인할 대상도 없다', () => {
    const view = aiNoteTab(input({ provider: noProvider() }));
    if (view.body.kind !== 'disabled') {
      throw new Error('비활성 상태여야 한다');
    }
    expect(view.body.notice.recheck).toBeNull();
  });

  it('닿지 못하는 provider는 다시 확인해 볼 수 있다', () => {
    const view = aiNoteTab(input({ provider: provider({ state: 'unavailable', models: [] }) }));
    if (view.body.kind !== 'disabled') {
      throw new Error('비활성 상태여야 한다');
    }
    expect(view.body.notice.recheck?.label).toContain('확인');
  });

  it('비활성 상태에서 생성을 요청했다가 받은 답도 실패로 그리지 않는다', () => {
    // provider를 고르지 않은 채 생성을 걸면 `aiProviderNotConfigured`가 상태로 돌아온다.
    // 그것은 command의 실패가 아니라 "AI가 꺼져 있다"는 사실이다 (INV-8 · ADR-0008 §13.2).
    const view = aiNoteTab(
      input({
        provider: provider(),
        live: liveStatus({
          state: 'failed',
          recordingId: 'r-1',
          mode: 'meeting',
          failure: failure('aiProviderNotConfigured', { retryable: false }),
        }),
      }),
    );
    expect(view.body.kind).toBe('disabled');
  });

  it('AI가 꺼져 있어도 Transcript 탭과 재생은 그대로다 (INV-8)', () => {
    const record = recording();
    const stored = transcript();

    // 같은 사실 위에서 AI 탭은 비활성이고,
    expect(aiNoteTab(input({ recording: record, transcript: stored, provider: noProvider() })).body.kind).toBe(
      'disabled',
    );

    // Transcript 탭은 전사된 문장을 그대로 보여주며,
    const transcriptView = transcriptTab(record, stored, null);
    expect(transcriptView.kind).toBe('done');

    // 재생 경로도 그대로 열린다.
    const detail = loadedRecordingDetail(record.id, record, [], (path) => `asset://${path}`);
    expect(detail.kind).toBe('playable');
  });
});

describe('만들 재료가 없는 상태 (§7.2)', () => {
  it('전사가 없는 것은 실패가 아니다', () => {
    const view = aiNoteTab(
      input({ recording: recording({ currentTranscriptId: null }), transcript: null }),
    );
    expect(view.body.kind).toBe('noTranscript');
    if (view.body.kind !== 'noTranscript') {
      throw new Error('재료가 없는 상태여야 한다');
    }
    expect(view.body.text).toBe(NO_TRANSCRIPT_INPUT_TEXT);
    expect(view.body.hint).toContain('전사 탭');
    expect(view.body).not.toHaveProperty('failure');
  });

  it('생성을 걸었더니 재료가 없다고 답한 것도 실패가 아니다', () => {
    const view = aiNoteTab(
      input({
        live: liveStatus({ state: 'noTranscript', recordingId: 'r-1', mode: 'meeting' }),
      }),
    );
    expect(view.body.kind).toBe('noTranscript');
  });
});

describe('아직 노트가 없는 상태', () => {
  it('만든 적이 없다는 사실과 만드는 수단이 함께 있다', () => {
    const view = aiNoteTab(input());
    if (view.body.kind !== 'none') {
      throw new Error('노트가 없는 상태여야 한다');
    }
    expect(view.body.text).toBe(NO_AI_NOTE_TEXT);
    expect(view.body.generate.kind).toBe('generate');
    expect(view.body.generate.label).toContain('회의록');
  });

  it('다른 mode의 노트가 있어도 고른 mode의 노트가 없으면 만들 수 있다', () => {
    const view = aiNoteTab(input({ notes: [aiNote()], mode: 'study' }));
    expect(view.body.kind).toBe('none');
  });
});

describe('생성 중인 상태', () => {
  it('지금 이 녹음의 생성이 돌고 있으면 그것이 먼저다', () => {
    const view = aiNoteTab(
      input({ live: liveStatus({ state: 'running', recordingId: 'r-1', mode: 'meeting' }) }),
    );
    expect(view.body.kind).toBe('generating');
    // 도는 동안에는 mode를 바꾸지 않는다.
    expect(view.modeSelectable).toBe(false);
  });

  it('다른 녹음의 생성은 이 화면과 상관이 없다', () => {
    const view = aiNoteTab(
      input({ live: liveStatus({ state: 'running', recordingId: 'r-9', mode: 'meeting' }) }),
    );
    expect(view.body.kind).toBe('none');
  });

  it('저장된 상태가 pending·running이어도 생성 중이다 — 앱을 다시 켠 뒤에도 남는 사실이다', () => {
    for (const aiStatus of ['pending', 'running'] as const) {
      const view = aiNoteTab(input({ recording: recording({ aiStatus }) }));
      expect(view.body.kind).toBe('generating');
    }
  });

  it('새로 만드는 동안에도 이미 있던 노트가 그대로 보인다 (INV-2)', () => {
    const view = aiNoteTab(
      input({
        notes: [aiNote()],
        live: liveStatus({ state: 'running', recordingId: 'r-1', mode: 'meeting' }),
      }),
    );
    if (view.body.kind !== 'generating') {
      throw new Error('생성 중이어야 한다');
    }
    expect(view.body.kept?.provenance.promptVersion).toBe('meeting/2026-09-03');
  });
});

describe('실패한 상태 (§13)', () => {
  function failed(kind: FailureKind, overrides: Partial<Failure> = {}) {
    return aiNoteTab(
      input({
        notes: [aiNote()],
        live: liveStatus({
          state: 'failed',
          recordingId: 'r-1',
          mode: 'meeting',
          failure: failure(kind, overrides),
        }),
      }),
    );
  }

  it('세 질문에 답한다 — 무엇이 실패했는가 · 원본은 안전한가 · 다시 시도할 수 있는가', () => {
    const view = failed('aiRequestFailed');
    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.headline).toBe(AI_NOTE_FAILED_HEADLINE);
    expect(view.body.failure?.kind).toBe('aiRequestFailed');
    expect(view.body.preservedNotice).toBe(AI_NOTE_PRESERVED_NOTICE);
    expect(view.body.retry.kind).toBe('retry');
    expect(view.body.retry.recordingId).toBe('r-1');
    expect(view.body.retry.mode).toBe('meeting');
  });

  it('원본이 그대로라는 사실을 말한다 — 지웠다고도 복구했다고도 하지 않는다', () => {
    const view = failed('aiResponseUnusable');
    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.preservedNotice).toContain('그대로');
    expect(view.body.preservedNotice).toContain('지워진 것은 없다');
  });

  it('실패해도 이미 있던 노트를 잃지 않는다 (INV-2 · INV-3)', () => {
    const view = failed('aiRequestFailed');
    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.kept?.sections).toHaveLength(5);
  });

  it('갈래마다 먼저 할 일이 다르다', () => {
    const cases = [
      ['aiModelUnavailable', 'modelUnavailable', '설치하거나 설정에서 다른 모델을 고른'],
      ['aiResponseUnusable', 'responseUnusable', '다시 만들면'],
      ['aiInputTooLarge', 'inputTooLarge', '아무것도 보내지 않았다'],
      ['aiProviderUnreachable', 'unreachable', '켠 뒤'],
    ] as const;

    for (const [kind, cause, resolution] of cases) {
      // provider 상태는 준비됐는데 그 시도만 실패한 경우다.
      const view = failed(kind);
      if (view.body.kind !== 'failed') {
        throw new Error('실패 상태여야 한다');
      }
      expect(view.body.cause).toBe(cause);
      expect(view.body.resolution).toContain(resolution);
    }
  });

  it('다시 시도할 수 없는 갈래에서도 재시도 수단은 남는다', () => {
    // retryable=false는 "지금 그대로 다시 눌러도 같다"는 뜻이지 "영영 만들 수 없다"가 아니다.
    const view = failed('aiModelUnavailable', { retryable: false });
    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.failure?.retryable).toBe(false);
    expect(view.body.retry.label).toContain('회의록');
  });

  it('저장된 상태만 failed면 이유를 지어내지 않는다', () => {
    const view = aiNoteTab(input({ recording: recording({ aiStatus: 'failed' }) }));
    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.failure).toBeNull();
    expect(view.body.cause).toBe('unknown');
    expect(view.body.resolution).toBe(UNKNOWN_AI_NOTE_NOTICE);
    // 이유를 모르는 것과 다시 시도할 수 없는 것은 다르다.
    expect(view.body.retry.kind).toBe('retry');
  });

  it('실패한 뒤에도 mode를 바꿔 다시 만들 수 있다', () => {
    expect(failed('aiRequestFailed').modeSelectable).toBe(true);
  });
});

describe('노트를 볼 수 있는 상태', () => {
  it('구조 그대로 보이고 다시 만들 수단이 있다 (요구 12)', () => {
    const view = aiNoteTab(input({ notes: [aiNote()] }));
    if (view.body.kind !== 'ready') {
      throw new Error('노트를 볼 수 있는 상태여야 한다');
    }
    expect(view.body.note.modeLabel).toBe('회의록');
    expect(view.body.note.sections.map((section) => section.title)).toContain('할 일');
    expect(view.body.regenerate.kind).toBe('regenerate');
    expect(view.body.regenerate.mode).toBe('meeting');
  });

  it('mode를 바꾸면 그 mode의 노트를 본다', () => {
    const notes = [aiNote(), aiNote({ id: 'n-2', mode: 'summary', note: SUMMARY })];
    const view = aiNoteTab(input({ notes, mode: 'summary' }));
    if (view.body.kind !== 'ready') {
      throw new Error('노트를 볼 수 있는 상태여야 한다');
    }
    expect(view.body.note.sections.map((section) => section.title)).toEqual([
      '짧은 요약',
      '핵심',
    ]);
  });
});

describe('전사가 어디로 가는가 (§12 · INV-5)', () => {
  it('로컬 provider는 기기를 떠나지 않는다고 말한다', () => {
    const view = aiNoteTab(input());
    expect(view.provider?.locality).toBe('local');
    expect(view.provider?.label).toContain('기기를 벗어나지 않는다');
    expect(view.provider?.label).toContain('오디오는 어떤 경우에도 보내지 않는다');
  });

  it('외부 provider는 전사가 나간다고 말한다 — audio는 어느 쪽이든 나가지 않는다', () => {
    const view = aiNoteTab(
      input({ provider: provider({ providerName: 'Some Cloud', locality: 'external' }) }),
    );
    expect(view.provider?.label).toContain('전사 텍스트가 그쪽으로 간다');
    expect(view.provider?.label).toContain('오디오는 어떤 경우에도 보내지 않는다');
  });

  it('고른 provider가 없으면 할 말이 없다', () => {
    expect(aiNoteTab(input({ provider: noProvider() })).provider).toBeNull();
  });
});

describe('거절된 요청 (§13)', () => {
  it('어느 요청이 거절됐는지 구분해서 말한다', () => {
    expect(aiNoteTrouble('provider', 'boom').headline).toContain('AI provider 상태');
    expect(aiNoteTrouble('status', 'boom').headline).toContain('AI 노트 상태');
    expect(aiNoteTrouble('notes', 'boom').headline).toContain('저장된 AI 노트');
    expect(aiNoteTrouble('start', 'boom').headline).toContain('시작하지 못했다');
  });

  it('구조화되지 않은 값도 화면에 보여줄 수 있는 실패가 된다', () => {
    const trouble = aiNoteTrouble('start', new Error('IPC 통로가 없다'));
    expect(trouble.failure.kind).toBe('unexpected');
    expect(trouble.failure.detail).toBe('IPC 통로가 없다');
  });

  it('Rust가 보낸 실패는 그대로 쓴다', () => {
    const original = failure('aiRequestFailed');
    expect(aiNoteTrouble('start', original).failure).toBe(original);
  });
});
