use crate::models::{
    AppSettings, Bank, ChoiceOption, DashboardStats, Question, QuestionType, QuizSessionQuestion,
};
use chrono::{Duration, Local, NaiveDateTime};
use rand::seq::SliceRandom;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Ebbinghaus review intervals in minutes:
/// Stage 0 -> 30m, 1 -> 4h, 2 -> 12h, 3 -> 1d, 4 -> 2d, 5 -> 4d, 6 -> 7d, 7 -> 15d
const EBBINGHAUS_MINUTES: [i64; 8] = [30, 240, 720, 1440, 2880, 5760, 10080, 21600];

#[derive(Debug, Serialize, Deserialize)]
struct BootstrapConfig {
    data_dir: String,
}

#[derive(Clone)]
pub struct DbState {
    pub conn: Arc<Mutex<Connection>>,
    pub data_dir: Arc<Mutex<PathBuf>>,
}

impl DbState {
    pub fn init() -> Result<Self, String> {
        let data_dir = resolve_data_dir()?;
        fs::create_dir_all(&data_dir).map_err(|e| format!("创建数据目录失败: {e}"))?;
        let db_path = data_dir.join("quizpulse.db");
        let conn = open_and_migrate(&db_path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            data_dir: Arc::new(Mutex::new(data_dir)),
        })
    }

    pub fn switch_data_dir(&self, new_dir: &str) -> Result<String, String> {
        let target = PathBuf::from(new_dir.trim());
        fs::create_dir_all(&target).map_err(|e| format!("无法访问目标目录: {e}"))?;

        let old_dir = self
            .data_dir
            .lock()
            .map_err(|_| "锁获取失败".to_string())?
            .clone();
        let old_db = old_dir.join("quizpulse.db");
        let new_db = target.join("quizpulse.db");

        // Flush WAL and copy existing database if target doesn't already contain one
        {
            let conn = self.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }
        if !new_db.exists() && old_db.exists() {
            fs::copy(&old_db, &new_db).map_err(|e| format!("迁移数据库失败: {e}"))?;
        }

        let new_conn = open_and_migrate(&new_db)?;
        {
            let mut conn_guard = self.conn.lock().map_err(|_| "数据库锁获取失败".to_string())?;
            *conn_guard = new_conn;
        }
        {
            let mut dir_guard = self.data_dir.lock().map_err(|_| "目录锁获取失败".to_string())?;
            *dir_guard = target.clone();
        }
        save_bootstrap_data_dir(&target)?;
        Ok(target.to_string_lossy().to_string())
    }
}

fn bootstrap_config_path() -> Result<PathBuf, String> {
    let base = dirs::config_dir()
        .ok_or_else(|| "无法获取系统配置目录".to_string())?
        .join("QuizPulse");
    fs::create_dir_all(&base).map_err(|e| format!("创建配置目录失败: {e}"))?;
    Ok(base.join("bootstrap.json"))
}

fn resolve_data_dir() -> Result<PathBuf, String> {
    let cfg_path = bootstrap_config_path()?;
    if cfg_path.exists() {
        if let Ok(raw) = fs::read_to_string(&cfg_path) {
            if let Ok(cfg) = serde_json::from_str::<BootstrapConfig>(&raw) {
                let p = PathBuf::from(cfg.data_dir.trim());
                if !cfg.data_dir.trim().is_empty() {
                    return Ok(p);
                }
            }
        }
    }
    let default_dir = dirs::data_local_dir()
        .or_else(dirs::config_dir)
        .ok_or_else(|| "无法获取本地数据目录".to_string())?
        .join("QuizPulse")
        .join("data");
    Ok(default_dir)
}

fn save_bootstrap_data_dir(dir: &Path) -> Result<(), String> {
    let cfg_path = bootstrap_config_path()?;
    let payload = serde_json::to_string_pretty(&BootstrapConfig {
        data_dir: dir.to_string_lossy().to_string(),
    })
    .map_err(|e| e.to_string())?;
    fs::write(cfg_path, payload).map_err(|e| format!("写入目录配置失败: {e}"))
}

fn open_and_migrate(db_path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("打开数据库失败: {e}"))?;
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;

        CREATE TABLE IF NOT EXISTS banks (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS questions (
            id TEXT PRIMARY KEY,
            bank_id TEXT NOT NULL,
            q_type TEXT NOT NULL,
            stem TEXT NOT NULL,
            options_json TEXT NOT NULL DEFAULT '[]',
            answer_json TEXT NOT NULL DEFAULT '[]',
            keywords_json TEXT NOT NULL DEFAULT '[]',
            explanation TEXT NOT NULL DEFAULT '',
            order_insensitive INTEGER NOT NULL DEFAULT 0,
            ebbinghaus_stage INTEGER NOT NULL DEFAULT 0,
            streak INTEGER NOT NULL DEFAULT 0,
            wrong_count INTEGER NOT NULL DEFAULT 0,
            correct_count INTEGER NOT NULL DEFAULT 0,
            next_review_at TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (bank_id) REFERENCES banks(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_questions_bank_status ON questions(bank_id, status);
        CREATE INDEX IF NOT EXISTS idx_questions_review ON questions(status, next_review_at);

        CREATE TABLE IF NOT EXISTS review_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            question_id TEXT NOT NULL,
            is_correct INTEGER NOT NULL,
            score_percent INTEGER NOT NULL,
            manual_override INTEGER NOT NULL DEFAULT 0,
            reviewed_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_review_logs_date ON review_logs(reviewed_at);

        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        ",
    )
    .map_err(|e| format!("数据库初始化失败: {e}"))?;

    seed_demo_data_if_empty(&conn)?;
    Ok(conn)
}

pub fn now_iso() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn seed_demo_data_if_empty(conn: &Connection) -> Result<(), String> {
    let bank_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM banks", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if bank_count > 0 {
        return Ok(());
    }

    let now = now_iso();
    let bank_id = "bank_demo_001".to_string();
    conn.execute(
        "INSERT INTO banks (id, name, description, enabled, created_at) VALUES (?1, ?2, ?3, 1, ?4)",
        params![
            bank_id,
            "示例题库 · 计算机基础",
            "内置示例题库（涵盖单选、多选、多空填空、简答按点给分），可直接体验或删除",
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    let demo_rows = vec![
        (
            "demo_q_001",
            "single_choice",
            "HTTP 状态码 429 表示什么含义？",
            serde_json::to_string(&vec![
                ChoiceOption { key: "A".into(), text: "请求频率过高（Too Many Requests）".into() },
                ChoiceOption { key: "B".into(), text: "未授权访问（Unauthorized）".into() },
                ChoiceOption { key: "C".into(), text: "网关超时（Gateway Timeout）".into() },
                ChoiceOption { key: "D".into(), text: "资源未修改（Not Modified）".into() },
            ]).unwrap(),
            serde_json::to_string(&vec!["A"]).unwrap(),
            "[]".to_string(),
            "429 Too Many Requests 用于限流场景，常配合 Retry-After 响应头告知客户端重试时间。",
            0,
        ),
        (
            "demo_q_002",
            "multi_choice",
            "以下哪些属于 TCP 协议的核心特性？（多选）",
            serde_json::to_string(&vec![
                ChoiceOption { key: "A".into(), text: "面向连接（三次握手 / 四次挥手）".into() },
                ChoiceOption { key: "B".into(), text: "提供可靠传输与超时重传".into() },
                ChoiceOption { key: "C".into(), text: "无连接、尽最大努力交付".into() },
                ChoiceOption { key: "D".into(), text: "具备流量控制与拥塞控制".into() },
            ]).unwrap(),
            serde_json::to_string(&vec!["A", "B", "D"]).unwrap(),
            "[]".to_string(),
            "C 是 UDP 的特点；TCP 是面向连接的可靠字节流协议，包含流量控制与拥塞控制。",
            0,
        ),
        (
            "demo_q_003",
            "fill_blank",
            "在数据库事务 ACID 特性中，A 代表 ____ ，I 代表 ____ 。（两空按顺序填写，中英文均可）",
            "[]".to_string(),
            serde_json::to_string(&vec!["原子性/Atomicity", "隔离性/Isolation"]).unwrap(),
            "[]".to_string(),
            "ACID 分别是 Atomicity（原子性）、Consistency（一致性）、Isolation（隔离性）、Durability（持久性）。",
            0,
        ),
        (
            "demo_q_004",
            "short_answer",
            "简述为什么浏览器/桌面端应用把耗时解析放到后台线程执行，而不要放在主 UI 线程？",
            "[]".to_string(),
            serde_json::to_string(&vec!["主线程负责界面渲染与用户事件响应，耗时任务会阻塞消息循环，导致界面卡顿或系统判定未响应。"]).unwrap(),
            serde_json::to_string(&vec![
                "主线程/UI线程",
                "阻塞/卡死/卡顿",
                "渲染/事件响应/未响应",
            ]).unwrap(),
            "采分点：1. 主线程/UI线程职责；2. 耗时操作导致阻塞/卡顿；3. 影响渲染/事件响应或触发未响应。",
            0,
        ),
    ];

    for (id, q_type, stem, opts, ans, kw, exp, order_ins) in demo_rows {
        conn.execute(
            "INSERT INTO questions (
                id, bank_id, q_type, stem, options_json, answer_json, keywords_json,
                explanation, order_insensitive, next_review_at, status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'active', ?10, ?10)",
            params![id, bank_id, q_type, stem, opts, ans, kw, exp, order_ins, now],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn load_settings(conn: &Connection, data_dir: &Path) -> Result<AppSettings, String> {
    let mut settings = AppSettings::default();
    let json_str: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'main'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some(raw) = json_str {
        if let Ok(parsed) = serde_json::from_str::<AppSettings>(&raw) {
            settings = parsed;
        }
    }
    settings.data_dir = data_dir.to_string_lossy().to_string();
    Ok(settings)
}

pub fn save_settings(conn: &Connection, settings: &AppSettings) -> Result<(), String> {
    let raw = serde_json::to_string(settings).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES ('main', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![raw],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_banks(conn: &Connection) -> Result<Vec<Bank>, String> {
    let now = now_iso();
    let mut stmt = conn
        .prepare(
            "SELECT b.id, b.name, b.description, b.enabled, b.created_at,
                    COUNT(q.id) AS total_count,
                    SUM(CASE WHEN q.status = 'active' THEN 1 ELSE 0 END) AS active_count,
                    SUM(CASE WHEN q.status = 'cleared' THEN 1 ELSE 0 END) AS cleared_count,
                    SUM(CASE WHEN q.status = 'active' AND q.next_review_at <= ?1 THEN 1 ELSE 0 END) AS due_count
             FROM banks b
             LEFT JOIN questions q ON q.bank_id = b.id
             GROUP BY b.id
             ORDER BY b.created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![now], |r| {
            Ok(Bank {
                id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
                enabled: r.get::<_, i64>(3)? != 0,
                created_at: r.get(4)?,
                total_count: r.get(5)?,
                active_count: r.get::<_, Option<i64>>(6)?.unwrap_or(0),
                cleared_count: r.get::<_, Option<i64>>(7)?.unwrap_or(0),
                due_count: r.get::<_, Option<i64>>(8)?.unwrap_or(0),
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn create_bank(conn: &Connection, name: &str, description: &str) -> Result<Bank, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("题库名称不能为空".into());
    }
    let id = format!("bank_{}", &Uuid::new_v4().simple().to_string()[..12]);
    let now = now_iso();
    conn.execute(
        "INSERT INTO banks (id, name, description, enabled, created_at) VALUES (?1, ?2, ?3, 1, ?4)",
        params![id, trimmed, description.trim(), now],
    )
    .map_err(|e| format!("创建题库失败（名称可能重复）: {e}"))?;

    Ok(Bank {
        id,
        name: trimmed.to_string(),
        description: description.trim().to_string(),
        enabled: true,
        total_count: 0,
        active_count: 0,
        cleared_count: 0,
        due_count: 0,
        created_at: now,
    })
}

pub fn ensure_bank_by_name(conn: &Connection, name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    let target_name = if trimmed.is_empty() { "默认题库" } else { trimmed };
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM banks WHERE name = ?1",
            params![target_name],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
    {
        return Ok(id);
    }
    let bank = create_bank(conn, target_name, "")?;
    Ok(bank.id)
}

pub fn toggle_bank(conn: &Connection, bank_id: &str, enabled: bool) -> Result<(), String> {
    conn.execute(
        "UPDATE banks SET enabled = ?1 WHERE id = ?2",
        params![i64::from(enabled), bank_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_bank(conn: &Connection, bank_id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM banks WHERE id = ?1", params![bank_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn map_question_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Question> {
    let q_type_str: String = r.get(3)?;
    let opts_json: String = r.get(5)?;
    let ans_json: String = r.get(6)?;
    let kw_json: String = r.get(7)?;
    Ok(Question {
        id: r.get(0)?,
        bank_id: r.get(1)?,
        bank_name: r.get(2)?,
        q_type: QuestionType::from_db(&q_type_str),
        stem: r.get(4)?,
        options: serde_json::from_str(&opts_json).unwrap_or_default(),
        answers: serde_json::from_str(&ans_json).unwrap_or_default(),
        keywords: serde_json::from_str(&kw_json).unwrap_or_default(),
        explanation: r.get(8)?,
        order_insensitive: r.get::<_, i64>(9)? != 0,
        ebbinghaus_stage: r.get(10)?,
        streak: r.get(11)?,
        wrong_count: r.get(12)?,
        correct_count: r.get(13)?,
        next_review_at: r.get(14)?,
        status: r.get(15)?,
        created_at: r.get(16)?,
        updated_at: r.get(17)?,
    })
}

pub fn list_questions(
    conn: &Connection,
    bank_id: Option<&str>,
    status: Option<&str>,
    keyword: Option<&str>,
) -> Result<Vec<Question>, String> {
    let mut sql = String::from(
        "SELECT q.id, q.bank_id, b.name, q.q_type, q.stem, q.options_json, q.answer_json,
                q.keywords_json, q.explanation, q.order_insensitive, q.ebbinghaus_stage,
                q.streak, q.wrong_count, q.correct_count, q.next_review_at, q.status,
                q.created_at, q.updated_at
         FROM questions q
         JOIN banks b ON b.id = q.bank_id
         WHERE 1=1",
    );
    let mut param_values: Vec<String> = Vec::new();
    if let Some(bid) = bank_id.filter(|s| !s.is_empty() && *s != "all") {
        param_values.push(bid.to_string());
        sql.push_str(&format!(" AND q.bank_id = ?{}", param_values.len()));
    }
    if let Some(st) = status.filter(|s| !s.is_empty() && *s != "all") {
        if st == "wrong" {
            sql.push_str(" AND q.status = 'active' AND q.wrong_count > 0");
        } else {
            param_values.push(st.to_string());
            sql.push_str(&format!(" AND q.status = ?{}", param_values.len()));
        }
    }
    if let Some(kw) = keyword.map(str::trim).filter(|s| !s.is_empty()) {
        param_values.push(format!("%{kw}%"));
        let idx = param_values.len();
        sql.push_str(&format!(" AND (q.stem LIKE ?{idx} OR q.explanation LIKE ?{idx})"));
    }
    sql.push_str(" ORDER BY q.updated_at DESC LIMIT 500");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> =
        param_values.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), map_question_row)
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get_question_by_id(conn: &Connection, id: &str) -> Result<Question, String> {
    conn.query_row(
        "SELECT q.id, q.bank_id, b.name, q.q_type, q.stem, q.options_json, q.answer_json,
                q.keywords_json, q.explanation, q.order_insensitive, q.ebbinghaus_stage,
                q.streak, q.wrong_count, q.correct_count, q.next_review_at, q.status,
                q.created_at, q.updated_at
         FROM questions q
         JOIN banks b ON b.id = q.bank_id
         WHERE q.id = ?1",
        params![id],
        map_question_row,
    )
    .map_err(|e| format!("题目不存在: {e}"))
}

pub fn delete_question(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM questions WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_question_status(conn: &Connection, id: &str, status: &str) -> Result<(), String> {
    let now = now_iso();
    if status == "active" {
        // Restored from cleared bank -> reset review due immediately so it re-enters queue
        conn.execute(
            "UPDATE questions SET status = 'active', ebbinghaus_stage = 0, streak = 0,
                    next_review_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "UPDATE questions SET status = 'cleared', updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Pick N questions for a timed popup session:
/// Priority 1: Due review/wrong questions (status='active', bank enabled, next_review_at <= now, highest wrong_count first)
/// Priority 2: Unseen new questions (correct_count=0 AND wrong_count=0)
/// Priority 3: Upcoming active questions sorted by earliest next_review_at
pub fn pick_session_questions(
    conn: &Connection,
    count: usize,
    shuffle_options: bool,
) -> Result<Vec<QuizSessionQuestion>, String> {
    let now = now_iso();
    let limit = count.max(1) as i64;
    let mut stmt = conn
        .prepare(
            "SELECT q.id, q.bank_id, b.name, q.q_type, q.stem, q.options_json, q.answer_json,
                    q.keywords_json, q.explanation, q.order_insensitive, q.ebbinghaus_stage,
                    q.streak, q.wrong_count, q.correct_count, q.next_review_at, q.status,
                    q.created_at, q.updated_at
             FROM questions q
             JOIN banks b ON b.id = q.bank_id
             WHERE b.enabled = 1 AND q.status = 'active'
             ORDER BY
                CASE
                    WHEN q.next_review_at <= ?1 AND q.wrong_count > 0 THEN 0
                    WHEN q.next_review_at <= ?1 THEN 1
                    WHEN q.correct_count = 0 AND q.wrong_count = 0 THEN 2
                    ELSE 3
                END ASC,
                q.wrong_count DESC,
                q.next_review_at ASC
             LIMIT ?2",
        )
        .map_err(|e| e.to_string())?;

    let questions: Vec<Question> = stmt
        .query_map(params![now, limit], map_question_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut rng = rand::thread_rng();
    let session = questions
        .into_iter()
        .map(|q| {
            let mut display_options = q.options.clone();
            if shuffle_options
                && matches!(q.q_type, QuestionType::SingleChoice | QuestionType::MultiChoice)
                && display_options.len() > 1
            {
                display_options.shuffle(&mut rng);
            }
            QuizSessionQuestion {
                question: q,
                display_options,
            }
        })
        .collect();

    Ok(session)
}

/// Apply Ebbinghaus interval update after answering (or manual override).
/// Returns (auto_cleared, next_review_at, new_stage, new_streak).
pub fn record_review_outcome(
    conn: &Connection,
    question_id: &str,
    is_correct: bool,
    score_percent: u8,
    manual_override: bool,
    auto_clear_streak: i32,
) -> Result<(bool, String, i32, i32), String> {
    let q = get_question_by_id(conn, question_id)?;
    let now_dt = Local::now();
    let now_str = now_dt.format("%Y-%m-%d %H:%M:%S").to_string();

    let (new_stage, new_streak, new_wrong, new_correct, next_dt, new_status, auto_cleared) =
        if is_correct {
            let stage = (q.ebbinghaus_stage + 1).min((EBBINGHAUS_MINUTES.len() - 1) as i32);
            let streak = q.streak + 1;
            let mins = EBBINGHAUS_MINUTES[stage as usize];
            let should_clear = auto_clear_streak > 0 && streak >= auto_clear_streak;
            let status = if should_clear { "cleared" } else { "active" };
            (
                stage,
                streak,
                q.wrong_count,
                q.correct_count + 1,
                now_dt + Duration::minutes(mins),
                status,
                should_clear,
            )
        } else {
            // Wrong answer -> reset streak & stage, boost frequency to 20 minutes later
            (
                0,
                0,
                q.wrong_count + 1,
                q.correct_count,
                now_dt + Duration::minutes(20),
                "active",
                false,
            )
        };

    let next_str = next_dt.format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "UPDATE questions
         SET ebbinghaus_stage = ?1, streak = ?2, wrong_count = ?3, correct_count = ?4,
             next_review_at = ?5, status = ?6, updated_at = ?7
         WHERE id = ?8",
        params![
            new_stage,
            new_streak,
            new_wrong,
            new_correct,
            next_str,
            new_status,
            now_str,
            question_id
        ],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO review_logs (question_id, is_correct, score_percent, manual_override, reviewed_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            question_id,
            i64::from(is_correct),
            i64::from(score_percent),
            i64::from(manual_override),
            now_str
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok((auto_cleared, next_str, new_stage, new_streak))
}

pub fn get_dashboard_stats(conn: &Connection) -> Result<DashboardStats, String> {
    let now = now_iso();
    let today_prefix = format!("{}%", Local::now().format("%Y-%m-%d"));

    let (total_questions, active_questions, cleared_questions, due_now): (i64, i64, i64, i64) = conn
        .query_row(
            "SELECT
                COUNT(*),
                SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status = 'cleared' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status = 'active' AND next_review_at <= ?1 THEN 1 ELSE 0 END)
             FROM questions",
            params![now],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(3)?.unwrap_or(0),
                ))
            },
        )
        .map_err(|e| e.to_string())?;

    let (today_answered, today_correct): (i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), SUM(CASE WHEN is_correct = 1 THEN 1 ELSE 0 END)
             FROM review_logs WHERE reviewed_at LIKE ?1",
            params![today_prefix],
            |r| Ok((r.get(0)?, r.get::<_, Option<i64>>(1)?.unwrap_or(0))),
        )
        .map_err(|e| e.to_string())?;

    let today_accuracy = if today_answered > 0 {
        ((today_correct * 100) / today_answered) as u8
    } else {
        0
    };

    Ok(DashboardStats {
        total_questions,
        active_questions,
        cleared_questions,
        due_now,
        today_answered,
        today_accuracy,
    })
}

#[allow(dead_code)]
pub fn is_valid_iso(s: &str) -> bool {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").is_ok()
}
