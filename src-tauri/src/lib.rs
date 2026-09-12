mod commands;
pub mod infrastructure;
pub mod models;
pub mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use tauri::{Emitter, Manager};

    let shutdown = Arc::new(AtomicBool::new(false));
    let setup_shutdown = shutdown.clone();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            let auth_manager = Arc::new(services::auth::AuthManager::new(
                Arc::new(
                    infrastructure::bilibili::BilibiliAuthClient::new()
                        .map_err(|error| std::io::Error::other(error.message))?,
                ),
                Arc::new(infrastructure::auth::SystemCredentialStore::new()),
                Arc::new(services::auth::ports::SystemAuthClock::default()),
                Arc::new(services::auth::ports::TokioAuthSleeper),
                Arc::new(infrastructure::auth::TauriAuthEventSink::new(
                    app.handle().clone(),
                )),
            ));
            let parser = Arc::new(
                services::ParserService::production(auth_manager.clone())
                    .map_err(|error| std::io::Error::other(error.message))?,
            );
            app.manage(parser.clone());
            app.manage(auth_manager.clone());
            tauri::async_runtime::spawn(async move {
                let _ = auth_manager.restore_on_startup().await;
            });

            let resource_dir = app.path().resource_dir()?;
            // Resolve the install root from the running executable. Tauri's
            // executable_dir resolver can return an unknown path on Windows.
            let install_directory = std::env::current_exe()?
                .parent()
                .map(std::path::Path::to_path_buf)
                .ok_or_else(|| std::io::Error::other("executable has no parent directory"))?;
            let defaults =
                services::settings::SettingsDefaults::from_install_directory(install_directory)
                    .map_err(|error| std::io::Error::other(error.message))?;
            let settings_store = infrastructure::settings::TauriSettingsStore::build(app)
                .map_err(|error| std::io::Error::other(error.message))?;
            let settings_manager = Arc::new(
                services::settings::SettingsManager::load(Arc::new(settings_store), defaults)
                    .map_err(|error| std::io::Error::other(error.message))?,
            );
            app.manage(settings_manager.clone());

            let store_path = app.path().app_data_dir()?.join("tasks.json");
            let task_manager = Arc::new(
                services::tasks::TaskManager::new(
                    Arc::new(infrastructure::tasks::JsonTaskStore::new(store_path)),
                    Arc::new(infrastructure::tasks::TauriTaskEventSink::new(
                        app.handle().clone(),
                    )),
                    Arc::new(infrastructure::tasks::FileTaskCleaner),
                    settings_manager,
                )
                .map_err(|error| std::io::Error::other(error.message))?,
            );
            app.manage(task_manager.clone());

            let show_window = tauri::menu::MenuItem::with_id(
                app,
                "show-window",
                "Show BiliCatch",
                true,
                None::<&str>,
            )?;
            let check_updates = tauri::menu::MenuItem::with_id(
                app,
                "check-updates",
                "Check for updates",
                true,
                None::<&str>,
            )?;
            let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = tauri::menu::MenuBuilder::new(app)
                .items(&[&show_window, &check_updates, &quit])
                .build()?;
            tauri::tray::TrayIconBuilder::new()
                .menu(&tray_menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show-window" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    "check-updates" => {
                        let _ = app.emit("system:update-check-requested", ());
                    }
                    _ => {}
                })
                .build(app)?;

            let workspace: Arc<dyn infrastructure::audio::AudioWorkspace> =
                Arc::new(infrastructure::audio::FsAudioWorkspace);
            let downloader = Arc::new(
                infrastructure::download::HttpByteDownloader::new(workspace.clone())
                    .map_err(|error| std::io::Error::other(error.message))?,
            );
            let ffmpeg_path = [
                resource_dir.join("ffmpeg.exe"),
                resource_dir.join("ffmpeg-x86_64-pc-windows-msvc.exe"),
                resource_dir
                    .join("binaries")
                    .join("ffmpeg-x86_64-pc-windows-msvc.exe"),
            ]
            .into_iter()
            .find(|candidate| candidate.is_file())
            .unwrap_or_else(|| resource_dir.join("ffmpeg.exe"));
            let audio_executor = Arc::new(services::audio::AudioExecutor::new(
                parser.clone(),
                downloader.clone(),
                downloader.clone(),
                workspace,
                Arc::new(infrastructure::audio::DeferredFfmpegMediaProcessor::new(
                    ffmpeg_path.clone(),
                )),
                task_manager.clone(),
            ));
            let video_workspace: Arc<dyn infrastructure::video::VideoWorkspace> =
                Arc::new(infrastructure::video::FsVideoWorkspace);
            let video_executor = Arc::new(services::video::VideoExecutor::new(
                parser,
                downloader.clone(),
                video_workspace,
                Arc::new(infrastructure::video::DeferredFfmpegVideoMuxer::new(
                    ffmpeg_path,
                )),
                task_manager.clone(),
            ));
            let executor = Arc::new(services::download_runtime::StrategyRegistry::new(
                audio_executor,
                video_executor,
            ));
            let runner = Arc::new(services::download_runtime::DownloadRuntimeRunner::new(
                task_manager,
                executor,
            ));
            let runtime_shutdown = setup_shutdown.clone();
            tauri::async_runtime::spawn(async move {
                while !runtime_shutdown.load(Ordering::Acquire) {
                    runner.tick().await;
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::health_check,
            commands::parse_video,
            commands::auth::get_auth_snapshot,
            commands::auth::start_qr_login,
            commands::auth::cancel_qr_login,
            commands::auth::logout,
            commands::settings::get_settings_snapshot,
            commands::settings::update_setting,
            commands::tasks::list_download_tasks,
            commands::tasks::create_download_tasks,
            commands::tasks::control_download_task,
            commands::tasks::pause_all_download_tasks,
            commands::tasks::clear_completed_tasks,
            commands::tasks::open_download_task
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    app.run(move |handle, event| {
        if let tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } = &event
        {
            if label == "main" {
                let minimize = handle
                    .try_state::<Arc<services::settings::SettingsManager>>()
                    .map(|state| {
                        state.snapshot().values.close_behavior
                            == models::CloseBehavior::MinimizeToTray
                    })
                    .unwrap_or(false);
                let has_active_tasks = handle
                    .try_state::<Arc<services::tasks::TaskManager>>()
                    .map(|state| {
                        state.list().tasks.iter().any(|task| {
                            matches!(
                                task.status,
                                models::TaskStatus::Downloading | models::TaskStatus::Processing
                            )
                        })
                    })
                    .unwrap_or(false);
                match services::system::WindowDecision::from_settings(minimize, has_active_tasks) {
                    services::system::WindowDecision::MinimizeToTray => {
                        api.prevent_close();
                        if let Some(window) = handle.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    services::system::WindowDecision::ConfirmExit => {
                        api.prevent_close();
                        let _ = handle.emit("system:close-confirmation-required", ());
                    }
                    services::system::WindowDecision::Exit => {}
                }
            }
        }
        if matches!(event, tauri::RunEvent::Exit) {
            shutdown.store(true, Ordering::Release);
        }
    });
}
