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
    pub key: String,  // "A", "B", "C", "D"...
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
    /// For single/multi choice: ["A"] or ["A", "C"]
    /// For fill_blank: each element is one blank's synonym group like "HTTP/http"
    /// For short_answer: reference full answer text
    pub answers: Vec<String>,
    /// For short_answer: list of scoring points; each point may contain "/" synonyms, e.g. "三次握手/3次握手"
    pub keywords: Vec<String>,
    pub explanation: String,
    pub order_insensitive: bool,
    pub ebbinghaus_stage: i32,
    pub streak: i32,
    pub wrong_count: i32,
    pub correct_count: i32,
    pub next_review_at: String,
    pub status: String, // "active" | "cleared"
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizSessionQuestion {
    pub question: Question,
    /// Shuffled display options for choice questions; key maps back to original option key
    pub display_options: Vec<ChoiceOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordMatchDetail {
    pub point_label: String,
    pub matched_alias: Option<String>,
    pub is_hit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeRequest {
    pub question_id: String,
    /// For single/multi choice: selected original keys ["A"] or ["A", "C"]
    /// For fill_blank: user's input for each blank
    /// For short_answer: single element containing full answer text
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub interval_minutes: u32,
    pub questions_per_session: u32,
    pub idle_pause_minutes: u32,
    pub snooze_minutes: u32,
    pub auto_clear_streak: i32,
    pub short_answer_pass_percent: u8,
    pub fullscreen_dnd: bool,
    pub shuffle_options: bool,
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
}
