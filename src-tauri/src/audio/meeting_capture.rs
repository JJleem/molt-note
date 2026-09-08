//! 마이크와 시스템 오디오를 **함께** 받는 [`SampleSource`] 구현 (ADR-0012 · §22).
//!
//! ```text
//! platform::system_audio  ─→ aggregate device (마이크 1 + tap 2)
//!                              ↓ AudioDeviceIOProc  ← 실시간 스레드
//!                            meeting_mix::fold_to_stereo
//!                              ↓ SampleSink::try_send
//!                            capture.rs 의 drain · WavFile · InputLevel  ← 그대로 재사용
//! ```
//!
//! ## `capture.rs`를 한 줄도 바꾸지 않는다
//!
//! [`SampleSource`]는 Phase 2B가 이미 세운 경계다 (`capture.rs`). 회의 입력은 그 trait의
//! **두 번째 구현**일 뿐이며, 파일 쓰기 · 일시정지 · 정지 · 레벨 측정 · 파일 확정은 전부
//! 기존 경로 그대로다 — ADR-0012 §9가 요구한 "같은 파일 쓰기와 같은 종료 규칙"이 이렇게
//! 지켜진다.
//!
//! ## 실시간 스레드에서 하지 않는 것
//!
//! IOProc은 오디오 실시간 스레드에서 돈다. 거기서 막히면 **소리가 끊긴다.**
//!
//! ```text
//! 하지 않는다   락을 기다리는 것 · 파일 쓰기 · 할당을 기다리는 것 · panic
//! 한다          채널 접기(순수 함수) · try_send 한 번
//! ```
//!
//! 큐가 가득 차면 **버리지 않고 세어 둔다.** 정지할 때 그 수가 실패로 올라간다 —
//! `SampleSink`가 세운 규칙과 같다.

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use objc2_core_audio::{
    kAudioHardwareNoError, AudioDeviceCreateIOProcID, AudioDeviceDestroyIOProcID, AudioDeviceStart,
    AudioDeviceStop, AudioDeviceIOProcID, AudioObjectID,
};
use objc2_core_audio_types::{AudioBufferList, AudioTimeStamp};

use super::capture::{CaptureFormat, OpenCapture, SampleSink, SampleSource};
use super::meeting_mix::{fold_to_stereo, OUTPUT_CHANNELS};
use crate::domain::{Failure, FailureKind};
use crate::platform::system_audio::{self, MeetingInput};

/// aggregate device가 내는 샘플레이트. **[A✓ 2026-09-08] 48 kHz로 관측됐다.**
///
/// **[미검증]** 다른 기기·다른 마이크에서도 48 kHz인지는 확인되지 않았다. 값을 장치에
/// 물어보는 것이 옳지만, 그 값이 녹음 도중에 바뀌면(AirPods 연결 등) 파일 헤더와 어긋난다 —
/// 그 경우를 어떻게 다룰지 정하기 전에는 고정값을 쓰고 이 주석을 남긴다.
const SAMPLE_RATE_HZ: u32 = 48_000;

/// IOProc과 정지 경로가 함께 보는 값.
///
/// **실시간 스레드가 만지는 것은 이 안의 원자값과 `SampleSink` 하나뿐이다.**
struct Shared {
    sink: SampleSink,
    /// 큐가 가득 차 보내지 못한 프레임 수. **버린 것을 세어 둔다.**
    dropped_frames: AtomicU64,
}

/// 회의 입력을 여는 [`SampleSource`].
pub struct MeetingSource;

impl SampleSource for MeetingSource {
    /// `device_key`는 마이크의 Core Audio UID다.
    ///
    /// **어느 마이크인지 이 구현이 고르지 않는다** — 부르는 쪽이 준 값을 그대로 쓴다.
    /// 기본 장치를 주면 AirPods를 끼는 순간 그것이 잡힌다 (§22).
    fn open(&self, device_key: &str, samples: SampleSink) -> Result<OpenCapture, Failure> {
        let input = system_audio::open(device_key)?;
        let device_id = input.device_id();

        let shared = Arc::new(Shared {
            sink: samples,
            dropped_frames: AtomicU64::new(0),
        });

        // IOProc에 넘길 raw 포인터. **정지할 때 여기서 다시 거둬들인다.**
        let context = Arc::into_raw(Arc::clone(&shared)) as *mut c_void;

        let mut proc_id: AudioDeviceIOProcID = None;
        let status = unsafe {
            AudioDeviceCreateIOProcID(
                device_id,
                Some(io_proc),
                context,
                NonNull::from(&mut proc_id),
            )
        };
        if status != kAudioHardwareNoError {
            unsafe { drop(Arc::from_raw(context as *const Shared)) };
            return Err(io_proc_failed("입력을 읽을 준비를 하지 못했다.", status));
        }

        let status = unsafe { AudioDeviceStart(device_id, proc_id) };
        if status != kAudioHardwareNoError {
            unsafe {
                AudioDeviceDestroyIOProcID(device_id, proc_id);
                drop(Arc::from_raw(context as *const Shared));
            }
            return Err(io_proc_failed("입력을 시작하지 못했다.", status));
        }

        let stop = Stopper {
            input,
            proc_id,
            context,
            shared,
        };

        Ok(OpenCapture {
            device_label: system_audio::AGGREGATE_NAME.to_owned(),
            format: CaptureFormat::pcm_16bit(SAMPLE_RATE_HZ, OUTPUT_CHANNELS),
            stop: Box::new(move || stop.stop()),
        })
    }
}

/// 정지에 필요한 전부. **`Drop`이 아니라 명시적인 [`Self::stop`]이 정리한다** —
/// 정지의 성공/실패가 값으로 올라가야 하기 때문이다 (`OpenCapture::stop`).
struct Stopper {
    input: MeetingInput,
    proc_id: AudioDeviceIOProcID,
    context: *mut c_void,
    shared: Arc<Shared>,
}

// IOProc에 넘긴 포인터를 다른 스레드에서 거둬들인다. 그 포인터가 가리키는 것은 `Arc`이며,
// 그 안의 값은 전부 `Send + Sync`다.
unsafe impl Send for Stopper {}

impl Stopper {
    fn stop(self) -> Result<(), Failure> {
        let device_id = self.input.device_id();

        // **순서가 규칙이다**: 멈추고 → 떼어내고 → 그다음에야 context를 거둔다.
        // 거꾸로 하면 아직 도는 콜백이 해제된 메모리를 읽는다.
        unsafe {
            AudioDeviceStop(device_id, self.proc_id);
            AudioDeviceDestroyIOProcID(device_id, self.proc_id);
            drop(Arc::from_raw(self.context as *const Shared));
        }

        let dropped = self.shared.dropped_frames.load(Ordering::Relaxed);
        // `self.input`이 여기서 떨어지며 tap과 aggregate device를 지운다.
        drop(self.input);

        if dropped > 0 {
            return Err(Failure::retryable(
                FailureKind::AudioDevice,
                "녹음 중에 일부 소리를 파일에 넘기지 못했다.",
            )
            .with_detail(format!("dropped_frames={dropped}"))
            .with_source_data_at_risk());
        }
        Ok(())
    }
}

/// 오디오 실시간 스레드에서 도는 콜백.
///
/// **여기서 하는 일이 곧 위험이다.** 채널을 접고 한 번 보낸다. 그 이상은 하지 않는다.
unsafe extern "C-unwind" fn io_proc(
    _device: AudioObjectID,
    _now: NonNull<AudioTimeStamp>,
    input_data: NonNull<AudioBufferList>,
    _input_time: NonNull<AudioTimeStamp>,
    _output_data: NonNull<AudioBufferList>,
    _output_time: NonNull<AudioTimeStamp>,
    context: *mut c_void,
) -> i32 {
    if context.is_null() {
        return kAudioHardwareNoError;
    }
    // **소유권을 가져오지 않는다.** 이 포인터는 `Stopper`가 거둔다.
    let shared = unsafe { &*(context as *const Shared) };

    let list = unsafe { input_data.as_ref() };
    let count = list.mNumberBuffers as usize;
    if count == 0 {
        return kAudioHardwareNoError;
    }

    // 버퍼가 여럿이면 스트림이 여럿이라는 뜻이다 (**[A✓] 마이크 1 + tap 2 = 스트림 2개**).
    // 프레임 수는 같으므로 채널을 이어 붙여 한 프레임으로 만든다.
    let buffers = unsafe {
        std::slice::from_raw_parts(list.mBuffers.as_ptr(), count)
    };

    let mut channels = 0usize;
    let mut frames = 0usize;
    for buffer in buffers {
        let per = buffer.mNumberChannels as usize;
        if per == 0 || buffer.mData.is_null() {
            continue;
        }
        let total = buffer.mDataByteSize as usize / std::mem::size_of::<f32>();
        channels += per;
        frames = frames.max(total / per);
    }
    if channels == 0 || frames == 0 {
        return kAudioHardwareNoError;
    }

    // 인터리브 한 덩어리로 모은다. **할당이 한 번뿐이고 크기를 미리 안다.**
    let mut interleaved = vec![0.0_f32; frames * channels];
    let mut offset = 0usize;
    for buffer in buffers {
        let per = buffer.mNumberChannels as usize;
        if per == 0 || buffer.mData.is_null() {
            continue;
        }
        let samples = unsafe {
            std::slice::from_raw_parts(buffer.mData as *const f32, frames * per)
        };
        for frame in 0..frames {
            for channel in 0..per {
                interleaved[frame * channels + offset + channel] = samples[frame * per + channel];
            }
        }
        offset += per;
    }

    let stereo = fold_to_stereo(&interleaved, channels);
    if shared.sink.try_send(stereo).is_err() {
        // **버린 것을 세어 둔다.** 정지할 때 이 수가 실패로 올라간다.
        shared
            .dropped_frames
            .fetch_add(frames as u64, Ordering::Relaxed);
    }

    kAudioHardwareNoError
}

fn io_proc_failed(message: &str, status: i32) -> Failure {
    Failure::permanent(FailureKind::AudioDevice, message.to_owned())
        .with_detail(format!("Core Audio status={status}"))
}
