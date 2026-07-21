use simplelog::*;
use std::fs::File;

#[uniffi::export]
fn configure_logging(log_path: String) -> Result<(), EngineError> {
    let log_file = File::create(&log_path)
        .map_err(|e| EngineError::TlsSetupError(format!("failed to create log file: {e}")))?;

    WriteLogger::init(LevelFilter::Debug, Config::default(), log_file)
        .map_err(|e| EngineError::TlsSetupError(format!("failed to init logger: {e}")))?;

    log::info!("Logging initialized at {log_path}");
    Ok(())
}