mod commands;
mod db;
mod excel;
mod grading;
mod models;
mod scheduler;
mod system;

use db::DbState;
use scheduler::SchedulerState;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

pub fn open_or_focus_dashboard(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        return;
    }

    if let Ok(win) = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("QuizPulse · 定时答题助手")
        .inner_size(1060.0, 720.0)
        .min_inner_size(880.0, 600.0)
        .center()
        .visible(true)
        .build()
    {
        #[cfg(target_os = "windows")]
        {
            let _ = window_vibrancy::apply_mica(&win, None);
        }
        let _ = win.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = DbState::init().expect("QuizPulse 数据库初始化失败");
    let initial_interval = {
        let conn = db.conn.lock().unwrap();
        let dir = db.data_dir.lock().unwrap();
        db::load_settings(&conn, &dir)
            .map(|s| s.interval_minutes)
            .unwrap_or(40)
    };
    let scheduler = SchedulerState::new(initial_interval);

    let launched_silent = std::env::args().any(|a| a == "--silent");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            open_or_focus_dashboard(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--silent"]),
        ))
        .manage(db.clone())
        .manage(scheduler.clone())
        .setup(move |app| {
            // Keep preloaded popup window hidden on close instead of destroying it (<50ms wake)
            if let Some(popup) = app.get_webview_window("popup") {
                let app_handle = app.handle().clone();
                let sched_clone = scheduler.clone();
                popup.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        sched_clone.finish_popup(&app_handle, None);
                    }
                });
            }

            // Build system tray menu
            let item_open = MenuItem::with_id(app, "open_dashboard", "打开管理面板", true, None::<&str>)?;
            let item_now = MenuItem::with_id(app, "trigger_now", "立即抽题练习", true, None::<&str>)?;
            let item_pause = MenuItem::with_id(app, "toggle_pause", "暂停 / 恢复定时", true, None::<&str>)?;
            let item_quit = MenuItem::with_id(app, "quit", "彻底退出 QuizPulse", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&item_open, &item_now, &item_pause, &item_quit])?;

            let db_for_menu = db.clone();
            let sched_for_menu = scheduler.clone();
            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("QuizPulse · 定时答题助手")
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "open_dashboard" => open_or_focus_dashboard(app),
                    "trigger_now" => {
                        let _ = sched_for_menu.trigger_popup_now(app, &db_for_menu);
                    }
                    "toggle_pause" => {
                        let _ = sched_for_menu.toggle_manual_pause();
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        open_or_focus_dashboard(tray.app_handle());
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            tray_builder.build(app)?;

            // Start 1-second background scheduler loop
            scheduler.start_background_loop(app.handle().clone(), db.clone());

            // Open dashboard on manual launch; stay silent in tray when launched with --silent on boot
            if !launched_silent {
                open_or_focus_dashboard(app.handle());
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_stats,
            commands::list_banks,
            commands::create_bank,
            commands::toggle_bank,
            commands::delete_bank,
            commands::list_questions,
            commands::delete_question,
            commands::set_question_status,
            commands::submit_answer,
            commands::manual_override_pass,
            commands::export_excel_template,
            commands::import_excel_file,
            commands::get_settings,
            commands::save_settings,
            commands::get_timer_status,
            commands::get_current_popup_session,
            commands::toggle_timer_pause,
            commands::trigger_popup_now,
            commands::finish_popup,
        ])
        .run(tauri::generate_context!())
        .expect("QuizPulse 启动异常");
}
