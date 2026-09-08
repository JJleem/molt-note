/**
 * Settings 화면의 상태 (PRODUCT-SPEC §5 D).
 *
 * command 호출 결과를 화면 상태로 바꾸는 규칙만 있다. React도 DOM도 Tauri도 알지 않으므로
 * 읽기 · 편집 · 저장 · 실패 경로를 vitest로 그대로 판정할 수 있다 (§18).
 *
 * **INV-7: secret이 없다.** API key · integration token은 이 상태에도, 폼에도 없다.
 * 전사 모델 값은 secret이 아니라 **파일이 어디 있는지**다 (ADR-0007 §8.2). 같은 이유로
 * Notion destination도 secret이 아니다 — **어디에 쓰는지**일 뿐이다 (ADR-0009 §8.4).
 *
 * **이 모듈은 사용자의 설정 값을 대신 고치지 않는다.** 모델이 없어서 지금 전사할 수 없다는
 * 것은 사실이지만, 그 사실 때문에 자동 전사 토글을 뒤집지 않는다 — 그 상태는 값을 바꾸는
 * 대신 {@link transcriptionNotices}가 말한다 (ADR-0007 §8.2.3).
 */
import { toFailure, type Failure } from '../ipc/failure';
import type { Settings } from '../ipc/types';
import { chosenMicrophone, NO_DEFAULT_MICROPHONE } from './defaultMicrophone';

/**
 * 입력란이 들고 있는 값.
 *
 * `recordingsDirectory`가 `Settings`와 달리 `null`이 아니라 빈 문자열인 이유는 하나다 —
 * 텍스트 입력은 `null`을 표현할 수 없다. 두 표현 사이의 변환은 이 모듈에만 있다.
 */
export interface SettingsForm {
  readonly recordingsDirectory: string;
  readonly automaticProcessing: boolean;
  /**
   * 정지해 저장한 직후에 전사를 자동으로 시작할지 여부.
   *
   * **`automaticProcessing`과 다른 값이다.** 하나를 켜는 것이 다른 하나를 켜지 않는다.
   */
  readonly automaticTranscription: boolean;
  /**
   * 전사에 쓸 모델의 이름 또는 경로. 고르지 않았으면 빈 문자열이다 —
   * `recordingsDirectory`와 같은 이유로 텍스트 입력은 `null`을 표현할 수 없다.
   */
  readonly transcriptionModel: string;
  /**
   * 전사할 때 무슨 언어로 들을지. **고르지 않았으면 빈 문자열이며, 그것은 자동 감지다**
   * (docs/ADR-0007-transcription-engine.md §17.1.4-1).
   *
   * `transcriptionModel`과 같은 이유로 `null`이 아니라 빈 문자열이다 — 텍스트 입력은 `null`을
   * 표현할 수 없고, 두 표현 사이의 변환은 이 모듈에만 있다.
   *
   * **폼이 이 값을 들고 있어야 하는 이유는 `notionParentPageId`와 같다** — {@link toSettings}가
   * 폼 전체를 저장할 값으로 옮기므로, 폼에 없는 설정은 저장할 때마다 `null`이 되어 사용자가
   * 고른 언어가 조용히 지워진다. 그리고 이 값에서 그것은 특히 조용하다: 지워진 뒤에도 화면은
   * 여전히 정상으로 보이고, 다음 전사가 자동 감지로 돌아간 것을 사용자는 결과를 보고서야 안다.
   */
  readonly transcriptionLanguage: string;
  /**
   * 고른 입력 장치의 선택 키. 고르지 않았으면 `NO_DEFAULT_MICROPHONE`(빈 문자열)이다 —
   * `<select>`의 값도 `null`을 담을 수 없다.
   *
   * **이 값이 지금 있는 장치인지는 여기서 묻지 않는다.** 없어진 장치도 고른 값 그대로
   * 남으며, 그 사실을 말하는 것은 `defaultMicrophone.ts`다.
   */
  readonly defaultMicrophone: string;
  /**
   * 고른 AI provider의 식별자. 고르지 않았으면 `NO_AI_PROVIDER`(빈 문자열)이다
   * (docs/ADR-0008-note-ai-provider.md §11.1).
   *
   * `defaultMicrophone`과 같은 이유로 `null`이 아니라 빈 문자열이다 — `<select>`의 값은
   * `null`을 담을 수 없다. 두 표현 사이의 변환은 이 모듈에만 있다.
   *
   * **고르지 않은 것은 오류가 아니다** (INV-8). 그 상태에서도 나머지 설정은 그대로 저장되며,
   * 그 사실을 말하는 것은 `aiProviderSettings.ts`다.
   */
  readonly aiProvider: string;
  /**
   * provider에 연결할 주소. 비어 있으면 고르지 않은 것이며, 그때 실제로 어디에 연결하는지는
   * **backend가 안다** (`Settings::ai_base_url_or_default`).
   *
   * 기본 주소를 이 화면에 옮겨 적지 않는다 — 같은 주소가 두 곳에 있으면 한 곳을 고쳤을 때
   * 나머지가 조용히 달라진다 (`src-tauri/src/domain/settings.rs`의 `DEFAULT_AI_BASE_URL`).
   */
  readonly aiBaseUrl: string;
  /**
   * 노트를 만들 때 쓸 모델. 고르지 않았으면 빈 문자열이다.
   *
   * `transcriptionModel`과 같은 성질을 갖는다 — 이 모델이 지금 그 서버에 설치돼 있는지는
   * 이 값만으로 알 수 없고, 없다고 해서 앱이 값을 지우거나 다른 모델로 바꾸지 않는다.
   */
  readonly aiModel: string;
  /**
   * Notion 페이지를 만들 부모 페이지의 식별자. 고르지 않았으면 빈 문자열이다
   * (docs/ADR-0009-notion-and-export.md §8.4).
   *
   * Settings 화면의 Notion 구역이 이 값을 편집하며 (Phase 5 · `notionSettings.ts`), 저장은
   * 화면 전체의 Save 하나가 한다 — 설정은 한 벌이기 때문이다. 폼이 이 값을 들고 있어야 하는
   * 이유는 그 전부터 같았다: {@link toSettings}가 폼 전체를 저장할 값으로 옮기므로, 폼에 없는
   * 설정은 저장할 때마다 `null`이 되어 **사용자가 고른 값이 조용히 지워진다.**
   *
   * **secret이 아니다** — 어디에 쓰는지일 뿐이다. integration 자격증명은 이 폼에도, 이
   * 화면의 어떤 상태에도 없다 (INV-7 · ADR-0009 §10.4).
   */
  readonly notionParentPageId: string;
}

/**
 * 화면이 놓일 수 있는 상태의 전부.
 *
 * `failed`는 **설정을 읽지 못한** 상태다 — 편집할 값 자체가 없다. 저장에 실패한 것은
 * 편집 중인 값이 그대로 남아 있는 `ready`의 한 모습이며(`failure`가 채워진다), 둘을 섞지 않는다.
 */
export type SettingsView =
  | { readonly kind: 'loading' }
  | {
      readonly kind: 'ready';
      readonly form: SettingsForm;
      readonly saving: boolean;
      /** 마지막 저장이 성공했고, 그 뒤로 편집이 없었다. */
      readonly saved: boolean;
      /** 마지막 저장이 실패했다면 그 실패. 성공했거나 아직 저장하지 않았으면 `null`이다. */
      readonly failure: Failure | null;
    }
  | { readonly kind: 'failed'; readonly failure: Failure };

export const LOADING_SETTINGS: SettingsView = { kind: 'loading' };

/** 저장된 설정을 읽었다. 저장된 적이 없어 기본값이 온 것도 정상이다. */
export function loadedSettings(settings: Settings): SettingsView {
  return ready(toForm(settings));
}

/** 설정을 읽지 못했다. 저장소 초기화 실패가 여기로 온다 (§13). */
export function failedSettings(error: unknown): SettingsView {
  return { kind: 'failed', failure: toFailure(error) };
}

/**
 * 입력란 하나가 바뀌었다.
 *
 * 아직 읽지 못했거나 읽지 못한 상태에서는 편집할 값이 없으므로 상태가 바뀌지 않는다.
 */
export function editedSettings(view: SettingsView, change: Partial<SettingsForm>): SettingsView {
  if (view.kind !== 'ready') {
    return view;
  }
  // 편집한 순간 "저장됨"은 더 이상 사실이 아니다. 직전 저장 실패는 아직 사실이므로 남긴다.
  return { ...view, form: { ...view.form, ...change }, saved: false };
}

/** 저장을 시작했다. 이전 저장 실패는 이 시점에 지운다 — 지금 무엇이 진행 중인지가 답이다. */
export function savingSettings(view: SettingsView): SettingsView {
  if (view.kind !== 'ready') {
    return view;
  }
  return { ...view, saving: true, saved: false, failure: null };
}

/**
 * 저장이 끝났다. 폼은 **저장소가 돌려준 값**으로 다시 채운다.
 *
 * 화면이 "무엇이 저장됐는가"를 추측하지 않게 하기 위해서다 — 공백만 있던 디렉터리처럼
 * Rust가 정규화한 값이 있으면 그 값이 그대로 보인다 (`SettingsPayload`).
 */
export function savedSettings(settings: Settings): SettingsView {
  return { kind: 'ready', form: toForm(settings), saving: false, saved: true, failure: null };
}

/**
 * 저장이 실패했다.
 *
 * 편집 중이던 값은 그대로 둔다 — 실패했다고 사용자가 입력한 것을 버리지 않는다.
 * 읽기 자체가 되지 않은 상태에서 온 실패라면 읽지 못한 화면으로 남는다.
 */
export function failedSave(view: SettingsView, error: unknown): SettingsView {
  const failure = toFailure(error);
  if (view.kind !== 'ready') {
    return { kind: 'failed', failure };
  }
  return { ...view, saving: false, saved: false, failure };
}

/** 폼 값을 저장할 수 있는 설정으로 옮긴다. 빈 입력은 "고르지 않음"(`null`)이다. */
export function toSettings(form: SettingsForm): Settings {
  const directory = form.recordingsDirectory.trim();
  const model = form.transcriptionModel.trim();
  return {
    recordingsDirectory: directory === '' ? null : directory,
    automaticProcessing: form.automaticProcessing,
    // 두 토글은 각자 그대로 간다. 모델이 없다는 이유로 여기서 토글을 뒤집지 않는다 —
    // 사용자가 켠 값을 앱이 대신 끄면, 무엇이 왜 꺼졌는지 말할 수 없게 된다 (ADR-0007 §8.2.3).
    automaticTranscription: form.automaticTranscription,
    // 입력한 값 그대로 보낸다(앞뒤 공백만 뺀다). 그 파일이 지금 있는지 여기서 찾아보지 않고,
    // 없다고 해서 다른 모델로 바꾸지도 않는다 — `defaultMicrophone`과 같은 이유다.
    transcriptionModel: model === '' ? null : model,
    // 빈 입력은 "고르지 않음"(`null`)이고 **그것은 자동 감지다** — 여기서 로캘을 짐작해
    // 언어를 채우지 않는다. 앞뒤 공백만 뺀다: 그 코드를 엔진이 아는지 화면은 알 수 없고,
    // 모른다고 짐작해 다른 언어로 바꾸지도 않는다 (`transcriptionModel`과 같은 규칙).
    transcriptionLanguage: chosen(form.transcriptionLanguage),
    // 고른 키는 그대로 보낸다. 지금 없는 장치라도 **사용자가 고른 값이므로 바꾸지 않는다.**
    defaultMicrophone: chosenMicrophone(form.defaultMicrophone),
    // AI 설정 세 값도 같은 규칙이다 — 빈 입력은 "고르지 않음"(`null`)이며 **그것뿐이다.**
    // provider가 응답하는지도, 그 모델이 설치돼 있는지도 여기서 묻지 않고, 아니라고 짐작해
    // 다른 값으로 바꾸지도 않는다.
    aiProvider: chosen(form.aiProvider),
    aiBaseUrl: chosen(form.aiBaseUrl),
    aiModel: chosen(form.aiModel),
    // 화면이 편집하지 않는 값이지만 **그대로 지나간다.** 여기서 `null`로 적으면 다른 설정을
    // 저장할 때마다 사용자가 고른 Notion destination이 조용히 지워진다.
    notionParentPageId: chosen(form.notionParentPageId),
  };
}

/** 텍스트 입력 하나를 저장할 값으로 옮긴다. 공백뿐인 입력은 "고르지 않음"(`null`)이다. */
function chosen(value: string): string | null {
  const trimmed = value.trim();
  return trimmed === '' ? null : trimmed;
}

/** 설정을 폼 값으로 옮긴다. 고르지 않은 디렉터리(`null`)는 빈 입력이다. */
export function toForm(settings: Settings): SettingsForm {
  return {
    recordingsDirectory: settings.recordingsDirectory ?? '',
    automaticProcessing: settings.automaticProcessing,
    automaticTranscription: settings.automaticTranscription,
    transcriptionModel: settings.transcriptionModel ?? '',
    // 고르지 않은 언어(`null`)도 빈 입력이다. 그 빈칸이 뜻하는 것은 자동 감지이며,
    // 그 사실을 말하는 것은 {@link transcriptionLanguageNotice}다.
    transcriptionLanguage: settings.transcriptionLanguage ?? '',
    defaultMicrophone: settings.defaultMicrophone ?? NO_DEFAULT_MICROPHONE,
    // 고르지 않은 상태(`null`)는 빈 입력이다 — `recordingsDirectory`와 같은 이유다.
    aiProvider: settings.aiProvider ?? '',
    aiBaseUrl: settings.aiBaseUrl ?? '',
    aiModel: settings.aiModel ?? '',
    notionParentPageId: settings.notionParentPageId ?? '',
  };
}

/**
 * 전사가 지금 실행될 수 있는 상태인가 — **설정 값만으로 말할 수 있는 데까지다.**
 *
 * ```text
 * notChosen  모델을 아직 고르지 않았다  → 지금은 전사할 수 없다
 * chosen     모델을 골랐다             → 그 파일이 실제로 그 자리에 있는지는 여기서 알 수 없다
 * ```
 *
 * `chosen`이 "전사할 수 있다"는 뜻은 아니다. 파일이 실제로 있는지 여는 자리는 backend 하나이고
 * (`src-tauri/src/transcription/model.rs`), 없으면 그 전사가 §13의 실패로 알린다. 화면이
 * 파일을 찾아보는 척하지 않는다 — 알 수 없는 것을 아는 것처럼 적지 않는다.
 */
export type TranscriptionModel =
  | { readonly kind: 'notChosen' }
  | { readonly kind: 'chosen'; readonly value: string };

/** 고른 모델이 있는가. 공백뿐인 입력은 고르지 않은 것과 같다. */
export function transcriptionModel(form: SettingsForm): TranscriptionModel {
  const value = form.transcriptionModel.trim();
  return value === '' ? { kind: 'notChosen' } : { kind: 'chosen', value };
}

/** 모델이 없어서 지금 전사할 수 없다는 **사실**. */
export const NO_TRANSCRIPTION_MODEL_NOTICE =
  '전사 모델이 지정되지 않아서 지금은 녹음을 전사할 수 없다.';

/** 그 사실을 어떻게 푸는지 (docs/ADR-0007-transcription-engine.md §8.2). */
export const HOW_TO_SET_A_TRANSCRIPTION_MODEL =
  "Put a Whisper model file (for example ggml-base.bin) in the app's models folder and enter its file name here, or enter the full path to a model kept somewhere else.";

/** 켜 둔 자동 전사를 **앱이 대신 끄지 않는다**는 사실 (ADR-0007 §8.2.3). */
export const AUTOMATIC_TRANSCRIPTION_STAYS_ON_NOTICE =
  '자동 전사는 켜진 채로 둔다 — 대신 꺼 주지 않는다. 모델이 지정되기 전까지는 녹음마다 모델이 없다는 사실을 알린다.';

/**
 * 전사 설정에 대해 사용자에게 할 말. 할 말이 없으면 빈 목록이다.
 *
 * **모델이 없는 상태를 조용한 skip으로 두지 않는다** — 그것은 설정 화면에 보이는 제품 상태이며,
 * 무엇이 사실이고 그것을 어떻게 푸는지가 함께 온다 (ADR-0007 §8.2.3 · §13).
 *
 * 자동 전사가 켜져 있다면 한 줄이 더 붙는다. 붙는 것은 **말**뿐이고 값은 그대로다 —
 * 이 모듈에는 `automaticTranscription`을 뒤집는 경로가 없다. 사용자가 켠 것은 켜진 채로 남고,
 * 지금 실행할 수 없다는 사실은 그것과 별개의 상태로 표현된다.
 */
export function transcriptionNotices(form: SettingsForm): string[] {
  if (transcriptionModel(form).kind === 'chosen') {
    return [];
  }

  const notices = [NO_TRANSCRIPTION_MODEL_NOTICE, HOW_TO_SET_A_TRANSCRIPTION_MODEL];
  if (form.automaticTranscription) {
    notices.push(AUTOMATIC_TRANSCRIPTION_STAYS_ON_NOTICE);
  }
  return notices;
}

/**
 * 전사를 무슨 언어로 들을 것인가 — **설정 값만으로 말할 수 있는 데까지다.**
 *
 * ```text
 * autoDetect  언어를 고르지 않았다  → 엔진이 녹음마다 감지한다 (오류가 아니라 정상 상태다)
 * chosen      언어를 골랐다        → 그 언어로 듣는다
 * ```
 *
 * **`autoDetect`는 "비어 있다"가 아니라 하나의 선택이다** (ADR-0007 §17.1.4-1). 고르지 않은
 * 자리에 앱이 언어를 대신 채워 넣지 않으며, 특히 **사용자의 로캘을 짐작하지 않는다** — 화면
 * 언어는 말하는 언어가 아니다. 짐작한 값이 틀렸을 때 무엇이 되는지는 이미 봤다: 아무도 고른
 * 적 없는 `en` 하나가 72분짜리 한국어 회의를 통째로 못 쓰게 만들었다 (§17.1.1).
 *
 * `chosen`이 "그 언어로 전사된다"는 보장은 아니다. 그 코드를 엔진이 아는지 화면은 알 수 없고,
 * 실제로 무엇으로 들었는지는 만들어진 Transcript의 `language`에 남는다 — 알 수 없는 것을 아는
 * 것처럼 적지 않는다 ({@link transcriptionModel}과 같은 태도).
 */
export type TranscriptionLanguage =
  | { readonly kind: 'autoDetect' }
  | { readonly kind: 'chosen'; readonly value: string };

/** 고른 언어가 있는가. 공백뿐인 입력은 고르지 않은 것과 같다. */
export function transcriptionLanguage(form: SettingsForm): TranscriptionLanguage {
  const value = form.transcriptionLanguage.trim();
  return value === '' ? { kind: 'autoDetect' } : { kind: 'chosen', value };
}

/**
 * 언어 입력란이 비어 있을 때 그 칸에 보이는 말.
 *
 * `Not set`이 아니다 — 다른 설정에서는 그것이 맞지만, 여기서 비어 있음은 **아직 안 한 일이
 * 아니라 자동 감지**다 (ADR-0007 §17.1.4-1).
 */
export const TRANSCRIPTION_LANGUAGE_PLACEHOLDER = '자동 감지';

/** 고르지 않은 상태가 무엇을 뜻하는지 (ADR-0007 §17.1.4-1). **결핍이 아니라 동작이다.** */
export const TRANSCRIPTION_LANGUAGE_IS_DETECTED_NOTICE =
  '언어가 지정되지 않아서 녹음마다 언어를 자동으로 감지한다. 이것은 정상 설정이며 빠뜨린 것이 아니다.';

/** 그 자리에서 무엇을 할 수 있는지. 고르는 것도, 고르지 않은 채 두는 것도 유효한 선택이다. */
export const HOW_TO_SET_A_TRANSCRIPTION_LANGUAGE =
  '비워 두면 언어를 계속 자동 감지한다. 언어 코드를 넣으면(예: ko 또는 en) 모든 녹음을 그 언어로 전사한다.';

/**
 * 전사 언어에 대해 사용자에게 할 말. **언제나 한 줄 이상 있다.**
 *
 * 고르지 않은 상태에서 아무 말도 하지 않으면, 그 빈칸은 "아직 안 한 일"로 읽힌다 — 그런데
 * 그것은 사실이 아니라 **자동 감지라는 동작**이다. 그래서 이 함수는 빈 목록을 돌려주지 않는다
 * ({@link transcriptionNotices}는 할 말이 없으면 비지만, 그쪽은 결핍에 대한 말이고 이쪽은
 * 지금 무엇이 일어나는지에 대한 말이다).
 */
export function transcriptionLanguageNotices(form: SettingsForm): string[] {
  const language = transcriptionLanguage(form);
  if (language.kind === 'chosen') {
    // 고른 값을 그대로 되읽어 준다. 앱이 그 코드를 해석하거나 이름으로 바꾸지 않는다 —
    // 무엇을 아는 엔진인지 화면은 알지 못한다.
    return [
      `Recordings are transcribed as "${language.value}", so the language is not detected.`,
      HOW_TO_SET_A_TRANSCRIPTION_LANGUAGE,
    ];
  }
  return [TRANSCRIPTION_LANGUAGE_IS_DETECTED_NOTICE, HOW_TO_SET_A_TRANSCRIPTION_LANGUAGE];
}

function ready(form: SettingsForm): SettingsView {
  return { kind: 'ready', form, saving: false, saved: false, failure: null };
}
