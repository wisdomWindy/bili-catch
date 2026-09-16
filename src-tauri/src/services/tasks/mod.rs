mod coalescing;
mod filename;
mod manager;
mod mutations;
mod notifications;
mod ports;
mod state_machine;
mod validation;

pub use filename::{sanitize_audio_filename, sanitize_filename};
pub use manager::TaskManager;
pub use mutations::ExecutionUpdate;
pub use notifications::{
    CompletionNotificationSettings, CompletionNotificationSettingsPort,
    CompletionNotifyingTaskEventSink, TaskCompletionNotifier,
};
pub use ports::{
    DeferredTaskExecutor, ExecutionControlSpec, SchedulerLimits, TaskCleanerPort, TaskEventSink,
    TaskExecutionSpec, TaskExecutorPort, TaskSettingsPort, TaskStorePort,
};
pub use state_machine::actions_for_status;
pub use validation::{validate_create_request, ValidatedCreateRequest};
