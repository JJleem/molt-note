//! 진행 중인 **Notion 전송**을 소유하는 자리.
//!
//! 1시간 분량 transcript는 여러 요청으로 나뉘어 나가고, 속도 제한을 만나면 그 사이에 기다리기도
//! 한다 (ADR-0009 §6 · §9). 그동안 사용자가 목록을 보거나 설정을 바꾸는 일이 멈추면 안 되고,
//! 화면을 떠났다 왔다는 이유로 진행 중인 전송이 사라져도 안 된다.
//!
//! **그 두 요구는 전사에서 이미 풀렸고, 여기서 같은 방식을 쓴다** ([`super::Transcriber`] ·
//! [`super::NoteGenerator`] · `transcriber` 모듈 문서 · R-001). 새로운 실행 방식을 만들지 않는다.
//!
//! ```text
//! start ──────→ NotionSender ─→ 배경 스레드 ──→ sync::run::send
//!                     │             (자물쇠를 쥐지 않고 도는 구간)
//! status ───→ 상태 한 값  ←────┘  끝날 때 결과를 여기에 남긴다
//! ```
//!
//! ## 상태 조회가 전송을 기다리지 않는다
//!
//! 배경 스레드는 [`NotionSender::state`]의 자물쇠를 **시작할 때와 끝날 때만** 잡는다. 요청이
//! 나가는 동안에도, 속도 제한 때문에 기다리는 동안에도 아무 자물쇠도 쥐고 있지 않으므로
//! [`NotionSender::status`]는 즉시 답한다.
//!
//! 저장소 연결도 같은 이유로 새로 연다 ([`send_one`]) — 앱이 들고 있는 연결([`super::Storage`])을
//! 전송 내내 붙들면 목록 조회 같은 다른 command가 그 시간만큼 멈춘다.
//!
//! ## 한 번에 한 건이다
//!
//! 여러 Recording을 줄 세우는 큐는 이 Phase의 범위 밖이다 (ADR-0009 §13의 `일괄 sync`).
//! 전송 중에 들어온 시작 요청은 줄을 서지 않고 [`already_sending`]으로 거절된다 — 조용히
//! 무시하면 사용자는 자신이 누른 것이 접수됐는지 알 수 없다 (§13).
//!
//! ## token은 지나갈 뿐 머무르지 않는다 (INV-7 · ADR-0009 §10.4)
//!
//! 값을 읽는 자리는 배경 스레드 안의 [`send_one`] 하나이고, 그 값은 요청의 헤더가 되는 것
//! 말고는 아무 데도 가지 않는다. **이 타입은 token을 들고 있지 않으며**, 상태값에도 실패
//! 문장에도 담길 자리가 없다.

use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use tauri::Manager;

use crate::db::{self, settings};
use crate::domain::{Failure, FailureKind, RecordingId};
use crate::notion::{HttpTransport, NotionClient, UreqNotionTransport};
use crate::platform::app_data_dir::AppDataDirectory;
use crate::platform::secret_store::{app_secret_store, Secret, SecretKey, SecretStore};
use crate::sync::pace::SleepingWaiter;
use crate::sync::run::{self, Confirmation, Destination, Sent};
use crate::sync::Waiter;

use super::payload::{NotionConnectionPayload, NotionTokenStatusPayload};

/// 전송 한 건이 있을 수 있는 상태. **네 가지가 전부다.**
///
/// 끝난 전송을 지우지 않고 남겨 두는 이유는 화면이 결과를 물어볼 수 있어야 하기 때문이다 —
/// 성공이면 어느 페이지가 됐는지, 실패면 무엇이 실패했는지. 다음 시작이 이 값을 덮는다.
///
/// **영속적인 사실은 여기 있지 않다.** 어디까지 보냈는지는 `notion_syncs` 행에 있고
/// (ADR-0009 §8.4), 이 값은 앱이 켜져 있는 동안의 진행 상황일 뿐이다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotionSendStatus {
    /// 아직 아무것도 보내지 않았다.
    Idle,
    /// 이 녹음을 지금 보내고 있다.
    Running { recording_id: String },
    /// 이 녹음이 이 페이지가 됐다.
    Done {
        recording_id: String,
        page_id: String,
        /// 이번 실행에서 **새로 만든** 페이지인가. 이어 보낸 것이면 `false`다.
        created_page: bool,
    },
    /// 전송이 실패했다. 오디오 · Transcript · AI Note는 그대로다 (INV-3).
    ///
    /// **부분 전송 뒤의 실패도 여기로 온다.** 어디까지 갔는지는 저장된 `NotionSync`가 말한다.
    Failed {
        recording_id: String,
        failure: Failure,
    },
}

/// 앱이 들고 있는 **Notion 전송 실행자**. 진행 중인 전송의 소유자다.
///
/// [`super::Transcriber`]와 같은 방식으로 갈라져 있다 — 실제 왕복은
/// [`HttpTransport`] 뒤에 있고, 테스트는 그 자리에 double을 넣어 **실제 Notion 워크스페이스도
/// 실제 자격증명 저장소도 없이** 이 경로를 그대로 지난다 ([`Self::with_transport`] · §18).
///
/// [`super::Storage`]처럼 **앱 데이터 디렉터리를 얻지 못한 실패도 값으로 들고 있다.** 그 실패는
/// 보내려 할 때 사용자에게 그대로 전달된다 — 앱 시작을 막지 않는다 (§13).
pub struct NotionSender {
    /// 실제로 왕복하는 쪽. 배경 스레드와 공유하므로 [`Arc`]다.
    transport: Arc<dyn HttpTransport>,
    /// token을 읽어 오는 자리. **여기에 값이 담기지는 않는다** (INV-7).
    secrets: Arc<dyn SecretStore>,
    /// 속도 제한을 만났을 때 실제로 기다리는 자리 (ADR-0009 §9.2).
    waiter: Arc<dyn Waiter>,
    /// DB가 파생되는 뿌리. 얻지 못했다면 그 실패를 들고 있다.
    app_data_dir: Result<AppDataDirectory, Failure>,
    /// 지금 상태 한 값. 배경 스레드와 공유한다.
    state: Arc<Mutex<NotionSendStatus>>,
}

impl NotionSender {
    /// Tauri가 결정한 앱 데이터 디렉터리 아래에서 **실제 Notion**으로 보낸다 (INV-10).
    pub fn open_for<R, M>(manager: &M) -> Self
    where
        R: tauri::Runtime,
        M: Manager<R>,
    {
        Self {
            transport: Arc::new(UreqNotionTransport::new()),
            // 실제 자격증명 저장소를 **여기서 세우지 않는다** — 어느 구현이 서는지는 platform
            // 경계가 정한다 (INV-10 · `crate::platform::secret_store`).
            secrets: app_secret_store(),
            waiter: Arc::new(SleepingWaiter),
            app_data_dir: AppDataDirectory::from_manager(manager).map_err(Into::into),
            state: Arc::new(Mutex::new(NotionSendStatus::Idle)),
        }
    }

    /// 주어진 디렉터리에서 주어진 transport · 자격증명 저장소 · 대기 자리로 보낸다.
    ///
    /// **자동 검증이 지나는 자리다** — 계약이 같은 double을 넣으면 실제 Notion에 한 번도 닿지
    /// 않고, 사용자의 Keychain에 아무것도 남기지 않으며, 한 밀리초도 자지 않고 이 경로 전체를
    /// 그대로 지난다 (§18 · `phase-prompt/05` Important Rules).
    pub fn with_transport(
        app_data_dir: AppDataDirectory,
        transport: Arc<dyn HttpTransport>,
        secrets: Arc<dyn SecretStore>,
        waiter: Arc<dyn Waiter>,
    ) -> Self {
        Self {
            transport,
            secrets,
            waiter,
            app_data_dir: Ok(app_data_dir),
            state: Arc::new(Mutex::new(NotionSendStatus::Idle)),
        }
    }

    /// Recording 하나의 Notion 전송을 시작하고, 그 결과 상태를 돌려준다.
    ///
    /// **돌아오는 것은 전송 결과가 아니라 접수 사실이다.** 실제 전송은 배경 스레드에서 돌고,
    /// 이 호출은 스레드를 만든 뒤 바로 끝난다 — 1시간짜리 transcript를 걸어도 이 command가
    /// IPC를 붙잡지 않는다 (전사 · 노트 생성과 같은 규약).
    ///
    /// 이미 보내는 중이면 **거절한다** ([`already_sending`]). 같은 Recording이어도 마찬가지다.
    ///
    /// token이 저장돼 있는지, 부모 페이지를 골랐는지, 그 녹음에 전사가 있는지는 여기서 묻지
    /// 않는다 — 저장소와 자격증명 저장소를 지나는 일은 전부 배경 스레드의 몫이며, 그 판정은
    /// `failed` 상태에 실려 화면에 도달한다 ([`send_one`] · 노트 생성과 같은 규칙).
    pub fn start(
        &self,
        recording_id: &str,
        confirmation: Confirmation,
    ) -> Result<NotionSendStatus, Failure> {
        let app_data_dir = self.app_data_dir.as_ref().map_err(Clone::clone)?.clone();

        let recording_id = recording_id.trim();
        if recording_id.is_empty() {
            return Err(Failure::permanent(
                FailureKind::InvalidInput,
                "보낼 녹음을 고르지 않았다.",
            ));
        }

        let mut state = self.state()?;
        if let NotionSendStatus::Running {
            recording_id: running,
        } = &*state
        {
            return Err(already_sending(running, recording_id));
        }
        let running = NotionSendStatus::Running {
            recording_id: recording_id.to_string(),
        };
        *state = running.clone();
        // 자물쇠를 놓고 나서 스레드를 만든다 — 시작하는 쪽이 자물쇠를 쥔 채로 남지 않게 한다.
        drop(state);

        self.spawn(recording_id.to_string(), confirmation, app_data_dir);
        Ok(running)
    }

    /// 지금 전송이 어떤 상태인지.
    ///
    /// **요청이 나가는 동안에도, 속도 제한 때문에 기다리는 동안에도 즉시 답한다.** 이 함수가
    /// 잡는 자물쇠를 배경 스레드는 시작할 때와 끝날 때만 잡기 때문이다 (모듈 문서).
    pub fn status(&self) -> Result<NotionSendStatus, Failure> {
        Ok(self.state()?.clone())
    }

    /// 저장된 token으로 **지금 Notion과 말할 수 있는지** 확인한다 (§5-D · ADR-0009 §5.1).
    ///
    /// 묻는 것은 하나다 — "이 token으로 말할 수 있는가". 어떻게 묻는지도, 그 답이 어떤 모양으로
    /// 오는지도 이 파일은 알지 않는다 ([`NotionClient::check_connection`]). 확인이 **어느
    /// 워크스페이스였는지**까지 말해 줬다면 그 값은 여기서 해석되지 않고 그대로 지나간다 (§5-D).
    ///
    /// ```text
    /// token 없음        notConfigured — 요청을 보내지 않는다. 정상 상태다 (INV-8)
    /// 확인됨            connected + 확인이 말해 준 워크스페이스 이름(말하지 않았으면 없음)
    /// 확인하지 못함      failed + §13의 실패 그대로 (token · destination · 네트워크가 갈린다)
    /// ```
    ///
    /// **부모 페이지를 고르지 않았다는 이유로 거절하지 않는다.** 그 사실은
    /// [`NotionConnectionPayload::destination_configured`]로 함께 오며, token이 유효한지와는
    /// 다른 질문이다 — 사용자는 둘 중 무엇이 남았는지 알 수 있어야 한다 (§13).
    ///
    /// `Err`가 되는 경우는 하나뿐이다: **자격증명 저장소를 읽지 못했을 때.** 그것은 실제로
    /// 시도했다가 실패한 일이며 Notion과 무관하다 ([`super::ai_provider_status`]와 같은 규칙).
    ///
    /// **token은 이 함수 안에서만 산다** (INV-7). 돌아가는 값에도 실패 문장에도 실리지 않는다.
    pub fn check_connection(
        &self,
        parent_page_id: Option<&str>,
    ) -> Result<NotionConnectionPayload, Failure> {
        let destination_configured = parent_page_id
            .map(str::trim)
            .is_some_and(|page| !page.is_empty());

        // 저장한 적이 없으면 **요청을 보내지 않는다.** 보낼 것이 없는데 왕복하면 사용자는
        // "연결 실패"를 보게 되고, 그것은 아직 설정하지 않은 상태와 다른 말이다.
        let Some(token) = self.secrets.get(SecretKey::NotionIntegrationToken)? else {
            return Ok(NotionConnectionPayload::not_configured(
                destination_configured,
            ));
        };

        let client = NotionClient::new(Arc::clone(&self.transport));

        Ok(match client.check_connection(&token) {
            // 어느 워크스페이스에 연결됐는지는 **Notion이 말한 그대로** 지나간다 (§5-D).
            // 말해 주지 않았으면 `None`이며, 화면은 이름 없이 연결됐다는 사실만 보인다.
            Ok(identity) => {
                NotionConnectionPayload::connected(destination_configured, identity.workspace_name)
            }
            Err(failure) => {
                NotionConnectionPayload::failed(destination_configured, failure.into_failure())
            }
        })
    }

    /// integration token을 자격증명 저장소에 넣고, **저장 여부를 다시 읽어** 돌려준다.
    ///
    /// 값은 이 함수의 인자로 한 번 지나갈 뿐이며, 돌아가는 것은 저장돼 있다는 사실 하나다
    /// (INV-7 · ADR-0009 §10.4). 실패 문장에도 값이 섞이지 않는다 — 빈 값을 거절하는 실패조차
    /// 받은 것을 되돌려 적지 않는다 ([`super::notes::parse_mode`]가 mode를 `detail`에 적는 것과
    /// 갈리는 지점이다: 그 값은 secret이 아니었다).
    ///
    /// 앞뒤 공백은 벗긴다 — 붙여 넣은 값에 딸려 온 줄바꿈이 token을 다른 값으로 만들지 않게
    /// 한다. **그것뿐이다**: 모양을 검사하지도, 유효한지 물어보지도 않는다. 그 답은
    /// [`Self::check_connection`]이 실제로 물어봐서 얻는다.
    pub fn save_token(&self, token: &str) -> Result<NotionTokenStatusPayload, Failure> {
        let token = token.trim();
        if token.is_empty() {
            return Err(Failure::permanent(
                FailureKind::InvalidInput,
                "저장할 Notion integration token이 비어 있다.",
            ));
        }

        self.secrets
            .set(SecretKey::NotionIntegrationToken, &Secret::new(token))?;

        self.token_status()
    }

    /// 저장된 integration token을 지운다.
    ///
    /// **없던 것을 지우는 것은 실패가 아니다** — 부르고 난 뒤 없다는 사실이 같으면 같은
    /// 결과다 ([`SecretStore::delete`]). 지운 뒤에도 녹음 · 전사 · 노트 · 이미 만들어진 Notion
    /// 페이지는 그대로다 (INV-3): 이 함수가 만지는 것은 자격증명 저장소의 항목 하나뿐이다.
    pub fn delete_token(&self) -> Result<NotionTokenStatusPayload, Failure> {
        self.secrets.delete(SecretKey::NotionIntegrationToken)?;

        self.token_status()
    }

    /// 지금 token이 저장돼 있는가. **값을 꺼내지 않고 있는지만 본다.**
    ///
    /// 저장·삭제 뒤에 이것을 다시 읽는 이유는 하나다 — 화면이 자기가 방금 부른 command의
    /// 이름으로 결과를 짐작하지 않고, 자격증명 저장소가 실제로 어떤 상태인지 받게 하기 위해서다.
    fn token_status(&self) -> Result<NotionTokenStatusPayload, Failure> {
        Ok(NotionTokenStatusPayload {
            stored: self.secrets.get(SecretKey::NotionIntegrationToken)?.is_some(),
        })
    }

    /// 전송 한 건을 배경 스레드에서 돌린다.
    ///
    /// 스레드는 **결과를 상태 한 값에 남기는 것 말고는 아무것도 하지 않는다.** 화면에 직접
    /// 알리지 않으므로 창이 닫히거나 화면이 바뀌어도 이 스레드가 향할 곳을 잃지 않는다.
    fn spawn(&self, recording_id: String, confirmation: Confirmation, app_data_dir: AppDataDirectory) {
        let transport = Arc::clone(&self.transport);
        let secrets = Arc::clone(&self.secrets);
        let waiter = Arc::clone(&self.waiter);
        let state = Arc::clone(&self.state);

        thread::spawn(move || {
            let outcome = send_one(
                &app_data_dir,
                &recording_id,
                confirmation,
                transport,
                secrets.as_ref(),
                waiter.as_ref(),
            );

            // 여기서 처음으로 자물쇠를 잡는다. 전송이 도는 동안에는 쥐고 있지 않았다.
            //
            // 잠그지 못하는 것은 상태를 읽는 쪽이 이미 죽은 채로 남았다는 뜻이며, 그 사실은
            // `status`가 자신의 실패로 알린다 — 여기서 다시 알릴 상대가 없다.
            if let Ok(mut state) = state.lock() {
                *state = match outcome {
                    Ok(sent) => NotionSendStatus::Done {
                        recording_id,
                        // 성공했다면 페이지 식별자는 반드시 저장돼 있다 (ADR-0009 §8.4-3).
                        page_id: sent.sync.page_id.clone().unwrap_or_default(),
                        created_page: sent.created_page,
                    },
                    Err(failure) => NotionSendStatus::Failed {
                        recording_id,
                        failure,
                    },
                };
            }
        });
    }

    /// 상태 한 값을 빌린다. 이전 호출이 쥔 채 죽었다면 그 사실을 그대로 알린다.
    fn state(&self) -> Result<MutexGuard<'_, NotionSendStatus>, Failure> {
        self.state.lock().map_err(|_| {
            Failure::permanent(
                FailureKind::Storage,
                "Notion 전송 상태를 더 이상 알 수 없다. 앱을 다시 시작해야 한다.",
            )
        })
    }
}

/// 배경 스레드가 하는 일 전부 — 저장소를 열고, 설정과 자격증명을 읽고, 전송 한 건을 돌린다.
///
/// **연결을 새로 연다.** 앱이 들고 있는 연결을 쓰면 전송이 도는 내내 그 자물쇠가 잡혀 있어
/// 목록 조회 같은 다른 command가 함께 멈춘다 ([`super::transcriber`]와 같은 이유다).
///
/// ## 어디로 보낼지와 무엇으로 보낼지는 **보낼 때마다 읽는다**
///
/// 앱을 켤 때 한 번 읽어 들고 있지 않는 이유는 전사·노트와 같다 — 사용자가 설정에서 부모
/// 페이지를 바꾸거나 token을 다시 넣은 뒤 앱을 다시 시작해야 한다면, 고친 값이 반영되지 않는
/// 구간이 생긴다.
///
/// 전송의 순서와 영속화 규칙은 여기에 없다 — 그것을 아는 자리는 [`crate::sync::run`] 하나다.
fn send_one(
    app_data_dir: &AppDataDirectory,
    recording_id: &str,
    confirmation: Confirmation,
    transport: Arc<dyn HttpTransport>,
    secrets: &dyn SecretStore,
    waiter: &dyn Waiter,
) -> Result<Sent, Failure> {
    let connection = db::open_in(app_data_dir)?;

    let parent_page_id = settings::load(&connection)?
        .notion_parent_page_id
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(destination_not_chosen)?;

    // **token이 지나가는 유일한 자리다** (ADR-0009 §10.4). 값은 이 함수의 지역 변수로 살고
    // 요청의 헤더가 되는 것 말고는 아무 데도 가지 않는다.
    let token = secrets
        .get(SecretKey::NotionIntegrationToken)?
        .ok_or_else(token_not_stored)?;

    run::send(
        &connection,
        &RecordingId::new(recording_id),
        &NotionClient::new(transport),
        &Destination {
            token: &token,
            parent_page_id: &parent_page_id,
        },
        waiter,
        confirmation,
    )
}

/// 아직 어디로 보낼지 고르지 않았다 (§5-D).
///
/// **Notion 실패 다섯 종류를 쓰지 않는다.** 그 다섯은 Notion과 실제로 말한 결과이고
/// (`crate::notion::client`), 이것은 요청을 보내기도 전의 상태다. 고르지 않은 것은 오류가
/// 아니라 정상 상태이며 (INV-8), 이 실패 값은 그 상태에서 굳이 전송을 눌렀을 때의 답이다.
fn destination_not_chosen() -> Failure {
    Failure::retryable(
        FailureKind::InvalidInput,
        "Notion에서 보낼 위치를 아직 고르지 않았다. 설정에서 부모 페이지를 정해야 한다.",
    )
}

/// 아직 integration token을 저장하지 않았다.
///
/// **어떤 값도 이 실패에 실리지 않는다** (INV-7). 저장돼 있지 않다는 사실 하나만 나간다.
fn token_not_stored() -> Failure {
    Failure::retryable(
        FailureKind::InvalidInput,
        "Notion integration token이 아직 저장돼 있지 않다. 설정에서 입력해야 한다.",
    )
}

/// 이미 보내는 중인데 또 시작하라는 요청이 왔다.
///
/// 두 경우를 구분한다 — **사용자가 할 수 있는 일이 다르기 때문이다.** 같은 녹음이면 이미 하고
/// 있는 일이라 다시 눌러도 달라지지 않고, 다른 녹음이면 지금 것이 끝난 뒤에 하면 된다.
fn already_sending(running: &str, requested: &str) -> Failure {
    let failure = if running == requested {
        Failure::permanent(
            FailureKind::InvalidInput,
            "이미 이 녹음을 Notion으로 보내고 있다.",
        )
    } else {
        Failure::retryable(
            FailureKind::InvalidInput,
            "다른 녹음을 Notion으로 보내고 있다. 그것이 끝난 뒤에 시작할 수 있다.",
        )
    };

    failure.with_detail(format!("runningRecordingId={running}"))
}

/// 화면이 보낸 확인 문자열을 [`Confirmation`]으로 읽는다 (ADR-0009 §8.3 · §8.5).
///
/// **아무것도 보내지 않은 것은 아무것도 확인하지 않은 것이다.** 값이 없을 때 [`Confirmation::NewPage`]로
/// 읽으면, 화면이 그 값을 실어 보내는 것을 잊은 순간 사용자가 모르는 사이에 페이지가 하나 더
/// 만들어진다 — 이 Phase가 금지한 "조용한 중복"이 정확히 그것이다.
///
/// 모르는 값도 같은 이유로 추측하지 않는다. 확인되지 않은 요청은 [`crate::sync::run::send`]가
/// 무엇을 확인해야 하는지와 함께 거절한다.
pub(super) fn parse_confirmation(confirmation: Option<&str>) -> Result<Confirmation, Failure> {
    match confirmation.map(str::trim) {
        None | Some("") | Some("notAsked") => Ok(Confirmation::NotAsked),
        Some("newPage") => Ok(Confirmation::NewPage),
        Some(other) => Err(Failure::permanent(
            FailureKind::InvalidInput,
            "무엇을 확인했는지 알 수 없어 전송을 시작하지 않았다.",
        )
        .with_detail(format!("confirmation={other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::secret_store::testing::InMemorySecretStore;

    /// 이 테스트가 쓰는 값. **실제 자격증명이 아니다** (ADR-0009 §10.5).
    const NOT_A_REAL_TOKEN: &str = "ntn-command-test-double-value-not-a-real-credential";

    /// 실제 Notion에도 실제 자격증명 저장소에도 닿지 않는 전송자 하나.
    fn sender(secrets: Arc<InMemorySecretStore>) -> NotionSender {
        NotionSender {
            transport: Arc::new(crate::notion::testing::StubServer::ready()),
            secrets,
            waiter: Arc::new(crate::sync::pace::testing::RecordedWaits::new()),
            app_data_dir: Ok(AppDataDirectory::new(std::env::temp_dir())),
            state: Arc::new(Mutex::new(NotionSendStatus::Idle)),
        }
    }

    #[test]
    fn a_stored_token_is_answered_as_a_fact_and_never_as_a_value() {
        // INV-7 — 저장하는 command의 입력으로 한 번 지나갈 뿐, 돌아오는 것은 있다/없다뿐이다.
        let secrets = Arc::new(InMemorySecretStore::new());
        let sender = sender(Arc::clone(&secrets));

        assert!(
            !sender.token_status().expect("읽을 수 있다").stored,
            "저장한 적이 없는 것은 실패가 아니라 상태다"
        );

        let saved = sender.save_token(NOT_A_REAL_TOKEN).expect("저장할 수 있다");
        assert!(saved.stored);
        assert_eq!(
            secrets
                .stored(SecretKey::NotionIntegrationToken)
                .expect("값이 있어야 한다")
                .expose(),
            NOT_A_REAL_TOKEN,
            "지나간 값이 자격증명 저장소에 그대로 들어간다"
        );

        // 돌아간 값 어디에도 token이 없다. 담을 자리 자체가 없다.
        let rendered = format!("{saved:?}");
        assert!(!rendered.contains(NOT_A_REAL_TOKEN), "{rendered}");

        assert!(
            !sender.delete_token().expect("지울 수 있다").stored,
            "지운 뒤에는 없다"
        );
        assert!(secrets.is_empty());
    }

    #[test]
    fn a_blank_token_is_refused_without_echoing_what_was_sent() {
        let sender = sender(Arc::new(InMemorySecretStore::new()));

        for blank in ["", "   ", "\n\t"] {
            let failure = sender.save_token(blank).expect_err("빈 값은 저장하지 않는다");

            assert_eq!(failure.kind, FailureKind::InvalidInput);
            assert!(!failure.retryable, "같은 값을 다시 보내도 같다");
            assert!(failure.source_data_safe);
            assert_eq!(failure.detail, None, "입력값이 실패에 실리지 않는다 (INV-7)");
        }
    }

    #[test]
    fn a_pasted_token_keeps_its_value_but_not_the_whitespace_around_it() {
        let secrets = Arc::new(InMemorySecretStore::new());
        let sender = sender(Arc::clone(&secrets));

        sender
            .save_token(&format!("  {NOT_A_REAL_TOKEN}\n"))
            .expect("저장할 수 있다");

        assert_eq!(
            secrets
                .stored(SecretKey::NotionIntegrationToken)
                .expect("값이 있어야 한다")
                .expose(),
            NOT_A_REAL_TOKEN
        );
    }

    #[test]
    fn nothing_confirmed_is_the_answer_when_the_screen_says_nothing() {
        // 값이 없을 때 '새 페이지를 만들어도 된다'로 읽으면 조용한 중복이 만들어진다 (§8.3).
        for nothing in [None, Some(""), Some("   "), Some("notAsked")] {
            assert_eq!(
                parse_confirmation(nothing).expect("읽을 수 있다"),
                Confirmation::NotAsked
            );
        }
        assert_eq!(
            parse_confirmation(Some(" newPage ")).expect("읽을 수 있다"),
            Confirmation::NewPage
        );
    }

    #[test]
    fn an_unknown_confirmation_is_refused_instead_of_being_guessed() {
        let failure = parse_confirmation(Some("yes")).expect_err("모르는 값은 읽지 않는다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(!failure.retryable);
        assert!(failure.source_data_safe, "아무것도 건드리지 않았다");
        assert_eq!(failure.detail.as_deref(), Some("confirmation=yes"));
    }

    #[test]
    fn a_directory_that_could_not_be_resolved_is_reported_instead_of_panicking() {
        // 앱 시작 시점의 실패가 여기까지 값으로 실려 온다 (§13). 그 상태에서도 앱은 떠 있다.
        let sender = NotionSender {
            transport: Arc::new(crate::notion::testing::StubServer::ready()),
            secrets: Arc::new(crate::platform::secret_store::testing::InMemorySecretStore::new()),
            waiter: Arc::new(crate::sync::pace::testing::RecordedWaits::new()),
            app_data_dir: Err(Failure::retryable(
                FailureKind::Storage,
                "앱 데이터 디렉터리 경로를 결정하지 못했다",
            )),
            state: Arc::new(Mutex::new(NotionSendStatus::Idle)),
        };

        let failure = sender
            .start("rec-1", Confirmation::NotAsked)
            .expect_err("자리를 모르면 보낼 수 없다");

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(failure.source_data_safe, "아무것도 건드리지 않았다 (INV-3)");
        assert_eq!(
            sender.status().expect("상태는 읽을 수 있다"),
            NotionSendStatus::Idle,
            "시작하지 못한 요청이 상태를 옮기지 않는다"
        );
    }

    #[test]
    fn a_refused_start_says_which_one_is_running() {
        let failure = already_sending("rec-a", "rec-b");
        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(failure.retryable, "지금 것이 끝난 뒤에는 시작할 수 있다");
        assert_eq!(failure.detail.as_deref(), Some("runningRecordingId=rec-a"));

        let same = already_sending("rec-a", "rec-a");
        assert!(!same.retryable, "이미 하고 있는 일이다");
    }

    #[test]
    fn a_missing_setting_is_a_normal_state_not_one_of_the_five_notion_failures() {
        // INV-8: 고르지 않은 것은 오류가 아니다. 그 다섯은 adapter가 만드는 것이며, 이
        // 경계에서 그것을 흉내 내지 않는다 (`NOTION_FAILURE_KINDS`).
        for failure in [destination_not_chosen(), token_not_stored()] {
            assert_eq!(failure.kind, FailureKind::InvalidInput);
            assert!(failure.retryable, "설정을 채우면 진행할 수 있다");
            assert!(failure.source_data_safe);
            assert!(
                !crate::notion::NOTION_FAILURE_KINDS.contains(&failure.kind),
                "요청을 보내기도 전의 상태가 Notion 실패로 보고됐다"
            );
            assert_eq!(failure.detail, None, "설정값도 자격증명도 실리지 않는다");
        }
    }

}
