use crate::db::{self, DbState};
use crate::excel;
use crate::grading;
use crate::models::{
    AppSettings, Bank, DashboardStats, GradeRequest, GradeResult, HeatmapDay, ImportSummary,
    ManualOverrideRequest, PaginatedQuestions, Question, QuestionUpsertRequest,
    QuizSessionQuestion, RoundProgress, TimerStatus,
};
use crate::scheduler::SchedulerState;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt;

fn broadcast_data_changed(app: &AppHandle, db: &DbState, scheduler: &SchedulerState) {
    scheduler.refresh_round_progress_from_db(db);
    scheduler.update_tray_tooltip(app);
    let _ = app.emit("data:updated", ());
}

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
pub async fn get_activity_heatmap(
    db: State<'_, DbState>,
    days: Option<i64>,
) -> Result<Vec<HeatmapDay>, String> {
    let db = db.inner().clone();
    let d = days.unwrap_or(90);
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::get_activity_heatmap(&conn, d)
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
    app: AppHandle,
    db: State<'_, DbState>,
    scheduler: State<'_, SchedulerState>,
    name: String,
    description: String,
) -> Result<Bank, String> {
    let db_clone = db.inner().clone();
    let bank = tokio::task::spawn_blocking(move || {
        let conn = db_clone.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::create_bank(&conn, &name, &description)
    })
    .await
    .map_err(|e| e.to_string())??;
    broadcast_data_changed(&app, db.inner(), scheduler.inner());
    Ok(bank)
}

#[tauri::command]
pub async fn toggle_bank(
    app: AppHandle,
    db: State<'_, DbState>,
    scheduler: State<'_, SchedulerState>,
    bank_id: String,
    enabled: bool,
) -> Result<(), String> {
    let db_clone = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db_clone.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::toggle_bank(&conn, &bank_id, enabled)
    })
    .await
    .map_err(|e| e.to_string())??;
    broadcast_data_changed(&app, db.inner(), scheduler.inner());
    Ok(())
}

#[tauri::command]
pub async fn delete_bank(
    app: AppHandle,
    db: State<'_, DbState>,
    scheduler: State<'_, SchedulerState>,
    bank_id: String,
) -> Result<(), String> {
    let db_clone = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db_clone.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::delete_bank(&conn, &bank_id)
    })
    .await
    .map_err(|e| e.to_string())??;
    broadcast_data_changed(&app, db.inner(), scheduler.inner());
    Ok(())
}

#[tauri::command]
pub async fn list_questions(
    db: State<'_, DbState>,
    bank_id: Option<String>,
    status: Option<String>,
    keyword: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<PaginatedQuestions, String> {
    let db = db.inner().clone();
    let p = page.unwrap_or(1);
    let ps = page_size.unwrap_or(50);
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::list_questions_paginated(
            &conn,
            bank_id.as_deref(),
            status.as_deref(),
            keyword.as_deref(),
            p,
            ps,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn upsert_question(
    app: AppHandle,
    db: State<'_, DbState>,
    scheduler: State<'_, SchedulerState>,
    req: QuestionUpsertRequest,
) -> Result<Question, String> {
    let db_clone = db.inner().clone();
    let q = tokio::task::spawn_blocking(move || {
        let conn = db_clone.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::upsert_question(&conn, &req)
    })
    .await
    .map_err(|e| e.to_string())??;
    broadcast_data_changed(&app, db.inner(), scheduler.inner());
    Ok(q)
}

#[tauri::command]
pub async fn delete_question(
    app: AppHandle,
    db: State<'_, DbState>,
    scheduler: State<'_, SchedulerState>,
    question_id: String,
) -> Result<(), String> {
    let db_clone = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db_clone.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::delete_question(&conn, &question_id)
    })
    .await
    .map_err(|e| e.to_string())??;
    broadcast_data_changed(&app, db.inner(), scheduler.inner());
    Ok(())
}

#[tauri::command]
pub async fn set_question_status(
    app: AppHandle,
    db: State<'_, DbState>,
    scheduler: State<'_, SchedulerState>,
    question_id: String,
    status: String,
) -> Result<(), String> {
    let db_clone = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db_clone.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::set_question_status(&conn, &question_id, &status)
    })
    .await
    .map_err(|e| e.to_string())??;
    broadcast_data_changed(&app, db.inner(), scheduler.inner());
    Ok(())
}

#[tauri::command]
pub async fn skip_question(
    db: State<'_, DbState>,
    question_id: String,
) -> Result<(), String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::skip_question(&conn, &question_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn start_new_round(
    app: AppHandle,
    db: State<'_, DbState>,
    scheduler: State<'_, SchedulerState>,
) -> Result<RoundProgress, String> {
    let db_clone = db.inner().clone();
    let rp = tokio::task::spawn_blocking(move || {
        let conn = db_clone.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::start_new_shuffle_round(&conn)?;
        db::get_round_progress(&conn)
    })
    .await
    .map_err(|e| e.to_string())??;
    scheduler.update_round_progress(rp.clone());
    scheduler.update_tray_tooltip(&app);
    let _ = app.emit("data:updated", ());
    Ok(rp)
}

#[tauri::command]
pub async fn pick_focus_questions(
    db: State<'_, DbState>,
    scheduler: State<'_, SchedulerState>,
    count: usize,
    bank_id: Option<String>,
    wrong_only: bool,
) -> Result<Vec<QuizSessionQuestion>, String> {
    let db = db.inner().clone();
    let settings = scheduler.get_cached_settings();
    tokio::task::spawn_blocking(move || {
        let conn = db.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        db::pick_session_questions(&conn, count, &settings, bank_id.as_deref(), wrong_only)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn submit_answer(
    app: AppHandle,
    db: State<'_, DbState>,
    scheduler: State<'_, SchedulerState>,
    req: GradeRequest,
) -> Result<GradeResult, String> {
    let db_clone = db.inner().clone();
    let settings = scheduler.get_cached_settings();
    let result = tokio::task::spawn_blocking(move || {
        let conn = db_clone.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        let question = db::get_question_by_id(&conn, &req.question_id)?;

        let outcome = grading::evaluate_question(
            &question,
            &req.user_answers,
            settings.short_answer_pass_percent,
        );

        let (auto_cleared, next_review_at, ebbinghaus_stage, streak, snapshot) =
            db::record_review_outcome(
                &conn,
                &question.id,
                outcome.is_correct,
                outcome.score_percent,
                settings.auto_clear_streak,
            )?;

        Ok::<GradeResult, String>(GradeResult {
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
            snapshot: Some(snapshot),
        })
    })
    .await
    .map_err(|e| e.to_string())??;

    broadcast_data_changed(&app, db.inner(), scheduler.inner());
    Ok(result)
}

#[tauri::command]
pub async fn manual_override_pass(
    app: AppHandle,
    db: State<'_, DbState>,
    scheduler: State<'_, SchedulerState>,
    req: ManualOverrideRequest,
) -> Result<GradeResult, String> {
    let db_clone = db.inner().clone();
    let settings = scheduler.get_cached_settings();
    let result = tokio::task::spawn_blocking(move || {
        let conn = db_clone.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        let (auto_cleared, next_review_at, ebbinghaus_stage, streak) =
            db::amend_manual_override_pass(
                &conn,
                &req.question_id,
                &req.snapshot,
                settings.auto_clear_streak,
            )?;

        Ok::<GradeResult, String>(GradeResult {
            question_id: req.question_id,
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
            snapshot: None,
        })
    })
    .await
    .map_err(|e| e.to_string())??;

    broadcast_data_changed(&app, db.inner(), scheduler.inner());
    Ok(result)
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
    app: AppHandle,
    db: State<'_, DbState>,
    scheduler: State<'_, SchedulerState>,
    file_path: String,
    fallback_bank_name: String,
) -> Result<ImportSummary, String> {
    let db_clone = db.inner().clone();
    let summary = tokio::task::spawn_blocking(move || {
        let mut conn = db_clone.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        excel::import_excel_file(&mut conn, &file_path, &fallback_bank_name)
    })
    .await
    .map_err(|e| e.to_string())??;
    broadcast_data_changed(&app, db.inner(), scheduler.inner());
    Ok(summary)
}

#[tauri::command]
pub async fn get_settings(scheduler: State<'_, SchedulerState>) -> Result<AppSettings, String> {
    Ok(scheduler.get_cached_settings())
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

    // Apply global shortcut; rollback to previous shortcut if registration fails
    let prev_shortcut = scheduler.get_cached_settings().global_shortcut;
    if let Err(err) = crate::apply_global_shortcut(&app, &settings.global_shortcut) {
        let _ = crate::apply_global_shortcut(&app, &prev_shortcut);
        return Err(err);
    }

    let db_clone = db.inner().clone();
    let saved = tokio::task::spawn_blocking(move || {
        let current_dir = db_clone
            .data_dir
            .lock()
            .map_err(|_| "目录锁获取失败".to_string())?
            .to_string_lossy()
            .to_string();

        if !settings.data_dir.trim().is_empty() && settings.data_dir.trim() != current_dir {
            db_clone.switch_data_dir(&settings.data_dir)?;
        }

        let conn = db_clone.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
        let dir = db_clone.data_dir.lock().map_err(|_| "目录锁获取失败".to_string())?;
        db::save_settings(&conn, &settings)?;
        db::load_settings(&conn, &dir)
    })
    .await
    .map_err(|e| e.to_string())??;

    scheduler.update_cached_settings(saved.clone());
    scheduler.refresh_round_progress_from_db(db.inner());
    scheduler.update_tray_tooltip(&app);
    let _ = app.emit("settings:updated", &saved);
    Ok(saved)
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
pub fn toggle_timer_pause(app: AppHandle, scheduler: State<'_, SchedulerState>) -> bool {
    scheduler.toggle_manual_pause(&app)
}

#[tauri::command]
pub fn snooze_dnd(app: AppHandle, scheduler: State<'_, SchedulerState>, minutes: u32) {
    scheduler.snooze_or_dnd(&app, minutes);
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

#[tauri::command]
pub fn reposition_popup(app: AppHandle, scheduler: State<'_, SchedulerState>) {
    scheduler.reposition_popup(&app);
}
