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
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub struct TrayHandles {
    pub pause_item: MenuItem<tauri::Wry>,
}

pub fn sync_tray_pause_label(app: &AppHandle, paused: bool) {
    if let Some(handles) = app.try_state::<TrayHandles>() {
        let label = if paused {
            "恢复定时提醒"
        } else {
            "暂停定时提醒"
        };
        let _ = handles.pause_item.set_text(label);
    }
}

pub fn apply_global_shortcut(app: &AppHandle, shortcut: &str) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let trimmed = shortcut.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    gs.register(trimmed)
        .map_err(|e| format!("注册全局快捷键「{trimmed}」失败（可能被其他程序占用）: {e}"))?;
    Ok(())
}

pub fn open_or_focus_dashboard(app: &AppHandle, initial_tab: Option<&str>) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        if let Some(tab) = initial_tab {
            let _ = win.emit("nav:tab", tab);
        }
        return;
    }

    let url = match initial_tab {
        Some(tab) => format!("index.html?tab={tab}"),
        None => "index.html".to_string(),
    };

    if let Ok(win) = WebviewWindowBuilder::new(app, "main", WebviewUrl::App(url.into()))
        .title("QuizPulse · 定时答题助手")
        .inner_size(1100.0, 740.0)
        .min_inner_size(900.0, 620.0)
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
    let (initial_settings, initial_progress) = {
        let conn = db.conn.lock().unwrap();
        let dir = db.data_dir.lock().unwrap();
        let settings = db::load_settings(&conn, &dir).unwrap_or_default();
        let rp = db::get_round_progress(&conn).unwrap_or(models::RoundProgress {
            current_round: 1,
            completed_in_round: 0,
            total_active: 0,
        });
        (settings, rp)
    };
    let initial_shortcut = initial_settings.global_shortcut.clone();
    let scheduler = SchedulerState::new(initial_settings, initial_progress);

    let launched_silent = std::env::args().any(|a| a == "--silent");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            open_or_focus_dashboard(app, None);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--silent"]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let db = app.state::<DbState>();
                        let sched = app.state::<SchedulerState>();
                        let _ = sched.trigger_popup_now(app, db.inner());
                    }
                })
                .build(),
        )
        .manage(db.clone())
        .manage(scheduler.clone())
        .setup(move |app| {
            // Register configured global hotkey (non-fatal if occupied)
            let _ = apply_global_shortcut(app.handle(), &initial_shortcut);

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

            // Build intelligent system tray menu
            let item_open =
                MenuItem::with_id(app, "open_dashboard", "打开管理面板", true, None::<&str>)?;
            let item_now =
                MenuItem::with_id(app, "trigger_now", "立即抽题练习", true, None::<&str>)?;
            let item_focus =
                MenuItem::with_id(app, "open_focus", "进入专注刷题模式", true, None::<&str>)?;
            let item_pause =
                MenuItem::with_id(app, "toggle_pause", "暂停定时提醒", true, None::<&str>)?;
            let item_dnd =
                MenuItem::with_id(app, "dnd_60", "一键勿扰 1 小时", true, None::<&str>)?;
            let item_quit =
                MenuItem::with_id(app, "quit", "彻底退出 QuizPulse", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &item_open,
                    &item_now,
                    &item_focus,
                    &item_pause,
                    &item_dnd,
                    &item_quit,
                ],
            )?;

            app.manage(TrayHandles {
                pause_item: item_pause,
            });

            let db_for_menu = db.clone();
            let sched_for_menu = scheduler.clone();
            let mut tray_builder = TrayIconBuilder::with_id("main_tray")
                .menu(&menu)
                .tooltip("QuizPulse · 定时答题助手")
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "open_dashboard" => open_or_focus_dashboard(app, None),
                    "open_focus" => open_or_focus_dashboard(app, Some("focus")),
                    "trigger_now" => {
                        let _ = sched_for_menu.trigger_popup_now(app, &db_for_menu);
                    }
                    "toggle_pause" => {
                        let _ = sched_for_menu.toggle_manual_pause(app);
                    }
                    "dnd_60" => {
                        sched_for_menu.snooze_or_dnd(app, 60);
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
                        open_or_focus_dashboard(tray.app_handle(), None);
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            tray_builder.build(app)?;

            scheduler.update_tray_tooltip(app.handle());
            scheduler.start_background_loop(app.handle().clone(), db.clone());

            if !launched_silent {
                open_or_focus_dashboard(app.handle(), None);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_stats,
            commands::get_activity_heatmap,
            commands::list_banks,
            commands::create_bank,
            commands::toggle_bank,
            commands::delete_bank,
            commands::list_questions,
            commands::upsert_question,
            commands::delete_question,
            commands::set_question_status,
            commands::skip_question,
            commands::start_new_round,
            commands::pick_focus_questions,
            commands::submit_answer,
            commands::manual_override_pass,
            commands::export_excel_template,
            commands::import_excel_file,
            commands::get_settings,
            commands::save_settings,
            commands::get_timer_status,
            commands::get_current_popup_session,
            commands::toggle_timer_pause,
            commands::snooze_dnd,
            commands::trigger_popup_now,
            commands::finish_popup,
            commands::reposition_popup,
        ])
        .run(tauri::generate_context!())
        .expect("QuizPulse 启动异常");
}
