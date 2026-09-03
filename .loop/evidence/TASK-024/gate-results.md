# TASK-024 — Gate 실행 결과

명령: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
실행 위치: `/Users/molt/orca/projects/molt-note`
일시: 2026-09-03

> 이것은 Worker의 self-check이며 **완료 판정이 아니다.** Runtime이 Worker 종료 후 Gate를
> 독립적으로 다시 돌린다.

## 요약

| Gate | 명령 | exit | 소요 |
| --- | --- | --- | --- |
| build | `npm run build` (`tsc && vite build`) | **0** | 0.9s |
| lint | `npm run lint` (`eslint .` + `cargo clippy --all-targets -- -D warnings`) | **0** | 2.1s |
| test | `npm run test` (`vitest run` + `cargo test`) | **0** | 9.8s |

`Self-check: all gates passed`

## build

```text
> molt-note@0.1.0 build
> tsc && vite build

vite v7.3.6 building client environment for production...
transforming...
✓ 46 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                   0.40 kB │ gzip:  0.26 kB
dist/assets/index-BYKmgwJd.css    4.18 kB │ gzip:  1.30 kB
dist/assets/index-BueuPG8k.js   212.63 kB │ gzip: 66.17 kB
✓ built in 272ms
```

stderr: 비어 있다.

## lint

```text
> molt-note@0.1.0 lint:web
> eslint .

> molt-note@0.1.0 lint:rust
> cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

stderr 전체:

```text
    Checking molt-note v0.1.0 (/Users/molt/orca/projects/molt-note/src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.97s
```

경고 없음. `-D warnings`이므로 clippy 경고가 하나라도 있으면 exit != 0이 된다.

## test

```text
> molt-note@0.1.0 test:web
> vitest run

 RUN  v4.1.11 /Users/molt/orca/projects/molt-note

 Test Files  14 passed (14)
      Tests  154 passed (154)

> molt-note@0.1.0 test:rust
> cargo test --manifest-path src-tauri/Cargo.toml

running 125 tests
...
test result: ok. 125 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Rust lib 테스트는 이 Task 이전 104개에서 **125개**가 됐다 (신규 21개).

### 이 Task가 추가한 테스트 (전부 `ok`)

```text
test transcription::audio_input::tests::a_device_native_recording_becomes_the_16khz_mono_buffer_whisper_asks_for ... ok
test transcription::audio_input::tests::one_second_of_audio_is_still_one_second_after_conversion ... ok
test transcription::audio_input::tests::downmixing_stereo_keeps_both_channels_rather_than_picking_one ... ok
test transcription::audio_input::tests::a_silent_channel_does_not_disappear_into_the_other_one ... ok
test transcription::audio_input::tests::an_input_that_is_already_16khz_mono_passes_through_untouched ... ok
test transcription::audio_input::tests::resampling_carries_the_signal_rather_than_emitting_silence ... ok
test transcription::audio_input::tests::a_sound_stays_where_it_was_in_time_after_resampling ... ok
test transcription::audio_input::tests::a_recording_shorter_than_one_resampler_chunk_still_converts ... ok
test transcription::audio_input::tests::the_source_file_is_byte_for_byte_identical_after_conversion ... ok
test transcription::audio_input::tests::conversion_leaves_no_derived_file_anywhere_near_the_source ... ok
test transcription::audio_input::tests::an_empty_file_is_a_defined_failure_rather_than_a_panic ... ok
test transcription::audio_input::tests::a_wav_with_a_header_but_no_sound_is_a_defined_failure ... ok
test transcription::audio_input::tests::a_truncated_file_is_a_defined_failure_rather_than_a_panic ... ok
test transcription::audio_input::tests::a_file_that_is_not_wav_at_all_is_a_defined_failure ... ok
test transcription::audio_input::tests::a_missing_file_is_a_defined_failure_that_names_the_path ... ok
test transcription::audio_input::tests::an_unsupported_sample_format_is_refused_instead_of_being_guessed_at ... ok
test transcription::audio_input::tests::an_unsupported_bit_depth_is_refused ... ok
test transcription::audio_input::tests::a_channel_count_we_cannot_downmix_is_refused_rather_than_averaged_blindly ... ok
test transcription::audio_input::tests::every_malformed_input_returns_a_failure_and_never_panics ... ok
test transcription::audio_input::tests::the_expected_frame_count_follows_the_ratio_without_drifting ... ok
test transcription::audio_input::tests::the_resampler_reports_the_delay_this_module_compensates_for ... ok
```
