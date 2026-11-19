use crate::db::{load_settings, pick_session_questions, DbState};
use crate::models::{QuizSessionQuestion, TimerStatus};
use crate::system::{get_idle_seconds, is_fullscreen_or_locked};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use tokio::time::{interval, Duration};

#[derive(Debug, Clone)]
pub struct TimerInner {
    pub remaining_secs: u64,
    pub total_secs: u64,
    pub paused_manual: bool,
    pub paused_auto_reason: Option<String>,
    pub popup_open: bool,
    pub was_away: bool,
    pub current_session: Vec<QuizSessionQuestion>,
}

#[derive(Clone)]
pub struct SchedulerState {
    pub inner: Arc<Mutex<TimerInner>>,
}

impl SchedulerState {
    pub fn new(interval_minutes: u32) -> Self {
        let total = (interval_minutes.max(1) as u64) * 60;
        Self {
            inner: Arc::new(Mutex::new(TimerInner {
                remaining_secs: total,
                total_secs: total,
                paused_manual: false,
                paused_auto_reason: None,
                popup_open: false,
                was_away: false,
                current_session: Vec::new(),
            })),
        }
    }

    pub fn get_status(&self) -> TimerStatus {
        let guard = self.inner.lock().unwrap();
        TimerStatus {
            remaining_secs: guard.remaining_secs,
            total_secs: guard.total_secs,
            paused_manual: guard.paused_manual,
            paused_auto_reason: guard.paused_auto_reason.clone(),
            popup_open: guard.popup_open,
        }
    }

    pub fn get_current_session(&self) -> Vec<QuizSessionQuestion> {
        self.inner.lock().unwrap().current_session.clone()
    }

    pub fn toggle_manual_pause(&self) -> bool {
        let mut guard = self.inner.lock().unwrap();
        guard.paused_manual = !guard.paused_manual;
        guard.paused_manual
    }

    pub fn update_interval(&self, interval_minutes: u32) {
        let total = (interval_minutes.max(1) as u64) * 60;
        let mut guard = self.inner.lock().unwrap();
        guard.total_secs = total;
        if guard.remaining_secs > total {
            guard.remaining_secs = total;
        }
    }

    pub fn finish_popup(&self, app: &AppHandle, snooze_minutes: Option<u32>) {
        {
            let mut guard = self.inner.lock().unwrap();
            guard.popup_open = false;
            guard.current_session.clear();
            if let Some(mins) = snooze_minutes {
                guard.remaining_secs = (mins.max(1) as u64) * 60;
            } else {
                guard.remaining_secs = guard.total_secs;
            }
        }
        if let Some(popup) = app.get_webview_window("popup") {
            let _ = popup.hide();
        }
        let _ = app.emit("timer:tick", self.get_status());
    }

    pub fn trigger_popup_now(&self, app: &AppHandle, db: &DbState) -> Result<bool, String> {
        {
            let guard = self.inner.lock().unwrap();
            if guard.popup_open {
                // Never stack popups; just focus the already open one
                if let Some(popup) = app.get_webview_window("popup") {
                    let _ = popup.show();
                    let _ = popup.set_focus();
                }
                return Ok(true);
            }
        }

        let (settings, session) = {
            let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
            let dir = db.data_dir.lock().map_err(|_| "目录锁获取失败".to_string())?;
            let settings = load_settings(&conn, &dir)?;
            let session = pick_session_questions(
                &conn,
                settings.questions_per_session as usize,
                settings.shuffle_options,
            )?;
            (settings, session)
        };

        if session.is_empty() {
            // No active questions left; reset timer so we don't busy-loop
            let mut guard = self.inner.lock().unwrap();
            guard.remaining_secs = guard.total_secs;
            return Ok(false);
        }

        {
            let mut guard = self.inner.lock().unwrap();
            guard.popup_open = true;
            guard.current_session = session.clone();
            guard.paused_auto_reason = Some("popup_open".into());
        }

        if let Some(popup) = app.get_webview_window("popup") {
            position_popup_window(&popup, &settings.popup_position);
            let _ = popup.emit("quiz:start", &session);
            let _ = popup.show();
            let _ = popup.unminimize();
            let _ = popup.set_focus();
        }

        let _ = app.emit("timer:tick", self.get_status());
        Ok(true)
    }

    pub fn start_background_loop(&self, app: AppHandle, db: DbState) {
        let scheduler = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut ticker = interval(Duration::from_secs(1));
            loop {
                ticker.tick().await;

                // Read lightweight settings snapshot
                let (fullscreen_dnd, idle_limit_secs) = {
                    let conn = match db.conn.lock() {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    let dir = match db.data_dir.lock() {
                        Ok(d) => d,
                        Err(_) => continue,
                    };
                    match load_settings(&conn, &dir) {
                        Ok(s) => (s.fullscreen_dnd, (s.idle_pause_minutes as u64) * 60),
                        Err(_) => (true, 600),
                    }
                };

                let mut should_trigger = false;
                {
                    let mut guard = scheduler.inner.lock().unwrap();
                    if guard.popup_open {
                        guard.paused_auto_reason = Some("popup_open".into());
                    } else if guard.paused_manual {
                        guard.paused_auto_reason = None;
                    } else {
                        let fs_or_locked = fullscreen_dnd && is_fullscreen_or_locked();
                        let idle_secs = if idle_limit_secs > 0 {
                            get_idle_seconds()
                        } else {
                            0
                        };
                        let is_idle = idle_limit_secs > 0 && idle_secs >= idle_limit_secs;

                        if fs_or_locked {
                            guard.paused_auto_reason = Some("fullscreen".into());
                            guard.was_away = true;
                        } else if is_idle {
                            guard.paused_auto_reason = Some("idle".into());
                            guard.was_away = true;
                        } else {
                            // User returned from away/lock/fullscreen -> apply 2-min return buffer
                            if guard.was_away {
                                guard.was_away = false;
                                if guard.remaining_secs < 120 {
                                    guard.remaining_secs = 120;
                                }
                            }
                            guard.paused_auto_reason = None;
                            if guard.remaining_secs > 0 {
                                guard.remaining_secs -= 1;
                            }
                            if guard.remaining_secs == 0 {
                                should_trigger = true;
                            }
                        }
                    }
                }

                // Emit status only if dashboard window is actually open (saves IPC overhead when minimized!)
                if app.get_webview_window("main").is_some() {
                    let _ = app.emit("timer:tick", scheduler.get_status());
                }

                if should_trigger {
                    let _ = scheduler.trigger_popup_now(&app, &db);
                }
            }
        });
    }
}

fn position_popup_window(popup: &tauri::WebviewWindow, position_pref: &str) {
    if position_pref == "center" {
        let _ = popup.center();
        return;
    }

    // Default: bottom_right with comfortable margin above taskbar
    if let Ok(Some(monitor)) = popup.current_monitor() {
        let screen_size = monitor.size();
        let screen_pos = monitor.position();
        if let Ok(win_size) = popup.outer_size() {
            let margin_x = 24i32;
            let margin_y = 68i32; // Clear Windows taskbar cleanly
            let x = screen_pos.x + (screen_size.width as i32) - (win_size.width as i32) - margin_x;
            let y = screen_pos.y + (screen_size.height as i32) - (win_size.height as i32) - margin_y;
            let _ = popup.set_position(PhysicalPosition::new(x.max(0), y.max(0)));
            return;
        }
    }
    let _ = popup.center();
}
