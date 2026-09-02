//! 플랫폼 지식이 갇혀 있는 경계 (PRODUCT-SPEC §3.1 · INV-10).
//!
//! OS별 경로 규약이나 `cfg(target_os)` 분기는 이 모듈 안에만 존재한다.
//! 나머지 코드는 여기서 노출되는 개념(예: 앱 데이터 디렉터리)만 안다.

pub mod app_data_dir;
