pub mod audio;
pub mod commands;
pub mod db;
pub mod domain;
pub mod platform;

use tauri::Manager;

use commands::{AudioDevices, Capture, Storage};

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

            // 캡처도 여기서 장치를 열지 않는다. 여는 것은 사용자가 시작할 때다.
            // 출력 파일이 놓일 자리를 얻지 못하면 그 실패를 값으로 들고 있다가
            // 녹음을 시작하려 할 때 알린다 — 앱 시작을 막지 않는다 (§13).
            app.manage(Capture::open_for(app));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // frontend가 부를 수 있는 전부다 (crate::commands).
            commands::list_recordings,
            commands::get_recording,
            commands::create_recording,
            commands::delete_recording,
            commands::get_settings,
            commands::update_settings,
            commands::list_input_devices,
            commands::start_capture,
            commands::stop_capture,
        ])
        .run(tauri::generate_context!());

    if let Err(error) = app {
        // 웹뷰 자체를 띄우지 못한 실패다. 이 시점에는 실패를 보여줄 화면이 없으므로
        // 표준 오류로 알리고 실패한 종료 코드로 끝낸다.
        eprintln!("Molt Note를 실행하지 못했다: {error}");
        std::process::exit(1);
    }
}
