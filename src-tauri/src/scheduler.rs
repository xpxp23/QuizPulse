use crate::db::{get_round_progress, pick_session_questions, DbState};
use crate::models::{AppSettings, QuizSessionQuestion, RoundProgress, TimerStatus};
use crate::system::{get_idle_seconds, is_fullscreen_or_locked};
use std::sync::{Arc, Mutex, RwLock};
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
    /// In-memory cached settings so the 1-second loop NEVER locks SQLite!
    pub settings: Arc<RwLock<AppSettings>>,
    /// In-memory cached round progress for live tray tooltip & UI without polling DB
    pub round_progress: Arc<RwLock<RoundProgress>>,
}

impl SchedulerState {
    pub fn new(initial_settings: AppSettings, initial_progress: RoundProgress) -> Self {
        let total = (initial_settings.interval_minutes.max(1) as u64) * 60;
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
            settings: Arc::new(RwLock::new(initial_settings)),
            round_progress: Arc::new(RwLock::new(initial_progress)),
        }
    }

    pub fn get_status(&self) -> TimerStatus {
        let guard = self.inner.lock().unwrap();
        let rp = self.round_progress.read().unwrap().clone();
        TimerStatus {
            remaining_secs: guard.remaining_secs,
            total_secs: guard.total_secs,
            paused_manual: guard.paused_manual,
            paused_auto_reason: guard.paused_auto_reason.clone(),
            popup_open: guard.popup_open,
            round_progress: rp,
        }
    }

    pub fn get_cached_settings(&self) -> AppSettings {
        self.settings.read().unwrap().clone()
    }

    pub fn update_cached_settings(&self, new_settings: AppSettings) {
        let old_interval = self.settings.read().unwrap().interval_minutes;
        let new_interval = new_settings.interval_minutes.max(1);
        *self.settings.write().unwrap() = new_settings;

        if new_interval != old_interval {
            let total = (new_interval as u64) * 60;
            let mut guard = self.inner.lock().unwrap();
            guard.total_secs = total;
            guard.remaining_secs = total;
        }
    }

    pub fn update_round_progress(&self, rp: RoundProgress) {
        *self.round_progress.write().unwrap() = rp;
    }

    pub fn refresh_round_progress_from_db(&self, db: &DbState) {
        if let Ok(conn) = db.conn.lock() {
            if let Ok(rp) = get_round_progress(&conn) {
                self.update_round_progress(rp);
            }
        }
    }

    pub fn get_current_session(&self) -> Vec<QuizSessionQuestion> {
        self.inner.lock().unwrap().current_session.clone()
    }

    pub fn toggle_manual_pause(&self, app: &AppHandle) -> bool {
        let paused = {
            let mut guard = self.inner.lock().unwrap();
            guard.paused_manual = !guard.paused_manual;
            guard.paused_manual
        };
        crate::sync_tray_pause_label(app, paused);
        self.update_tray_tooltip(app);
        let _ = app.emit("timer:tick", self.get_status());
        paused
    }

    pub fn snooze_or_dnd(&self, app: &AppHandle, minutes: u32) {
        {
            let mut guard = self.inner.lock().unwrap();
            guard.paused_manual = false;
            guard.remaining_secs = (minutes.max(1) as u64) * 60;
        }
        crate::sync_tray_pause_label(app, false);
        self.update_tray_tooltip(app);
        let _ = app.emit("timer:tick", self.get_status());
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
        self.update_tray_tooltip(app);
        let _ = app.emit("timer:tick", self.get_status());
    }

    pub fn reposition_popup(&self, app: &AppHandle) {
        let pos_pref = self.settings.read().unwrap().popup_position.clone();
        if let Some(popup) = app.get_webview_window("popup") {
            position_popup_window(&popup, &pos_pref);
        }
    }

    pub fn trigger_popup_now(&self, app: &AppHandle, db: &DbState) -> Result<bool, String> {
        {
            let guard = self.inner.lock().unwrap();
            if guard.popup_open {
                if let Some(popup) = app.get_webview_window("popup") {
                    let _ = popup.show();
                    let _ = popup.set_focus();
                }
                return Ok(true);
            }
        }

        let settings = self.get_cached_settings();
        let (session, rp) = {
            let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
            let session = pick_session_questions(
                &conn,
                settings.questions_per_session as usize,
                &settings,
                None,
                false,
            )?;
            let rp = get_round_progress(&conn)?;
            (session, rp)
        };
        self.update_round_progress(rp);

        if session.is_empty() {
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

        self.update_tray_tooltip(app);
        let _ = app.emit("timer:tick", self.get_status());
        Ok(true)
    }

    pub fn update_tray_tooltip(&self, app: &AppHandle) {
        let Some(tray) = app.tray_by_id("main_tray") else {
            return;
        };
        let st = self.get_status();
        let mins = st.remaining_secs / 60;
        let secs = st.remaining_secs % 60;
        let round_str = if st.round_progress.total_active > 0 {
            format!(
                " · 第{}轮 {}/{}",
                st.round_progress.current_round,
                st.round_progress.completed_in_round,
                st.round_progress.total_active
            )
        } else {
            String::new()
        };

        let text = if st.popup_open {
            format!("QuizPulse · 答题中{round_str}")
        } else if st.paused_manual {
            format!("QuizPulse · 已暂停定时{round_str}")
        } else if st.paused_auto_reason.as_deref() == Some("fullscreen") {
            format!("QuizPulse · 全屏勿扰暂停中{round_str}")
        } else if st.paused_auto_reason.as_deref() == Some("idle") {
            format!("QuizPulse · 闲置自动暂停中{round_str}")
        } else {
            format!("QuizPulse · 距下次抽题 {:02}:{:02}{round_str}", mins, secs)
        };
        let _ = tray.set_tooltip(Some(text));
    }

    pub fn start_background_loop(&self, app: AppHandle, db: DbState) {
        let scheduler = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut ticker = interval(Duration::from_secs(1));
            let mut tick_count: u64 = 0;

            loop {
                ticker.tick().await;
                tick_count = tick_count.wrapping_add(1);

                // Pure RAM read — ZERO SQLite lock or JSON parse per second!
                let (fullscreen_dnd, idle_limit_secs) = {
                    let s = scheduler.settings.read().unwrap();
                    (s.fullscreen_dnd, (s.idle_pause_minutes as u64) * 60)
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

                // Update tray tooltip every 5 seconds to keep overhead near zero
                if tick_count % 5 == 0 {
                    scheduler.update_tray_tooltip(&app);
                }

                // Emit status only if dashboard window is open
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

/// DPI-aware & taskbar-safe popup positioning using monitor.work_area()
pub fn position_popup_window(popup: &tauri::WebviewWindow, position_pref: &str) {
    if position_pref == "center" {
        let _ = popup.center();
        return;
    }

    if let Ok(Some(monitor)) = popup.current_monitor() {
        let work = monitor.work_area();
        let scale = monitor.scale_factor();
        let margin = (18.0 * scale) as i32;
        if let Ok(win_size) = popup.outer_size() {
            let x = work.position.x + (work.size.width as i32) - (win_size.width as i32) - margin;
            let y =
                work.position.y + (work.size.height as i32) - (win_size.height as i32) - margin;
            let min_x = work.position.x + margin;
            let min_y = work.position.y + margin;
            let _ = popup.set_position(PhysicalPosition::new(x.max(min_x), y.max(min_y)));
            return;
        }
    }
    let _ = popup.center();
}
