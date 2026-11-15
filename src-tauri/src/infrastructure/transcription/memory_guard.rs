use crate::models::{AppError, AppErrorCode, AsrModelId};

/// 各模型运行时的保守内存下限（字节）。
///
/// 数值取「模型权重 + 推理峰值工作区」的保守估计，宁大勿小：
/// 低估会让用户在跑了一半时 OOM，高估只会让极端小内存机器提前拒绝。
pub fn required_memory_bytes(model: AsrModelId) -> u64 {
    const GIB: u64 = 1024 * 1024 * 1024;
    match model {
        AsrModelId::Tiny => GIB / 2,
        AsrModelId::Base => GIB,
        AsrModelId::Small => 2 * GIB,
        AsrModelId::Medium => 3 * GIB,
        AsrModelId::LargeV3 => 4 * GIB,
        AsrModelId::FireRedAsr2 => 2 * GIB,
        AsrModelId::FireRedAsr2Aed => 2 * GIB,
        AsrModelId::FireRedAsr2Llm => 9 * GIB,
        AsrModelId::Qwen3Asr => 3 * GIB,
    }
}

/// 当前可用物理内存。无法探测时返回 `None`，调用方应放行而不是拦截。
pub fn available_memory_bytes() -> Option<u64> {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let available = system.available_memory();
    (available > 0).then_some(available)
}

/// 纯函数形式的容量判定，便于测试。
pub fn check_capacity(required: u64, available: Option<u64>) -> Result<(), AppError> {
    match available {
        Some(available) if available < required => Err(AppError {
            code: AppErrorCode::E018,
            message: "Not enough memory to load the selected model".into(),
            details: Some("MODEL_MEMORY_INSUFFICIENT".into()),
        }),
        _ => Ok(()),
    }
}

pub fn ensure_capacity(model: AsrModelId) -> Result<(), AppError> {
    check_capacity(required_memory_bytes(model), available_memory_bytes())
}

#[cfg(test)]
mod tests {
    use super::{check_capacity, required_memory_bytes};
    use crate::models::{AppErrorCode, AsrModelId};

    #[test]
    fn insufficient_memory_is_rejected_with_a_retryable_code() {
        let error = check_capacity(4 << 30, Some(1 << 30)).unwrap_err();

        assert_eq!(error.code, AppErrorCode::E018);
        assert_eq!(error.details.as_deref(), Some("MODEL_MEMORY_INSUFFICIENT"));
    }

    #[test]
    fn unknown_available_memory_never_blocks_the_user() {
        assert!(check_capacity(8 << 30, None).is_ok());
        assert!(check_capacity(1 << 30, Some(2 << 30)).is_ok());
    }

    #[test]
    fn high_accuracy_models_require_more_than_draft_models() {
        assert!(
            required_memory_bytes(AsrModelId::LargeV3)
                > required_memory_bytes(AsrModelId::Tiny)
        );
        assert!(required_memory_bytes(AsrModelId::Qwen3Asr) >= 3 * 1024 * 1024 * 1024);
    }
}
