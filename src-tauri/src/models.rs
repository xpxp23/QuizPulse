use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    SingleChoice,
    MultiChoice,
    FillBlank,
    ShortAnswer,
}

impl QuestionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            QuestionType::SingleChoice => "single_choice",
            QuestionType::MultiChoice => "multi_choice",
            QuestionType::FillBlank => "fill_blank",
            QuestionType::ShortAnswer => "short_answer",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "multi_choice" => QuestionType::MultiChoice,
            "fill_blank" => QuestionType::FillBlank,
            "short_answer" => QuestionType::ShortAnswer,
            _ => QuestionType::SingleChoice,
        }
    }

    pub fn from_cn_or_en(s: &str) -> Option<Self> {
        let normalized = s.trim().to_lowercase();
        match normalized.as_str() {
            "单选" | "单选题" | "single" | "single_choice" => Some(QuestionType::SingleChoice),
            "多选" | "多选题" | "multi" | "multi_choice" => Some(QuestionType::MultiChoice),
            "填空" | "填空题" | "fill" | "fill_blank" => Some(QuestionType::FillBlank),
            "简答" | "简答题" | "short" | "short_answer" => Some(QuestionType::ShortAnswer),
            _ => None,
        }
    }

    pub fn to_cn(&self) -> &'static str {
        match self {
            QuestionType::SingleChoice => "单选",
            QuestionType::MultiChoice => "多选",
            QuestionType::FillBlank => "填空",
            QuestionType::ShortAnswer => "简答",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bank {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub total_count: i64,
    pub active_count: i64,
    pub cleared_count: i64,
    pub due_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceOption {
    pub key: String, // "A", "B", "C", "D"...
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    pub bank_id: String,
    pub bank_name: String,
    pub q_type: QuestionType,
    pub stem: String,
    pub options: Vec<ChoiceOption>,
    pub answers: Vec<String>,
    pub keywords: Vec<String>,
    pub explanation: String,
    pub order_insensitive: bool,
    pub ebbinghaus_stage: i32,
    pub streak: i32,
    pub wrong_count: i32,
    pub correct_count: i32,
    pub next_review_at: String,
    pub status: String, // "active" | "cleared"
    pub round_num: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionUpsertRequest {
    pub id: Option<String>,
    pub bank_id: String,
    pub q_type: QuestionType,
    pub stem: String,
    pub options: Vec<ChoiceOption>,
    pub answers: Vec<String>,
    pub keywords: Vec<String>,
    pub explanation: String,
    pub order_insensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedQuestions {
    pub items: Vec<Question>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizSessionQuestion {
    pub question: Question,
    pub display_options: Vec<ChoiceOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordMatchDetail {
    pub point_label: String,
    pub matched_alias: Option<String>,
    pub is_hit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSnapshot {
    pub log_id: i64,
    pub prev_stage: i32,
    pub prev_streak: i32,
    pub prev_wrong_count: i32,
    pub prev_correct_count: i32,
    pub prev_status: String,
    pub prev_round_num: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeRequest {
    pub question_id: String,
    pub user_answers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeResult {
    pub question_id: String,
    pub is_correct: bool,
    pub score_percent: u8,
    pub earned_points: usize,
    pub total_points: usize,
    pub keyword_details: Vec<KeywordMatchDetail>,
    pub blank_results: Vec<bool>,
    pub auto_cleared: bool,
    pub next_review_at: String,
    pub ebbinghaus_stage: i32,
    pub streak: i32,
    pub snapshot: Option<ReviewSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualOverrideRequest {
    pub question_id: String,
    pub snapshot: ReviewSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundProgress {
    pub current_round: i64,
    pub completed_in_round: i64,
    pub total_active: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapDay {
    pub date: String, // "YYYY-MM-DD"
    pub total: i64,
    pub correct: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub interval_minutes: u32,
    pub questions_per_session: u32,
    pub idle_pause_minutes: u32,
    pub snooze_minutes: u32,
    pub auto_clear_streak: i32,
    pub short_answer_pass_percent: u8,
    pub fullscreen_dnd: bool,
    pub shuffle_options: bool,
    /// "shuffle_round" (不放回轮次随机) | "ebbinghaus" (艾宾浩斯智能复习) | "sequential" (严格顺序)
    pub study_order: String,
    /// Whether to still interleave due wrong questions during shuffle_round / sequential modes
    pub interleave_due_wrong: bool,
    /// Instant submit when selecting a single-choice option (click or press 1-6)
    pub instant_single_choice: bool,
    /// Subtle Web Audio chime feedback
    pub sound_enabled: bool,
    /// Global hotkey to summon quiz popup immediately, e.g. "Alt+Q" (empty to disable)
    pub global_shortcut: String,
    pub popup_position: String, // "center" | "bottom_right"
    pub theme: String,          // "system" | "light" | "dark"
    pub auto_start: bool,
    pub data_dir: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            interval_minutes: 40,
            questions_per_session: 2,
            idle_pause_minutes: 10,
            snooze_minutes: 10,
            auto_clear_streak: 5,
            short_answer_pass_percent: 60,
            fullscreen_dnd: true,
            shuffle_options: true,
            study_order: "shuffle_round".to_string(),
            interleave_due_wrong: true,
            instant_single_choice: false,
            sound_enabled: true,
            global_shortcut: "Alt+Q".to_string(),
            popup_position: "bottom_right".to_string(),
            theme: "system".to_string(),
            auto_start: true,
            data_dir: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerStatus {
    pub remaining_secs: u64,
    pub total_secs: u64,
    pub paused_manual: bool,
    pub paused_auto_reason: Option<String>,
    pub popup_open: bool,
    pub round_progress: RoundProgress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSummary {
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_questions: i64,
    pub active_questions: i64,
    pub cleared_questions: i64,
    pub due_now: i64,
    pub today_answered: i64,
    pub today_accuracy: u8,
    pub round_progress: RoundProgress,
}
