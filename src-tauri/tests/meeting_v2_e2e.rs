//! V2 Meeting의 end-to-end 검증 — **실제 provider 호출 없이** ~25KB fixture가
//! `parse_note → encode_content → decode_content → Markdown render → 앱 noteSections`
//! 를 **모두** 통과하는 것을 확인한다.
//!
//! ## AC-8-1: 25KB V2 fixture가 parse → encode → decode → Markdown render를 통과한다
//! ## AC-8-2: Markdown 출력에 🔑·🔁·✅·🔍 섹션이 기준 형식 순서로 존재한다
//! ## AC-8-3: 크기 상한(MAX_ITEMS · MAX_ITEM_CHARS · OUTPUT_RESERVE_TOKENS)이 이 fixture를 거부하지 않는다

use molt_note_lib::ai::note::{
    decode_content, encode_content, parse_note, ActionItemV2, MeetingNoteV2, PreviousActionItems,
    StructuredNote, Topic, TopicGroup, MAX_ITEMS, MAX_ITEM_CHARS, MAX_OVERVIEW_CHARS,
};
use molt_note_lib::ai::prompt::{CHARS_PER_ESTIMATED_TOKEN, OUTPUT_RESERVE_TOKENS};
use molt_note_lib::domain::{NoteType, ProcessingStatus, Recording, RecordingId, Transcript, TranscriptId, TranscriptSegment};
use molt_note_lib::export::markdown::{render, ExportDocument, MEETING_V2_SECTIONS};

// ── fixture ────────────────────────────────────────────────────────────────────

/// 2시간 분량 회의를 모사하는 ~25KB V2 Meeting fixture.
///
/// - keySummary 7개
/// - previousActionItems 각 분류 2~3개
/// - topics 6개 (각 2~3 groups)
/// - actionItems 16개 (두 기간)
/// - needsConfirmation 6개
fn large_v2_fixture() -> MeetingNoteV2 {
    MeetingNoteV2 {
        meeting_title: "2026년 4분기 사업 전략 및 제품 로드맵 정기 검토 회의".to_owned(),
        meeting_kind: "정기 위클리 (사업 계획 확정 및 Q4 로드맵 중간 점검, 일정 조율)".to_owned(),
        attendees: vec![
            "김철수 (PM)".to_owned(),
            "이영희 (디자인 리드)".to_owned(),
            "박지민 (백엔드 개발)".to_owned(),
            "최민수 (프론트엔드 개발)".to_owned(),
            "정하늘 (QA 매니저)".to_owned(),
        ],
        reliability_notice: "📌 이 문서는 1시간 53분 분량의 녹음을 자동 전사(STT) 후 AI가 정리한 회의록입니다. 이름과 전문 용어는 음성 인식 기준이므로 틀릴 수 있습니다. 하단의 🔍 확인 필요 항목을 반드시 검토해 주세요. 원문 전사는 별도 문서에 있습니다.".to_owned(),
        key_summary: vec![
            "**9월 출시 확정**: Q4 첫 번째 마일스톤인 MVP를 9월 말까지 출시하기로 최종 합의했습니다. 예상 비용은 5,000만원이며, 추가 인력 투입 없이 현 팀으로 진행합니다. 디자인 QA와 E2E 자동화를 포함한 전체 릴리스 체크리스트를 정하늘 매니저가 관리합니다.".to_owned(),
            "**디자인 시스템 v2 전환**: 신규 디자인 시스템을 11월 1일까지 전면 적용하기로 했으며, 이영희 리드가 마이그레이션 가이드를 다음 주까지 배포합니다. 디자인 토큰 표준화, 컴포넌트 라이브러리 업데이트, Storybook 문서화가 포함됩니다. 외주 디자이너 1명을 10월 한 달간 투입하기로 확정했습니다.".to_owned(),
            "**백엔드 API 리팩토링 완료**: 박지민 개발자가 진행한 REST → GraphQL 전환이 완료되어 스테이징 배포가 완료된 상태입니다. 성능 테스트 결과 응답 시간이 30% 개선되었고, 번들 사이즈도 15% 감소했습니다. 레거시 REST 엔드포인트는 11월 말까지 유지 후 폐기하며, 외부 연동 파트너 3곳에 마이그레이션 가이드를 이미 공유했습니다.".to_owned(),
            "**QA 자동화 커버리지 확대**: 현재 72%인 자동화 테스트 커버리지를 12월 말까지 90%로 높이기로 했습니다. 정하늘 매니저가 테스트 계획서를 이번 주 금요일까지 공유합니다. 특히 결제 플로우와 사용자 가입 플로우의 E2E 자동화를 우선 진행하고, Playwright 도입도 검토 중입니다. 회귀 테스트 실행 시간은 45분에서 20분으로 단축할 계획입니다.".to_owned(),
            "**고객 피드백 반영 우선순위**: 지난 달 수집한 VOC 153건 중 상위 20건을 11월 스프린트에 반영하기로 했으며, 우선순위 기준은 고객 영향도와 구현 난이도 매트릭스를 따릅니다. 가장 많이 요청된 기능은 다크 모드(23건), 대시보드 커스터마이징(18건), 모바일 앱 오프라인 모드(15건) 순이었습니다. 심각도 1 버그 3건은 이번 주 핫픽스로 처리합니다.".to_owned(),
            "**인프라 이관 일정 조율**: AWS에서 GCP로의 이관을 내년 1분기로 연기하기로 했습니다. 현재 인프라의 안정성이 충분하고, Q4에는 제품 개발에 집중하는 것이 맞다는 판단입니다. 현재 AWS 비용이 월 850만원이며, GCP 이관 시 예상 비용은 월 620만원으로 연간 2,760만원 절감 효과가 있으나, 이관 자체에 3개월과 추가 인력이 필요합니다.".to_owned(),
            "**채용 계획**: 시니어 백엔드 개발자 1명을 12월까지 채용하기로 했으며, JD는 김철수 PM이 이번 주 중으로 HR에 전달합니다. 필수 요건은 GraphQL + TypeScript 3년 이상이며, GCP 경험은 우대 사항입니다. 면접 프로세스는 코딩 테스트 → 기술 면접 → 컬처 핏 3단계로 진행합니다.".to_owned(),
        ],
        previous_action_items: PreviousActionItems {
            done: vec![
                "RFP 초안 완성 — 마케팅팀 검토 대기 중, 내부 법무 검토도 병행 진행하여 계약 조건 확인 완료 (김철수)".to_owned(),
                "스테이징 환경 API 배포 완료 — 성능 테스트 결과 P95 응답 시간 1.2초로 만족스러움, Grafana 대시보드에 모니터링 알림 설정 완료 (박지민)".to_owned(),
                "QA 테스트 시나리오 문서 v1 작성 완료 — 팀 리뷰에서 결제 시나리오 보강 의견이 있어 v1.1로 업데이트 예정 (정하늘)".to_owned(),
            ],
            changed: vec![
                "일정: 디자인 시스템 전환 11월 → 12월로 연기 — 디자인 토큰 표준화에 추가 2주가 필요하고, 외주 디자이너 투입 후 진행하는 것이 효율적이라는 판단 (이영희)".to_owned(),
                "범위: 모바일 앱 리뉴얼을 v2에서 빼고 v3로 이동 — Q4에는 웹 앱 MVP 출시에 리소스를 집중하고, 모바일은 다음 분기에 별도 스프린트로 진행 (김철수)".to_owned(),
                "기술 스택: 모니터링 도구를 자체 구축에서 SaaS(Datadog 또는 New Relic)로 전환 검토 — 구축 비용 대비 운영 효율이 더 높다는 분석 결과 (박지민)".to_owned(),
            ],
            in_progress: vec![
                "프론트엔드 성능 최적화 작업 중 — Lighthouse 점수 60→85 목표, 현재 75 (최민수)".to_owned(),
                "사용자 인터뷰 진행 중 — 20명 중 12명 완료, 나머지 8명 다음 주까지 완료 예정 (김철수)".to_owned(),
                "CI/CD 파이프라인 개선 — 빌드 시간 단축 작업 50% 진행 (박지민)".to_owned(),
            ],
            no_progress: vec![
                "서버 이관 사전 조사 — 담당자(박지민)가 API 리팩토링에 집중하느라 착수하지 못함, 다음 주부터 시작 예정이며 GCP 콘솔 접근 권한 먼저 요청해 둠".to_owned(),
                "접근성 감사 — 외부 전문 업체 3곳에 견적 요청했으나 2곳이 일정이 맞지 않아 지연 중, 남은 1곳과 11월 중 계약 예정 (정하늘)".to_owned(),
            ],
            not_mentioned: vec![
                "테스트 환경 분리 구축 계획 — 스테이징/QA/개발 환경을 독립적으로 운영하는 작업, 인프라 이관과 연계하여 Q1에 진행 예정".to_owned(),
                "기술 블로그 작성 — 이번 분기 KPI 항목으로 팀원당 1편 작성이 목표이나, 제품 개발 일정 우선으로 아직 주제 선정도 시작하지 못함".to_owned(),
            ],
        },
        topics: vec![
            Topic {
                title: "Q4 제품 로드맵 점검 및 마일스톤 확인".to_owned(),
                groups: vec![
                    TopicGroup {
                        heading: "일정 확정".to_owned(),
                        items: vec![
                            "MVP 출시를 9월 30일로 확정했다. 지난 주 논의에서 10월 중순으로 밀릴 수 있다는 우려가 있었으나, 백엔드 API 리팩토링이 예정보다 일주일 빨리 끝나면서 여유가 생겼다. 스프린트 계획을 재조정하여 남은 6주 동안 2주 단위 3개 스프린트를 진행한다.".to_owned(),
                            "베타 테스트는 10월 15일부터 2주간 진행하며, 초대 대상은 기존 고객 중 활성 사용자 100명으로 한다. 정하늘 매니저가 선정 기준을 이번 주 금요일까지 공유한다. 선정 기준은 MAU(월간 활성 사용), 사용 기능 다양성, 과거 피드백 이력을 기반으로 한다.".to_owned(),
                            "정식 출시는 11월 1일을 목표로 하되, 베타 피드백에 따라 최대 2주 연장할 수 있다. 연장 판단 기준은 심각도 1의 버그가 5건 이상일 때이다. 출시 이후 2주간 안정화 기간을 두고 핫픽스 전담 팀을 운영한다.".to_owned(),
                            "마케팅팀과의 협업 일정도 확정했다. 출시 2주 전부터 사전 홍보를 시작하고, 출시 당일에는 블로그 포스트, 소셜 미디어 발표, 기존 고객 이메일 안내를 동시에 진행한다. 프레스 킷은 김철수 PM이 마케팅팀과 조율하여 준비한다.".to_owned(),
                        ],
                    },
                    TopicGroup {
                        heading: "리소스 배분".to_owned(),
                        items: vec![
                            "프론트엔드 최민수 개발자가 디자인 시스템 전환과 성능 최적화를 병행하기 어렵다는 의견을 냈다. 김철수 PM이 디자인 시스템 전환의 우선순위를 낮추고 성능 최적화에 집중하도록 조율했다. 성능 최적화가 사용자 경험에 직접적인 영향을 주므로 출시 전에 반드시 완료해야 한다는 데 합의했다.".to_owned(),
                            "외주 디자이너 1명을 10월 한 달간 투입하여 디자인 시스템 마이그레이션 가이드 작성을 지원하기로 했다. 이영희 리드가 외주 업체 3곳의 포트폴리오를 검토 중이며, 이번 주 내로 선정한다. 예산은 Q4 운영비에서 충당한다.".to_owned(),
                            "QA팀의 자동화 테스트 작성과 수동 테스트 병행이 부담이라는 의견이 있었다. 11월부터 인턴 1명을 추가 배치하여 수동 테스트를 지원하기로 했다. 인턴 채용은 정하늘 매니저가 HR과 조율한다.".to_owned(),
                        ],
                    },
                    TopicGroup {
                        heading: "리스크 관리".to_owned(),
                        items: vec![
                            "10월 연휴(추석)로 인해 실질 작업일이 예상보다 3일 적다. 이를 감안하여 스프린트 2의 스토리 포인트를 10% 줄이기로 했다. 연휴 전에 코드 프리즈를 하고, 연휴 후에는 통합 테스트에 집중한다.".to_owned(),
                            "외부 API 연동 파트너 중 1곳이 자체 API 버전 업그레이드를 11월에 예고했다. 호환성 문제가 발생할 수 있으므로, 박지민 개발자가 사전에 변경 사항을 확인하고 대응 계획을 세운다.".to_owned(),
                        ],
                    },
                ],
            },
            Topic {
                title: "디자인 시스템 v2 전환 현황 및 타이포그래피 표준화".to_owned(),
                groups: vec![
                    TopicGroup {
                        heading: "토큰 표준화 현황".to_owned(),
                        items: vec![
                            "디자인 토큰 표준화가 예상보다 복잡하여 2주 추가가 필요하다. 색상 토큰은 완료되었으나 타이포그래피와 스페이싱 토큰이 남아 있다. 색상 토큰은 라이트/다크 모드 각각 48개로, 시맨틱 네이밍 규칙을 적용했다. 타이포그래피는 본문/제목/캡션/라벨 4단계 × 4가지 굵기로 설계 중이며, 반응형 크기 조절 로직도 포함된다.".to_owned(),
                            "컴포넌트 라이브러리의 Storybook 문서화가 60% 완료되었다. 나머지 40%는 외주 디자이너 투입 후 2주 내 완료 예정이다. 현재 문서화된 컴포넌트는 Button, Input, Card, Modal, Toast, Navigation 등 24개이며, 미완료 컴포넌트는 Table, Chart, Calendar, DatePicker 등 16개이다.".to_owned(),
                            "아이콘 시스템도 재정비하기로 했다. 기존 SVG 아이콘 120개를 통합 아이콘 라이브러리로 옮기고, 크기(16/20/24/32px)와 색상 변수를 표준화한다. 이영희 리드가 아이콘 정리 목록을 이번 주 내로 공유한다.".to_owned(),
                        ],
                    },
                    TopicGroup {
                        heading: "마이그레이션 계획".to_owned(),
                        items: vec![
                            "기존 컴포넌트에서 신규 디자인 토큰으로의 전환은 화면별로 진행한다. 우선순위는 사용 빈도 순이며, 대시보드 → 설정 → 상세 화면 → 보고서 → 관리자 화면 순서다. 각 화면의 전환에는 디자인 검수와 QA를 포함하여 평균 3일이 소요될 것으로 예상한다.".to_owned(),
                            "마이그레이션 중 기존 디자인과 신규 디자인이 공존하는 기간이 최대 3주이며, 그 동안의 시각적 불일치는 감수하기로 했다. 다만, 고객 문의가 예상되므로 공지사항을 미리 준비한다. 공지 문안은 김철수 PM이 작성한다.".to_owned(),
                            "이영희 리드가 마이그레이션 가이드를 작성하여 다음 주 월요일까지 Confluence에 공유한다. 가이드에는 토큰 매핑 표, 화면별 전환 순서, 공존 기간 QA 체크리스트, 롤백 절차가 포함된다.".to_owned(),
                        ],
                    },
                    TopicGroup {
                        heading: "접근성 고려사항".to_owned(),
                        items: vec![
                            "신규 디자인 시스템이 WCAG 2.1 AA 수준을 충족하는지 확인이 필요하다. 특히 색상 대비, 키보드 내비게이션, 스크린 리더 호환성을 집중 점검한다. 정하늘 매니저가 접근성 체크리스트를 마이그레이션 가이드에 추가한다.".to_owned(),
                            "다크 모드 적용 시 텍스트 가독성과 색상 대비 비율이 달라지므로, 다크 모드 전용 색상 토큰을 별도로 정의한다. 이영희 리드가 담당하며, 자동 대비 검사 도구(Axe 또는 Lighthouse)를 CI에 통합하는 것도 검토한다.".to_owned(),
                        ],
                    },
                ],
            },
            Topic {
                title: "백엔드 API 리팩토링 결과 리뷰 및 성능 분석".to_owned(),
                groups: vec![
                    TopicGroup {
                        heading: "성과 지표".to_owned(),
                        items: vec![
                            "REST → GraphQL 전환이 계획대로 완료되었다. 스테이징 환경 배포 후 2주간의 안정성 모니터링에서 에러율 0.02%를 기록했다. 모니터링 대시보드(Grafana)에서 P95 응답 시간, 에러율, 스루풋을 실시간으로 추적하고 있으며, 알림 임계값도 설정 완료했다.".to_owned(),
                            "응답 시간이 평균 30% 개선되었고, 특히 대시보드 초기 로드가 2.1초에서 1.4초로 줄었다. 번들 사이즈도 15% 감소했다. GraphQL의 필드 선택 기능 덕분에 불필요한 데이터 전송이 줄었고, 네트워크 바이트 전송량이 40% 감소한 것으로 측정되었다.".to_owned(),
                            "DB 쿼리 수도 개선되었다. 기존 REST에서 대시보드 한 화면을 그리려면 7개 엔드포인트를 호출해야 했으나, GraphQL에서는 단일 쿼리로 같은 데이터를 가져온다. 서버 측 쿼리 수가 화면당 평균 12개에서 4개로 줄었다.".to_owned(),
                        ],
                    },
                    TopicGroup {
                        heading: "남은 작업과 폐기 일정".to_owned(),
                        items: vec![
                            "레거시 REST 엔드포인트는 11월 말까지 유지한 후 폐기한다. 외부 연동 파트너 3곳에 마이그레이션 가이드를 이미 공유했다. 파트너별 전환 완료 예정일은 A사 11/10, B사 11/15, C사 11/20이며, 각 파트너의 테스트 환경에서 검증 완료 후 본번 전환한다.".to_owned(),
                            "GraphQL subscription을 활용한 실시간 알림 기능은 11월 스프린트에서 구현할 예정이다. WebSocket과 SSE 중 WebSocket을 선택했으며, 재연결 전략(exponential backoff + jitter)과 메시지 스키마를 먼저 설계한다. 박지민 개발자가 설계 문서를 12월 1일까지 공유한다.".to_owned(),
                            "API 게이트웨이의 레이트 리미팅 정책을 GraphQL에 맞게 조정해야 한다. REST에서는 엔드포인트별로 설정했으나, GraphQL은 쿼리 복잡도 기반으로 바꿔야 한다. 박지민 개발자가 조사하여 11월 중 적용한다.".to_owned(),
                        ],
                    },
                    TopicGroup {
                        heading: "우려 사항 및 기술 부채".to_owned(),
                        items: vec![
                            "N+1 쿼리 문제가 일부 resolver에서 발견되었다. DataLoader 패턴 적용이 필요하며, 박지민 개발자가 이번 주 내로 수정할 예정이다. 영향 받는 resolver는 User→Posts, Post→Comments, Comment→Author 3곳이다.".to_owned(),
                            "GraphQL 스키마 버전 관리 전략이 아직 정해지지 않았다. deprecated 필드의 유지 기간, 클라이언트 버전별 스키마 호환성 정책을 11월 초까지 확정한다. 김철수 PM이 논의를 주도한다.".to_owned(),
                        ],
                    },
                ],
            },
            Topic {
                title: "QA 자동화 전략 및 커버리지 목표 수립".to_owned(),
                groups: vec![
                    TopicGroup {
                        heading: "현황 분석".to_owned(),
                        items: vec![
                            "현재 자동화 테스트 커버리지는 72%이며, 주로 단위 테스트에 집중되어 있다. E2E 테스트는 15개 시나리오만 자동화되어 있고, 나머지 43개는 수동이다. 수동 테스트 1회 실행에 QA 인력 2명 × 3일이 소요되어 릴리스 주기를 늦추는 주요 병목이다.".to_owned(),
                            "Playwright 도입을 검토 중이며, Cypress에서의 마이그레이션 비용을 정하늘 매니저가 평가 중이다. Playwright의 장점은 크로스 브라우저 테스트(Chrome, Firefox, Safari)를 단일 API로 지원하고, 자동 대기(auto-wait) 기능으로 테스트 안정성이 높다는 점이다. 단점은 팀의 학습 곡선과 기존 테스트 코드 재작성 비용이다.".to_owned(),
                            "테스트 데이터 관리도 이슈다. 현재 테스트마다 수동으로 데이터를 셋업하고 있어 테스트 간 의존성이 생기고, 병렬 실행이 어렵다. 팩토리 패턴과 트랜잭션 롤백을 도입하여 각 테스트가 독립적으로 실행되도록 개선할 계획이다.".to_owned(),
                        ],
                    },
                    TopicGroup {
                        heading: "목표 및 일정".to_owned(),
                        items: vec![
                            "12월 말까지 전체 커버리지 90% 달성. 특히 결제 플로우와 사용자 가입 플로우의 E2E 자동화를 우선 진행한다. 결제 플로우는 카드 결제, 계좌 이체, 쿠폰 적용 3가지 시나리오를 포함하며, 가입 플로우는 이메일, 소셜(구글/카카오/애플) 4가지 경로를 포함한다.".to_owned(),
                            "회귀 테스트 실행 시간을 현재 45분에서 20분 이내로 단축한다. 병렬 실행과 테스트 분할 전략을 적용할 예정이다. CI 파이프라인에서 테스트를 4개 샤드로 나누어 병렬 실행하고, 느린 테스트는 별도 야간 스위트로 분리한다.".to_owned(),
                            "시각적 회귀 테스트(Visual Regression Testing)도 도입한다. Percy 또는 Chromatic 중 하나를 선택하여 디자인 시스템 전환 과정에서 의도치 않은 UI 변경을 자동으로 감지한다. 정하늘 매니저가 두 도구를 비교 평가하여 11월 중 결정한다.".to_owned(),
                        ],
                    },
                ],
            },
            Topic {
                title: "고객 피드백(VOC) 분석 및 우선순위 결정".to_owned(),
                groups: vec![
                    TopicGroup {
                        heading: "분석 결과 요약".to_owned(),
                        items: vec![
                            "지난 달 수집된 VOC 153건 중 기능 요청이 67건(44%), 버그 리포트가 42건(27%), UX 개선 요청이 31건(20%), 기타 13건(9%)이었다. 수집 채널별로는 인앱 피드백 82건, 고객센터 전화 35건, 이메일 21건, SNS 멘션 15건이었다.".to_owned(),
                            "가장 많이 요청된 기능은 다크 모드(23건), 대시보드 커스터마이징(18건), 모바일 앱 오프라인 모드(15건) 순이었다. 다크 모드는 3개월 연속 1위를 차지하고 있어 즉각적인 대응이 필요하다. 대시보드 커스터마이징은 위젯 재배치와 데이터 소스 선택 기능을 포함한다.".to_owned(),
                            "버그 리포트 42건 중 심각도 1(서비스 불가)이 3건, 심각도 2(기능 제한)가 12건, 심각도 3(불편)이 27건이다. 심각도 1 버그는 결제 오류(#1234), 로그인 실패(#1235), 데이터 유실(#1236)이며, 모두 이번 주 핫픽스로 처리한다.".to_owned(),
                        ],
                    },
                    TopicGroup {
                        heading: "우선순위 선정 기준".to_owned(),
                        items: vec![
                            "고객 영향도(1~5)와 구현 난이도(1~5) 매트릭스를 기반으로 상위 20건을 선정했다. 영향도 5 × 난이도 1인 항목(예: 로딩 속도 개선)이 최우선이고, 영향도 5 × 난이도 5인 항목(예: 오프라인 모드)은 장기 로드맵으로 분류했다.".to_owned(),
                            "심각도 1 버그 3건은 이번 주 핫픽스로 처리하고, 나머지는 11월 스프린트 백로그에 추가한다. 핫픽스는 별도 릴리스 브랜치에서 진행하며, 프로덕션 배포 후 1시간 모니터링을 필수로 한다.".to_owned(),
                            "김철수 PM이 선정된 20건의 상세 요구사항을 정리하여 목요일까지 Jira에 등록한다. 각 스토리에는 사용자 시나리오, 수용 기준(Acceptance Criteria), 영향 범위 분석, 예상 작업일을 포함한다.".to_owned(),
                        ],
                    },
                ],
            },
            Topic {
                title: "인프라 및 DevOps 현안 논의".to_owned(),
                groups: vec![
                    TopicGroup {
                        heading: "클라우드 이관 결정".to_owned(),
                        items: vec![
                            "AWS → GCP 이관을 내년 1분기로 연기한다. Q4에는 제품 개발에 리소스를 집중하고, 이관은 안정기에 진행하는 것이 리스크가 적다는 판단이다. 이관 시에는 데이터베이스(RDS→CloudSQL), 스토리지(S3→GCS), 컴퓨트(EC2/ECS→GKE), CDN(CloudFront→Cloud CDN) 순서로 진행할 계획이다.".to_owned(),
                            "현재 AWS 비용이 월 850만원이며, GCP 이관 시 예상 비용은 월 620만원이다. 연간 2,760만원 절감 효과가 있으나, 이관 자체에 3개월과 추가 인력이 필요하다. 이관 기간 동안 두 클라우드를 동시에 운영해야 하므로 일시적으로 비용이 증가한다(예상 월 1,200만원).".to_owned(),
                            "이관 전까지 AWS 비용 최적화를 먼저 진행한다. 미사용 리소스 정리, 예약 인스턴스 전환, 스토리지 계층 최적화로 월 100만원 절감이 가능할 것으로 보인다. 박지민 개발자가 비용 분석 보고서를 11월 중 작성한다.".to_owned(),
                        ],
                    },
                    TopicGroup {
                        heading: "CI/CD 파이프라인 개선".to_owned(),
                        items: vec![
                            "빌드 시간이 현재 평균 12분이며, 캐싱 전략 개선으로 8분으로 단축할 수 있을 것으로 보인다. Docker 레이어 캐싱, npm ci 캐싱, 빌드 아티팩트 재사용이 주요 개선 포인트다. 박지민 개발자가 이번 주 내로 적용한다.".to_owned(),
                            "스테이징 환경과 프로덕션 환경의 설정 차이로 인한 배포 실패가 이번 달 2건 발생했다. 환경 설정을 코드화(IaC)하여 일치시키는 작업을 다음 스프린트에 포함한다. Terraform 또는 Pulumi 중 선택하며, 박지민 개발자가 두 도구를 비교하여 11월 중 결정한다.".to_owned(),
                            "배포 파이프라인에 자동 롤백 기능을 추가한다. 배포 후 5분 이내에 에러율이 임계값(1%)을 넘으면 자동으로 이전 버전으로 롤백하는 기능이다. 현재는 수동 롤백만 가능하여 장애 대응 시간이 길다.".to_owned(),
                        ],
                    },
                    TopicGroup {
                        heading: "모니터링 및 알림".to_owned(),
                        items: vec![
                            "현재 Grafana + Prometheus 기반 모니터링을 사용 중이다. 알림 채널은 Slack과 PagerDuty를 병행하며, 심각도에 따라 대응 수준을 차등화한다. P1은 PagerDuty(즉시 호출), P2는 Slack + 이메일, P3는 Slack만.".to_owned(),
                            "APM(Application Performance Monitoring) 도입을 검토한다. Datadog, New Relic, Elastic APM 중 하나를 선택하여 트레이싱과 분산 로그를 통합 관리한다. 특히 GraphQL 전환 후 쿼리별 성능 추적이 중요해졌다.".to_owned(),
                        ],
                    },
                ],
            },
        ],
        action_items: vec![
            ActionItemV2 {
                task: "베타 테스트 대상자 선정 기준 문서 작성 및 공유".to_owned(),
                detail: "기존 고객 중 활성 사용자 100명 선정 기준 — MAU, 사용 기능 다양성, 피드백 이력 기반".to_owned(),
                owner: "정하늘".to_owned(),
                due: "11/15 (금)".to_owned(),
                period: "이번 주 (~11/20 금)".to_owned(),
            },
            ActionItemV2 {
                task: "디자인 시스템 마이그레이션 가이드 작성".to_owned(),
                detail: "디자인 토큰 매핑 표 + 화면별 전환 순서 + 공존 기간 QA 체크리스트 포함".to_owned(),
                owner: "이영희".to_owned(),
                due: "11/18 (월)".to_owned(),
                period: "이번 주 (~11/20 금)".to_owned(),
            },
            ActionItemV2 {
                task: "N+1 쿼리 문제 수정 (DataLoader 패턴 적용)".to_owned(),
                detail: "스테이징에서 발견된 GraphQL resolver 3곳의 N+1 쿼리를 DataLoader로 해결".to_owned(),
                owner: "박지민".to_owned(),
                due: "11/14 (목)".to_owned(),
                period: "이번 주 (~11/20 금)".to_owned(),
            },
            ActionItemV2 {
                task: "프론트엔드 Lighthouse 성능 점수 85점 달성".to_owned(),
                detail: "현재 75점 → 이미지 최적화, 코드 스플리팅, 번들 사이즈 축소로 85점 목표".to_owned(),
                owner: "최민수".to_owned(),
                due: "11/20 (금)".to_owned(),
                period: "이번 주 (~11/20 금)".to_owned(),
            },
            ActionItemV2 {
                task: "심각도 1 버그 3건 핫픽스 배포".to_owned(),
                detail: "결제 오류(#1234), 로그인 실패(#1235), 데이터 유실(#1236) — 프로덕션 직접 배포".to_owned(),
                owner: "박지민".to_owned(),
                due: "11/13 (수)".to_owned(),
                period: "이번 주 (~11/20 금)".to_owned(),
            },
            ActionItemV2 {
                task: "VOC 상위 20건 상세 요구사항 Jira 등록".to_owned(),
                detail: "고객 영향도·구현 난이도 매트릭스 기반 선정 결과를 스토리 단위로 분할".to_owned(),
                owner: "김철수".to_owned(),
                due: "11/14 (목)".to_owned(),
                period: "이번 주 (~11/20 금)".to_owned(),
            },
            ActionItemV2 {
                task: "CI/CD 빌드 캐싱 전략 개선 (12분→8분)".to_owned(),
                detail: "Docker 레이어 캐싱 + npm ci 캐싱 + 빌드 아티팩트 재사용".to_owned(),
                owner: "박지민".to_owned(),
                due: "11/20 (금)".to_owned(),
                period: "이번 주 (~11/20 금)".to_owned(),
            },
            ActionItemV2 {
                task: "시니어 백엔드 개발자 JD 작성 및 HR 전달".to_owned(),
                detail: "필수: GraphQL + TypeScript 3년+, 우대: GCP 경험".to_owned(),
                owner: "김철수".to_owned(),
                due: "11/15 (금)".to_owned(),
                period: "이번 주 (~11/20 금)".to_owned(),
            },
            ActionItemV2 {
                task: "사용자 인터뷰 나머지 8명 완료".to_owned(),
                detail: "20명 중 12명 완료, 나머지 스케줄 확정 및 진행".to_owned(),
                owner: "김철수".to_owned(),
                due: "11/27 (수)".to_owned(),
                period: "다음 주 (11/24~11/28)".to_owned(),
            },
            ActionItemV2 {
                task: "Playwright 도입 vs Cypress 유지 검토 보고서".to_owned(),
                detail: "마이그레이션 비용, 학습 곡선, 커뮤니티 지원, CI 통합 비교 분석".to_owned(),
                owner: "정하늘".to_owned(),
                due: "11/25 (월)".to_owned(),
                period: "다음 주 (11/24~11/28)".to_owned(),
            },
            ActionItemV2 {
                task: "외주 디자이너 투입 확정 및 킥오프".to_owned(),
                detail: "디자인 시스템 Storybook 문서화 나머지 40% 완료 목표".to_owned(),
                owner: "이영희".to_owned(),
                due: "11/24 (월)".to_owned(),
                period: "다음 주 (11/24~11/28)".to_owned(),
            },
            ActionItemV2 {
                task: "레거시 REST 엔드포인트 폐기 타임라인 파트너 공지".to_owned(),
                detail: "외부 연동 파트너 3곳에 11월 말 폐기 일정 재안내 (마이그레이션 가이드 포함)".to_owned(),
                owner: "박지민".to_owned(),
                due: "11/25 (월)".to_owned(),
                period: "다음 주 (11/24~11/28)".to_owned(),
            },
            ActionItemV2 {
                task: "QA 자동화 테스트 계획서 작성 및 공유".to_owned(),
                detail: "12월 말 90% 커버리지 로드맵 — E2E 우선 시나리오 + 병렬 실행 전략".to_owned(),
                owner: "정하늘".to_owned(),
                due: "11/15 (금)".to_owned(),
                period: "이번 주 (~11/20 금)".to_owned(),
            },
            ActionItemV2 {
                task: "환경 설정 코드화(IaC) 작업 스프린트 포함 요청".to_owned(),
                detail: "스테이징/프로덕션 설정 불일치 재발 방지 — Terraform 또는 Pulumi".to_owned(),
                owner: "박지민".to_owned(),
                due: "11/28 (금)".to_owned(),
                period: "다음 주 (11/24~11/28)".to_owned(),
            },
            ActionItemV2 {
                task: "다크 모드 디자인 시안 초안 작성".to_owned(),
                detail: "VOC 최다 요청 기능 — 디자인 시스템 v2 토큰과 연동하여 설계".to_owned(),
                owner: "이영희".to_owned(),
                due: "12/01 (월)".to_owned(),
                period: "다음 주 (11/24~11/28)".to_owned(),
            },
            ActionItemV2 {
                task: "GraphQL subscription 기반 실시간 알림 설계 문서".to_owned(),
                detail: "WebSocket vs SSE 비교 + 메시지 스키마 + 클라이언트 재연결 전략".to_owned(),
                owner: "박지민".to_owned(),
                due: "12/01 (월)".to_owned(),
                period: "다음 주 (11/24~11/28)".to_owned(),
            },
        ],
        needs_confirmation: vec![
            "STT상 '프로젝트 알파' — '프로젝트 앱타'일 가능성 있음. 김철수 PM에게 확인 필요.".to_owned(),
            "금액 '5,000만원'이 정확한지 확인 필요 — 음성에서 '5천만원'과 '5백만원' 구분이 불명확. 예산 문서 대조 필요.".to_owned(),
            "'이영희 리드' 명칭 — 음성에서 '영희 씨'와 '이 리드'가 같은 사람인지 문맥상 추정. 참석자 확인 필요.".to_owned(),
            "베타 테스트 시작일 '10월 15일' — '10월 5일'로도 들림. 회의 자료와 대조 필요.".to_owned(),
            "'Playwright'라는 도구명이 STT에서 '플레이라이트', '플레이 라이트' 등 여러 형태로 전사됨. 맥락상 동일한 도구로 판단.".to_owned(),
            "AWS 비용 '월 850만원' — 음성에서 '팔백오십'과 '팔십오'가 혼동될 수 있음. 인프라팀 비용 리포트 대조 필요.".to_owned(),
        ],
    }
}

/// fixture를 JSON 문자열로 직렬화한다 — `parse_note`의 입력이 된다.
fn fixture_json() -> String {
    let fixture = large_v2_fixture();
    serde_json::to_string(&fixture).expect("fixture 직렬화")
}

fn recording() -> Recording {
    Recording {
        id: RecordingId::new("rec-e2e"),
        title: "Q4 사업 전략 회의".to_owned(),
        created_at: "2026-11-13T10:00:00.000Z".to_owned(),
        updated_at: "2026-11-13T12:00:00.000Z".to_owned(),
        duration_ms: 6_780_000, // 1시간 53분
        audio_path: "recordings/rec-e2e.wav".to_owned(),
        audio_format: "wav".to_owned(),
        microphone: None,
        current_transcript_id: Some(TranscriptId::new("tr-e2e")),
        transcription_status: ProcessingStatus::Done,
        ai_status: ProcessingStatus::Done,
        notion_status: ProcessingStatus::None,
    }
}

fn transcript() -> Transcript {
    Transcript {
        id: TranscriptId::new("tr-e2e"),
        recording_id: RecordingId::new("rec-e2e"),
        language: Some("ko".to_owned()),
        segments: vec![
            TranscriptSegment {
                start_ms: 3_000,
                end_ms: 8_000,
                text: "안녕하세요, 그럼 회의를 시작하겠습니다.".to_owned(),
            },
        ],
        raw_text: "안녕하세요, 그럼 회의를 시작하겠습니다.".to_owned(),
        created_at: "2026-11-13T10:00:00.000Z".to_owned(),
        engine: "whisper-rs".to_owned(),
        model: "ggml-base".to_owned(),
        transcription_ms: None,
    }
}

// ── AC-8-3: 크기 상한이 이 fixture를 거부하지 않는다 ─────────────────────────

#[test]
fn the_fixture_respects_all_size_limits() {
    let fixture = large_v2_fixture();

    // 개별 문자열 필드가 MAX_OVERVIEW_CHARS 이내
    assert!(fixture.meeting_title.chars().count() <= MAX_OVERVIEW_CHARS);
    assert!(fixture.meeting_kind.chars().count() <= MAX_OVERVIEW_CHARS);
    assert!(fixture.reliability_notice.chars().count() <= MAX_OVERVIEW_CHARS);

    // 배열 원소 수가 MAX_ITEMS 이내
    assert!(fixture.key_summary.len() <= MAX_ITEMS);
    assert!(fixture.topics.len() <= MAX_ITEMS);
    assert!(fixture.action_items.len() <= MAX_ITEMS);
    assert!(fixture.needs_confirmation.len() <= MAX_ITEMS);
    assert!(fixture.attendees.len() <= MAX_ITEMS);

    // 각 배열 원소가 MAX_ITEM_CHARS 이내
    for item in &fixture.key_summary {
        assert!(item.chars().count() <= MAX_ITEM_CHARS, "keySummary 항목이 너무 길다");
    }
    for item in &fixture.needs_confirmation {
        assert!(item.chars().count() <= MAX_ITEM_CHARS, "needsConfirmation 항목이 너무 길다");
    }
    for topic in &fixture.topics {
        assert!(topic.title.chars().count() <= MAX_ITEM_CHARS, "topic title이 너무 길다");
        for group in &topic.groups {
            for item in &group.items {
                assert!(item.chars().count() <= MAX_ITEM_CHARS, "topic item이 너무 길다");
            }
        }
    }
    for ai in &fixture.action_items {
        assert!(ai.task.chars().count() <= MAX_ITEM_CHARS, "action item task가 너무 길다");
    }
}

#[test]
fn the_fixture_is_at_least_25kb() {
    let json = fixture_json();
    assert!(
        json.len() >= 25_000,
        "fixture가 25KB 이상이어야 한다: {} bytes",
        json.len()
    );
}

#[test]
fn output_reserve_tokens_covers_the_fixture() {
    let json = fixture_json();
    let estimated_tokens =
        u32::try_from(json.len().div_ceil(CHARS_PER_ESTIMATED_TOKEN)).unwrap();
    assert!(
        estimated_tokens <= OUTPUT_RESERVE_TOKENS,
        "fixture({estimated_tokens} 토큰 추정)가 예약({OUTPUT_RESERVE_TOKENS})보다 크다"
    );
}

// ── AC-8-1: parse → encode → decode → Markdown render 전부 통과 ─────────────

#[test]
fn the_25kb_v2_fixture_passes_the_full_pipeline() {
    let json = fixture_json();

    // 1) parse_note — 생성된 텍스트를 StructuredNote로 읽는다.
    let parsed = parse_note(NoteType::Meeting, &json)
        .expect("25KB V2 fixture가 parse_note를 통과해야 한다");
    assert!(matches!(parsed, StructuredNote::MeetingV2(_)));

    // 2) encode_content — 저장할 문자열로 만든다.
    let encoded = encode_content(&parsed);
    assert!(encoded.contains("\"schemaVersion\":2"));
    assert!(encoded.contains("\"mode\":\"meeting\""));

    // 3) decode_content — 저장된 문자열을 다시 읽는다.
    let decoded = decode_content(&encoded, NoteType::Meeting)
        .expect("encode된 content를 decode할 수 있어야 한다");
    assert_eq!(decoded, parsed, "encode → decode 왕복이 값을 보존해야 한다");

    // 4) Markdown render — 문서를 만든다.
    let rec = recording();
    let tr = transcript();
    let document = ExportDocument {
        recording: &rec,
        transcript: &tr,
        note: Some(&decoded),
    };
    let markdown = render(&document, true);

    // 결과가 비어 있지 않고, 올바른 문서 구조다.
    assert!(markdown.starts_with("# Q4 사업 전략 회의\n"));
    assert!(markdown.contains("Date: 2026-11-13\n"));
    assert!(markdown.contains("Duration: 1:53:00\n"));
    assert!(markdown.ends_with('\n'));
    assert!(!markdown.ends_with("\n\n"));

    // 5) 앱 noteSections 확인 — Rust 쪽에서는 StructuredNote가 올바른 V2 필드를 갖고 있는지 확인.
    let StructuredNote::MeetingV2(ref v2) = decoded else {
        panic!("MeetingV2여야 한다");
    };
    assert_eq!(v2.key_summary.len(), 7, "핵심 요약 7개");
    assert_eq!(v2.topics.len(), 6, "topics 6개");
    assert!(v2.action_items.len() >= 15, "action items 15+개");
    assert!(v2.needs_confirmation.len() >= 5, "needs confirmation 5+개");
    // previousActionItems 각 분류에 값이 있다.
    assert!(!v2.previous_action_items.done.is_empty());
    assert!(!v2.previous_action_items.changed.is_empty());
    assert!(!v2.previous_action_items.in_progress.is_empty());
    assert!(!v2.previous_action_items.no_progress.is_empty());
    assert!(!v2.previous_action_items.not_mentioned.is_empty());
}

// ── AC-8-2: Markdown 출력의 섹션 순서와 이모지 확인 ─────────────────────────

#[test]
fn the_markdown_contains_v2_sections_in_the_reference_format_order() {
    let json = fixture_json();
    let parsed = parse_note(NoteType::Meeting, &json).expect("parse");
    let rec = recording();
    let tr = transcript();
    let document = ExportDocument {
        recording: &rec,
        transcript: &tr,
        note: Some(&parsed),
    };
    let markdown = render(&document, true);

    // 기준 형식의 네 가지 이모지 섹션이 모두 존재한다.
    for section_title in &MEETING_V2_SECTIONS {
        assert!(
            markdown.contains(&format!("## {section_title}")),
            "섹션이 없다: {section_title}"
        );
    }

    // 머리말의 📌 안내가 있다.
    assert!(markdown.contains("> 📌"), "📌 안내 문구가 없다");

    // 메타데이터가 있다.
    assert!(markdown.contains("- **회의명**:"), "회의명 메타데이터가 없다");
    assert!(markdown.contains("- **유형**:"), "유형 메타데이터가 없다");
    assert!(markdown.contains("- **참석자**:"), "참석자 메타데이터가 없다");

    // 섹션의 순서를 확인한다: 🔑 → 🔁 → (topics) → ✅ → 🔍
    let pos_key = markdown.find("## 🔑 핵심 요약").expect("🔑");
    let pos_prev = markdown.find("## 🔁 지난주 액션 아이템 진행 상황").expect("🔁");
    let pos_topic1 = markdown.find("## 1.").expect("첫 번째 topic");
    let pos_action = markdown.find("## ✅ 액션 아이템").expect("✅");
    let pos_confirm = markdown.find("## 🔍 확인 필요").expect("🔍");
    let pos_transcript = markdown.find("## Transcript").expect("Transcript");

    assert!(pos_key < pos_prev, "🔑 → 🔁 순서");
    assert!(pos_prev < pos_topic1, "🔁 → topics 순서");
    assert!(pos_topic1 < pos_action, "topics → ✅ 순서");
    assert!(pos_action < pos_confirm, "✅ → 🔍 순서");
    assert!(pos_confirm < pos_transcript, "🔍 → Transcript 순서");

    // topic 번호가 연속적이다.
    for i in 1..=6 {
        assert!(
            markdown.contains(&format!("## {i}.")),
            "topic {i} 번호가 없다"
        );
    }

    // 액션 아이템이 기간별로 그룹된다.
    assert!(markdown.contains("### 이번 주"), "이번 주 기간 그룹");
    assert!(markdown.contains("### 다음 주"), "다음 주 기간 그룹");

    // 액션 아이템 체크박스 형식이 맞다.
    assert!(markdown.contains("- [ ]  **"), "체크박스 형식");
    assert!(markdown.contains("📅"), "📅 마감일 표시");

    // previousActionItems의 분류 제목이 있다.
    assert!(markdown.contains("**완료**"), "완료 분류");
    assert!(markdown.contains("**일정·방향 변경**"), "변경 분류");
    assert!(markdown.contains("**진행 중**"), "진행 중 분류");
    assert!(markdown.contains("**진전 없음**"), "진전 없음 분류");
    assert!(markdown.contains("**회의에서 언급 없음**"), "언급 없음 분류");
}

#[test]
fn the_markdown_deterministically_renders_the_same_string() {
    let json = fixture_json();
    let parsed = parse_note(NoteType::Meeting, &json).expect("parse");
    let rec = recording();
    let tr = transcript();
    let document = ExportDocument {
        recording: &rec,
        transcript: &tr,
        note: Some(&parsed),
    };

    assert_eq!(render(&document, true), render(&document, true));
}
