pub mod audio;
pub mod commands;
pub mod db;
pub mod domain;
pub mod platform;
pub mod transcription;

use tauri::Manager;

use commands::{AudioDevices, Recorder, Storage, Transcriber};
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
        ])
        .run(tauri::generate_context!());

    if let Err(error) = app {
        // 웹뷰 자체를 띄우지 못한 실패다. 이 시점에는 실패를 보여줄 화면이 없으므로
        // 표준 오류로 알리고 실패한 종료 코드로 끝낸다.
        eprintln!("Molt Note를 실행하지 못했다: {error}");
        std::process::exit(1);
    }
}
