//! 실제 장치에서 샘플을 받는 유일한 자리 (ADR-0003의 잠정 선택 · `cpal`).
//!
//! [`super::system_devices`]와 짝이다 — 그 파일이 장치를 **열거**하고, 이 파일이 장치를 **연다.**
//! 두 파일 밖에서는 `cpal`을 알지 않는다. 그래서 캡처가 만들어 내는 파일과 보고 값의 규칙
//! 전부를 이 경로를 지나지 않고 검증할 수 있다 (`super::capture`의 테스트 · §18).
//!
//! **자동 테스트는 이 파일의 코드를 실행하지 않는다.** 여기서부터는 실제 하드웨어와 OS 권한의
//! 영역이다 — 장치가 실제로 열리는지, macOS TCC 프롬프트가 뜨는지, 어떤 샘플레이트·채널이
//! 오는지는 전부 **UNVERIFIED**이며 사람이 번들된 앱에서 확인한다
//! (`docs/ADR-0003-recording-engine.md` §12).
//!
//! ## 스트림은 자기 스레드를 떠나지 않는다
//!
//! `cpal::Stream`은 스레드 사이를 건널 수 없다. 그래서 이 파일은 스트림 하나를 소유하는
//! 스레드를 만들고, 바깥에는 **정지 신호와 결과만** 건넨다. 스트림은 그 스레드에서 태어나
//! 그 스레드에서 닫힌다.

use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::audio::capture::{CaptureFormat, OpenCapture, SampleSource};
use crate::audio::devices::{catalog, selection_keys, ObservedInputDevice};
use crate::audio::system_devices::name_of;
use crate::domain::{Failure, FailureKind};

/// 캡처 도중에 일어난 사건. 정지할 때 사용자에게 전달된다.
///
/// 오디오 콜백은 실패를 돌려줄 곳이 없다 — 그래서 여기에 적어 두고 정지 시점에 꺼낸다.
type Interrupted = Arc<Mutex<Option<Failure>>>;

/// 이 기기의 실제 입력 장치를 `cpal`로 연다.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemSampleSource;

impl SampleSource for SystemSampleSource {
    fn open(&self, device_key: &str, samples: SyncSender<Vec<i16>>) -> Result<OpenCapture, Failure> {
        let interrupted: Interrupted = Arc::new(Mutex::new(None));

        // 스트림을 소유할 스레드와 주고받는 두 통로: 열린 결과와 정지 신호.
        let (ready_sender, ready) = mpsc::channel::<Result<(String, CaptureFormat), Failure>>();
        let (stop_sender, stop_signal) = mpsc::channel::<()>();

        let key = device_key.to_string();
        let slot = Arc::clone(&interrupted);
        let device_thread = thread::Builder::new()
            .name("molt-note-capture-device".to_string())
            .spawn(move || {
                let opened = match open_stream(&key, samples, slot) {
                    Ok(opened) => opened,
                    Err(failure) => {
                        // 부른 쪽이 이미 사라졌다면 보낼 곳이 없다. 그것도 정상 종료다.
                        let _ = ready_sender.send(Err(failure));
                        return;
                    }
                };

                if ready_sender.send(Ok((opened.label, opened.format))).is_err() {
                    return;
                }
                // 정지 신호를 기다린다. 보내는 쪽이 사라진 것도 같은 뜻이다.
                let _ = stop_signal.recv();
                drop(opened.stream);
            })
            .map_err(|error| {
                Failure::retryable(
                    FailureKind::AudioDevice,
                    "녹음을 시작하지 못했다. 오디오 스레드를 만들지 못했다.",
                )
                .with_detail(error)
            })?;

        let opened = ready.recv().map_err(|_| {
            Failure::retryable(
                FailureKind::AudioDevice,
                "입력 장치를 여는 중 문제가 생겨 녹음을 시작하지 못했다.",
            )
        })?;

        let (device_label, format) = match opened {
            Ok(opened) => opened,
            Err(failure) => {
                let _ = device_thread.join();
                return Err(failure);
            }
        };

        Ok(OpenCapture {
            device_label,
            format,
            stop: Box::new(move || {
                // 스트림을 가진 스레드에 알리고, 그 스레드가 스트림을 닫을 때까지 기다린다.
                let _ = stop_sender.send(());
                if device_thread.join().is_err() {
                    return Err(Failure::retryable(
                        FailureKind::AudioDevice,
                        "녹음을 멈추는 중 문제가 있었다.",
                    ));
                }
                match interrupted.lock() {
                    Ok(mut slot) => match slot.take() {
                        Some(failure) => Err(failure),
                        None => Ok(()),
                    },
                    Err(_) => Err(Failure::retryable(
                        FailureKind::AudioDevice,
                        "녹음이 끝까지 정상이었는지 확인하지 못했다.",
                    )),
                }
            }),
        })
    }
}

/// 열려서 돌아가고 있는 스트림 하나.
struct OpenedStream {
    label: String,
    format: CaptureFormat,
    stream: cpal::Stream,
}

/// 고른 장치를 찾아 열고 캡처를 시작한다. **스트림을 소유한 스레드 안에서만 불린다.**
fn open_stream(
    device_key: &str,
    samples: SyncSender<Vec<i16>>,
    interrupted: Interrupted,
) -> Result<OpenedStream, Failure> {
    let host = cpal::default_host();
    let devices: Vec<cpal::Device> = host
        .input_devices()
        .map_err(|error| {
            Failure::retryable(FailureKind::AudioDevice, "입력 장치 목록을 읽지 못했다.")
                .with_detail(error)
        })?
        .collect();

    let names: Vec<String> = devices
        .iter()
        .map(|device| name_of(device).unwrap_or_default())
        .collect();

    // 목록을 만들 때와 **같은 규칙으로** 키를 만든다. 그래서 사용자가 고른 항목과
    // 여기서 열리는 장치가 같은 것이다 (`super::devices::selection_keys`).
    let index = selection_keys(names.iter().map(String::as_str))
        .iter()
        .position(|candidate| candidate == device_key)
        .ok_or_else(|| {
            Failure::retryable(
                FailureKind::AudioDevice,
                "고른 입력 장치를 찾지 못했다. 장치가 빠졌을 수 있다.",
            )
        })?;

    let device = devices.get(index).ok_or_else(|| {
        Failure::retryable(FailureKind::AudioDevice, "고른 입력 장치를 열지 못했다.")
    })?;

    // 보여줄 이름도 목록과 같은 규칙으로 정한다 — 이름이 없는 장치도 부를 이름을 갖는다.
    let label = catalog(names.iter().map(ObservedInputDevice::named))
        .into_iter()
        .find(|listed| listed.key == device_key)
        .map(|listed| listed.label)
        .unwrap_or_default();

    let config = device.default_input_config().map_err(|error| {
        Failure::retryable(
            FailureKind::AudioDevice,
            "입력 장치의 기본 형식을 읽지 못했다.",
        )
        .with_detail(error)
    })?;

    // 형식을 지어내지 않는다 — 장치가 알려준 샘플레이트와 채널 수를 그대로 쓴다.
    // 리샘플링·다운믹스는 여기에 없다 (`super::capture` 모듈 주석의 UNVERIFIED).
    let format = CaptureFormat::pcm_16bit(config.sample_rate(), config.channels());
    let stream_config: cpal::StreamConfig = config.into();

    // 오류 콜백은 여기서 바로 만든다. 인자 타입은 `cpal`이 정하므로 그 이름을 적지 않고
    // 그대로 받는다 — 필요한 것은 사용자에게 옮겨 적을 수 있다는 사실 하나뿐이다.
    let built = match config.sample_format() {
        cpal::SampleFormat::I16 => {
            let slot = Arc::clone(&interrupted);
            device.build_input_stream(
                stream_config,
                forward_i16(samples.clone(), Arc::clone(&interrupted)),
                move |error| note_stream_error(&slot, error),
                None,
            )
        }
        cpal::SampleFormat::F32 => {
            let slot = Arc::clone(&interrupted);
            device.build_input_stream(
                stream_config,
                forward_f32(samples.clone(), Arc::clone(&interrupted)),
                move |error| note_stream_error(&slot, error),
                None,
            )
        }
        other => {
            return Err(Failure::permanent(
                FailureKind::AudioDevice,
                "이 입력 장치가 주는 샘플 형식은 아직 다루지 못한다. 다른 장치를 골라야 한다.",
            )
            .with_detail(format!("sample format: {other:?}")))
        }
    };

    let stream = built.map_err(|error| {
        Failure::retryable(
            FailureKind::AudioDevice,
            "입력 장치를 열지 못했다. 다른 앱이 쓰고 있거나 마이크 권한이 없을 수 있다.",
        )
        .with_detail(error)
    })?;

    stream.play().map_err(|error| {
        Failure::retryable(FailureKind::AudioDevice, "녹음을 시작하지 못했다.").with_detail(error)
    })?;

    Ok(OpenedStream {
        label,
        format,
        stream,
    })
}

/// 장치가 16-bit 정수로 주는 경우. 그대로 넘긴다.
fn forward_i16(
    samples: SyncSender<Vec<i16>>,
    interrupted: Interrupted,
) -> impl FnMut(&[i16], &cpal::InputCallbackInfo) + Send + 'static {
    move |data, _| deliver(&samples, &interrupted, data.to_vec())
}

/// 장치가 32-bit 실수로 주는 경우(macOS CoreAudio의 흔한 경우). 16-bit PCM으로 옮긴다.
fn forward_f32(
    samples: SyncSender<Vec<i16>>,
    interrupted: Interrupted,
) -> impl FnMut(&[f32], &cpal::InputCallbackInfo) + Send + 'static {
    move |data, _| {
        deliver(
            &samples,
            &interrupted,
            data.iter().copied().map(to_i16).collect(),
        )
    }
}

/// 스트림 자체가 낸 오류를 적어 둔다. 캡처가 중간에 끊긴 경우다.
///
/// 오류 타입을 이름으로 받지 않는다 — 콜백의 인자 타입은 `cpal`이 정하고, 이 함수가 그 값에
/// 요구하는 것은 사용자에게 옮겨 적을 수 있다는 사실 하나뿐이다.
fn note_stream_error(interrupted: &Interrupted, error: impl std::fmt::Display) {
    note(
        interrupted,
        Failure::retryable(FailureKind::AudioDevice, "녹음이 중간에 끊겼다.").with_detail(error),
    );
}

/// 받은 샘플을 파일에 쓰는 쪽으로 넘긴다.
fn deliver(samples: &SyncSender<Vec<i16>>, interrupted: &Interrupted, chunk: Vec<i16>) {
    match samples.try_send(chunk) {
        Ok(()) => {}
        // 큐가 찼다. **버렸다는 사실을 숨기지 않는다** — 정지할 때 사용자에게 알린다 (R-005).
        Err(TrySendError::Full(_)) => note(
            interrupted,
            Failure::retryable(
                FailureKind::AudioDevice,
                "저장이 녹음을 따라가지 못해 일부 구간이 유실됐다.",
            ),
        ),
        // 받는 쪽이 이미 끝났다. 정지 중이라는 뜻이며 실패가 아니다.
        Err(TrySendError::Disconnected(_)) => {}
    }
}

/// 캡처 도중의 사건을 적어 둔다. **먼저 일어난 것 하나만 남긴다** —
/// 같은 사건이 콜백마다 반복돼도 사용자에게 필요한 것은 한 문장이다.
fn note(interrupted: &Interrupted, failure: Failure) {
    if let Ok(mut slot) = interrupted.lock() {
        if slot.is_none() {
            *slot = Some(failure);
        }
    }
}

/// 32-bit 실수 샘플 하나를 16-bit PCM으로. 범위를 벗어난 값은 잘라 낸다.
///
/// `as`는 포화 변환이므로 값이 얼마든 여기서 죽지 않는다.
fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    // 이 파일에서 하드웨어 없이 검증할 수 있는 것은 샘플 값 변환 하나뿐이다.
    // 장치를 여는 나머지 경로는 실제 하드웨어의 영역이며, 사람이 확인한다 (ADR-0003 §12).

    #[test]
    fn float_samples_become_16_bit_without_wrapping_around() {
        assert_eq!(to_i16(0.0), 0);
        assert_eq!(to_i16(1.0), i16::MAX);
        assert_eq!(to_i16(-1.0), -i16::MAX);
    }

    #[test]
    fn samples_outside_the_expected_range_are_clipped_not_wrapped() {
        // 넘치는 값이 반대 부호로 감기면 딸깍거리는 잡음이 된다.
        assert_eq!(to_i16(9.0), i16::MAX);
        assert_eq!(to_i16(-9.0), -i16::MAX);
        assert_eq!(to_i16(f32::INFINITY), i16::MAX);
        assert_eq!(to_i16(f32::NEG_INFINITY), -i16::MAX);
        assert_eq!(to_i16(f32::NAN), 0, "값을 알 수 없으면 무음이다");
    }
}
