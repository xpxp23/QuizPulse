use crate::db::{
    self, DbState,
};
use crate::excel;
use crate::grading;
use crate::models::{
    AppSettings, Bank, DashboardStats, GradeRequest, GradeResult, ImportSummary, Question,
    QuizSessionQuestion, TimerStatus,
};
use crate::scheduler::SchedulerState;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub async fn get_dashboard_stats(db: State<'_, DbState>) -> Result<DashboardStats, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::get_dashboard_stats(&conn)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_banks(db: State<'_, DbState>) -> Result<Vec<Bank>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::list_banks(&conn)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_bank(
    db: State<'_, DbState>,
    name: String,
    description: String,
) -> Result<Bank, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::create_bank(&conn, &name, &description)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn toggle_bank(
    db: State<'_, DbState>,
    bank_id: String,
    enabled: bool,
) -> Result<(), String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::toggle_bank(&conn, &bank_id, enabled)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_bank(db: State<'_, DbState>, bank_id: String) -> Result<(), String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::delete_bank(&conn, &bank_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_questions(
    db: State<'_, DbState>,
    bank_id: Option<String>,
    status: Option<String>,
    keyword: Option<String>,
) -> Result<Vec<Question>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::list_questions(&conn, bank_id.as_deref(), status.as_deref(), keyword.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_question(db: State<'_, DbState>, question_id: String) -> Result<(), String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::delete_question(&conn, &question_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_question_status(
    db: State<'_, DbState>,
    question_id: String,
    status: String,
) -> Result<(), String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::set_question_status(&conn, &question_id, &status)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn submit_answer(
    db: State<'_, DbState>,
    req: GradeRequest,
) -> Result<GradeResult, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        let dir = db.data_dir.lock().map_err(|_| "目录锁获取失败".to_string())?;
        let settings = db::load_settings(&conn, &dir)?;
        let question = db::get_question_by_id(&conn, &req.question_id)?;

        let outcome = grading::evaluate_question(
            &question,
            &req.user_answers,
            settings.short_answer_pass_percent,
        );

        let (auto_cleared, next_review_at, ebbinghaus_stage, streak) = db::record_review_outcome(
            &conn,
            &question.id,
            outcome.is_correct,
            outcome.score_percent,
            false,
            settings.auto_clear_streak,
        )?;

        Ok(GradeResult {
            question_id: question.id,
            is_correct: outcome.is_correct,
            score_percent: outcome.score_percent,
            earned_points: outcome.earned_points,
            total_points: outcome.total_points,
            keyword_details: outcome.keyword_details,
            blank_results: outcome.blank_results,
            auto_cleared,
            next_review_at,
            ebbinghaus_stage,
            streak,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn manual_override_pass(
    db: State<'_, DbState>,
    question_id: String,
) -> Result<GradeResult, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        let dir = db.data_dir.lock().map_err(|_| "目录锁获取失败".to_string())?;
        let settings = db::load_settings(&conn, &dir)?;

        let (auto_cleared, next_review_at, ebbinghaus_stage, streak) = db::record_review_outcome(
            &conn,
            &question_id,
            true,
            100,
            true,
            settings.auto_clear_streak,
        )?;

        Ok(GradeResult {
            question_id,
            is_correct: true,
            score_percent: 100,
            earned_points: 1,
            total_points: 1,
            keyword_details: Vec::new(),
            blank_results: Vec::new(),
            auto_cleared,
            next_review_at,
            ebbinghaus_stage,
            streak,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn export_excel_template(
    db: State<'_, DbState>,
    target_path: String,
    bank_id: Option<String>,
) -> Result<usize, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        excel::export_excel_template(&conn, &target_path, bank_id.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn import_excel_file(
    db: State<'_, DbState>,
    file_path: String,
    fallback_bank_name: String,
) -> Result<ImportSummary, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        excel::import_excel_file(&mut conn, &file_path, &fallback_bank_name)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_settings(db: State<'_, DbState>) -> Result<AppSettings, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        let dir = db.data_dir.lock().map_err(|_| "目录锁获取失败".to_string())?;
        db::load_settings(&conn, &dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    db: State<'_, DbState>,
    scheduler: State<'_, SchedulerState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let autostart = app.autolaunch();
    if settings.auto_start {
        let _ = autostart.enable();
    } else {
        let _ = autostart.disable();
    }

    scheduler.update_interval(settings.interval_minutes);

    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let current_dir = db
            .data_dir
            .lock()
            .map_err(|_| "目录锁获取失败".to_string())?
            .to_string_lossy()
            .to_string();

        if !settings.data_dir.trim().is_empty() && settings.data_dir.trim() != current_dir {
            db.switch_data_dir(&settings.data_dir)?;
        }

        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        let dir = db.data_dir.lock().map_err(|_| "目录锁获取失败".to_string())?;
        db::save_settings(&conn, &settings)?;
        db::load_settings(&conn, &dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_timer_status(scheduler: State<'_, SchedulerState>) -> TimerStatus {
    scheduler.get_status()
}

#[tauri::command]
pub fn get_current_popup_session(scheduler: State<'_, SchedulerState>) -> Vec<QuizSessionQuestion> {
    scheduler.get_current_session()
}

#[tauri::command]
pub fn toggle_timer_pause(scheduler: State<'_, SchedulerState>) -> bool {
    scheduler.toggle_manual_pause()
}

#[tauri::command]
pub fn trigger_popup_now(
    app: AppHandle,
    db: State<'_, DbState>,
    scheduler: State<'_, SchedulerState>,
) -> Result<bool, String> {
    scheduler.trigger_popup_now(&app, db.inner())
}

#[tauri::command]
pub fn finish_popup(
    app: AppHandle,
    scheduler: State<'_, SchedulerState>,
    snooze_minutes: Option<u32>,
) {
    scheduler.finish_popup(&app, snooze_minutes);
}
