pub mod ai;
pub mod audio;
pub mod commands;
pub mod db;
pub mod domain;
pub mod export;
pub mod notion;
pub mod platform;
pub mod sync;
pub mod transcription;

use tauri::Manager;

use commands::{
    AudioDevices, Exporter, NoteGenerator, NotionSender, Recorder, Storage, Transcriber,
};
use platform::app_data_dir::AppDataDirectory;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            // 저장소는 앱 시작 시 한 번 연다. 이후의 접근은 모두 command를 통한다 (ADR-0001).
            //
            // **열지 못해도 여기서 멈추지 않는다.** 실패는 Storage 안에 값으로 남아
            // 사용자가 부르는 command마다 그대로 전달된다 — 창은 뜨고, 화면은 무엇이
            // 실패했는지 말할 수 있다. 여기서 죽으면 그 설명이 콘솔에만 남는다 (§13).
            app.manage(Storage::open_for(app));

            // 입력 장치는 여기서 열지 않는다. 목록은 물어볼 때마다 새로 얻으므로
            // 앱 시작이 마이크나 마이크 권한에 의존하지 않는다 (ADR-0003).
            app.manage(AudioDevices::system());

            // 녹음 session의 소유자는 여기다 — **화면이 아니다.** 화면이 다시 그려지거나
            // 사용자가 다른 화면에 다녀와도 진행 중인 녹음은 이 값 안에 그대로 있다
            // (R-001 · docs/ADR-0004-recording-session-lifecycle.md).
            //
            // 장치는 여기서 열지 않는다. 여는 것은 사용자가 시작할 때다. 출력 파일이 놓일
            // 자리를 얻지 못하면 그 실패를 값으로 들고 있다가 녹음을 시작하려 할 때
            // 알린다 — 앱 시작을 막지 않는다 (§13).
            app.manage(Recorder::open_for(app));

            // 진행 중인 전사의 소유자도 여기다 — 녹음과 같은 이유이며 하나가 더 있다.
            // 전사는 1시간 분량이면 오래 걸리므로 **배경 스레드에서 돈다.** 그동안 다른
            // command와 화면은 계속 응답하고, 화면이 사라져도 돌던 전사는 사라지지 않는다
            // (`phase-prompt/03` 요구 3 · crate::commands::transcriber).
            //
            // 모델도 엔진도 여기서 열지 않는다. 모델은 전사할 때마다 해석되며, 지정된 모델이
            // 없는 것은 앱 시작을 막는 문제가 아니라 전사를 시작할 때 알리는 제품 상태다
            // (§13 · ADR-0007 §8.2).
            app.manage(Transcriber::open_for(app));

            // 진행 중인 AI 노트 생성의 소유자도 여기다 — 전사와 같은 이유이며, 로컬 모델의
            // 생성은 그보다 더 오래 걸릴 수 있다 (ADR-0008 §12.2). 그래서 같은 규약을 쓴다:
            // **배경 스레드에서 돌고, 화면은 상태를 물어본다**
            // (crate::commands::notes · phase-prompt/04 요구 16).
            //
            // provider는 여기서 만들지 않는다. 어떤 provider에 어떤 모델로 연결할지는 생성할
            // 때마다 설정에서 읽으며 (ADR-0008 §11.1), **고르지 않은 상태는 앱 시작을 막는
            // 문제가 아니라 정상 상태다** — AI를 설정하지 않은 사용자에게도 녹음 · 전사 ·
            // 열람은 그대로 동작한다 (INV-8).
            app.manage(NoteGenerator::open_for(app));

            // Markdown export도 앱 데이터 루트 하나에서 자리를 얻는다 (INV-10 · ADR-0009 §4.1).
            // **여기서 디렉터리를 만들지 않는다** — 한 번도 내보내지 않은 사용자에게 빈
            // `exports/`를 만들어 두지 않으려는 것이며, 자리는 실제로 내보낼 때 준비된다.
            //
            // 진행 중인 무언가를 들고 있지도 않다. export는 기다릴 모델도 서버도 없는 짧은
            // 일이라 배경 스레드와 상태 조회 규약을 쓰지 않는다 (crate::commands::export).
            // 경로를 얻지 못한 실패는 값으로 남아 내보내려 할 때 사용자에게 전달된다 (§13).
            app.manage(Exporter::open_for(app));

            // 진행 중인 **Notion 전송**의 소유자도 여기다 — 전사 · 노트 생성과 같은 이유이며
            // (R-001), 하나가 더 있다: 긴 transcript는 여러 요청으로 나뉘어 나가고 속도 제한을
            // 만나면 그 사이에 기다린다 (ADR-0009 §6 · §9). 그동안 목록을 보거나 설정을 바꾸는
            // 일이 멈추면 안 되고, 화면을 떠났다 왔다는 이유로 진행 중인 전송이 사라져도 안 된다
            // (crate::commands::notion).
            //
            // **token은 여기서 읽지 않는다.** 값이 지나가는 자리는 전송이 도는 배경 스레드 하나뿐이며
            // (ADR-0009 §10.4 · INV-7), 어느 자격증명 저장소가 서는지는 platform 경계가 정한다
            // (INV-10). 저장하지 않은 것은 앱 시작을 막는 문제가 아니라 정상 상태다 (INV-8) —
            // Notion을 설정하지 않은 사용자에게도 녹음 · 전사 · 열람 · Markdown export는 그대로
            // 동작한다.
            app.manage(NotionSender::open_for(app));

            // 저장된 녹음을 재생하려면 webview가 그 파일을 읽을 수 있어야 한다.
            // 그 통로는 Tauri v2의 asset protocol이고(`protocol-asset` feature),
            // **열어 주는 자리는 녹음 디렉터리 하나뿐이다** — 사용자의 홈도, 앱 데이터
            // 루트도 아니다 (PRODUCT-SPEC §12). 경로는 여기서 짓지 않고 파일을 쓰는 쪽과
            // 같은 자리에서 받는다 ([`AppDataDirectory::recordings_dir`] · INV-10).
            //
            // 이 통로는 로컬 webview로만 향한다. 오디오를 밖으로 보내는 경로는 이 앱에
            // 없다 (INV-6).
            //
            // 열지 못해도 앱은 뜬다 — 재생만 되지 않으며, 그 사실은 화면에서 드러난다.
            // 여기서 죽으면 녹음도 목록도 함께 잃는다 (§13).
            if let Ok(app_data_dir) = AppDataDirectory::from_manager(app) {
                if let Ok(recordings_dir) = app_data_dir.ensure_recordings_dir() {
                    // 하위 디렉터리까지 열지 않는다(`recursive: false`) — 녹음 파일은
                    // 이 디렉터리에 바로 놓인다 (`capture::output_path`).
                    let _ = app
                        .asset_protocol_scope()
                        .allow_directory(&recordings_dir, false);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // frontend가 부를 수 있는 전부다 (crate::commands).
            commands::list_recordings,
            commands::get_recording,
            // 저장된 Transcript를 segment까지 읽는 자리다. **읽기뿐이다** — Transcript는
            // immutable이므로 고치거나 지우는 이름은 여기에도 저장소에도 없다 (§7.1 · INV-2).
            commands::get_transcript,
            commands::create_recording,
            commands::delete_recording,
            commands::get_settings,
            commands::update_settings,
            commands::list_input_devices,
            commands::start_capture,
            commands::pause_capture,
            commands::resume_capture,
            commands::stop_capture,
            commands::capture_status,
            // 레코드와 파일이 어긋난 상태를 알리는 자리다 — 고치거나 지우지 않는다 (INV-4).
            commands::list_missing_audio,
            // 전사를 **움직이는** 표면은 이 둘뿐이다 — 한 건 시작과 상태 조회. 여러 Recording을
            // 줄 세우는 큐는 이 Phase의 범위 밖이다 (PRODUCT-SPEC §16 DEFERRED).
            commands::start_transcription,
            commands::transcription_status,
            // AI 노트의 표면은 이 다섯이다 — provider 상태 조회 · 생성 시작 · 진행 상태 조회 ·
            // 저장된 노트 읽기 둘. **고치거나 지우는 이름은 없다**: 재생성은 대체가 아니라
            // 추가이며 (ADR-0008 §9.2), 저장소의 `ai_notes` 쓰기 경로도 추가 하나뿐이다.
            //
            // 벤더 이름이 이 목록에 없다는 것이 INV-9의 표현이다 — 어떤 provider를 쓰는지는
            // 설정 값이고, 그것을 아는 코드는 adapter 안에만 있다.
            commands::ai_provider_status,
            commands::start_ai_note,
            commands::ai_note_status,
            commands::list_ai_notes,
            commands::get_ai_note,
            // Markdown export의 표면은 이 하나다 — Recording 하나를 파일 하나로 만든다.
            // **AI가 없어도 부를 수 있고** (INV-8), 이미 있는 파일을 덮어쓰지 않으며
            // (ADR-0009 §4.3), 저장된 것을 고치거나 지우는 이름은 여기에도 없다.
            commands::export_markdown,
            // Notion 전송의 표면은 이 여섯이다 — 전송 시작 · 진행 상태 조회 · 저장된 전송 기록
            // 읽기 · 연결 확인 · token 저장 · token 삭제. **저장된 것을 고치거나 지우는 이름은
            // 여기에도 없다**: 지우는 하나는 이 앱이 넣은 자격증명 항목이며, 녹음 · 전사 · 노트 ·
            // 이미 만들어진 Notion 페이지를 지우는 경로는 어디에도 없다 (INV-3 · INV-4).
            //
            // **token을 돌려주는 이름이 없다는 것이 INV-7의 표현이다** — 값은 저장 command의
            // 입력으로 한 번 지나가고, 조회가 답하는 것은 저장돼 있다는 사실뿐이다.
            commands::start_notion_sync,
            commands::notion_sync_status,
            commands::get_notion_sync,
            commands::check_notion_connection,
            commands::save_notion_token,
            commands::delete_notion_token,
        ])
        .run(tauri::generate_context!());

    if let Err(error) = app {
        // 웹뷰 자체를 띄우지 못한 실패다. 이 시점에는 실패를 보여줄 화면이 없으므로
        // 표준 오류로 알리고 실패한 종료 코드로 끝낸다.
        eprintln!("Molt Note를 실행하지 못했다: {error}");
        std::process::exit(1);
    }
}
