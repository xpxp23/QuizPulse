export type QuestionType =
  | "single_choice"
  | "multi_choice"
  | "fill_blank"
  | "short_answer";

export interface ChoiceOption {
  key: string;
  text: string;
}

export interface Bank {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  total_count: number;
  active_count: number;
  cleared_count: number;
  due_count: number;
  created_at: string;
}

export interface Question {
  id: string;
  bank_id: string;
  bank_name: string;
  q_type: QuestionType;
  stem: string;
  options: ChoiceOption[];
  answers: string[];
  keywords: string[];
  explanation: string;
  order_insensitive: boolean;
  ebbinghaus_stage: number;
  streak: number;
  wrong_count: number;
  correct_count: number;
  next_review_at: string;
  status: "active" | "cleared";
  round_num: number;
  created_at: string;
  updated_at: string;
}

export interface QuestionUpsertRequest {
  id: string | null;
  bank_id: string;
  q_type: QuestionType;
  stem: string;
  options: ChoiceOption[];
  answers: string[];
  keywords: string[];
  explanation: string;
  order_insensitive: boolean;
}

export interface PaginatedQuestions {
  items: Question[];
  total: number;
  page: number;
  page_size: number;
}

export interface QuizSessionQuestion {
  question: Question;
  display_options: ChoiceOption[];
}

export interface KeywordMatchDetail {
  point_label: string;
  matched_alias: string | null;
  is_hit: boolean;
}

export interface ReviewSnapshot {
  log_id: number;
  prev_stage: number;
  prev_streak: number;
  prev_wrong_count: number;
  prev_correct_count: number;
  prev_status: string;
  prev_round_num: number;
}

export interface GradeResult {
  question_id: string;
  is_correct: boolean;
  score_percent: number;
  earned_points: number;
  total_points: number;
  keyword_details: KeywordMatchDetail[];
  blank_results: boolean[];
  auto_cleared: boolean;
  next_review_at: string;
  ebbinghaus_stage: number;
  streak: number;
  snapshot: ReviewSnapshot | null;
}

export interface RoundProgress {
  current_round: number;
  completed_in_round: number;
  total_active: number;
}

export interface HeatmapDay {
  date: string;
  total: number;
  correct: number;
}

export type StudyOrder = "shuffle_round" | "ebbinghaus" | "sequential";

export interface AppSettings {
  interval_minutes: number;
  questions_per_session: number;
  idle_pause_minutes: number;
  snooze_minutes: number;
  auto_clear_streak: number;
  short_answer_pass_percent: number;
  fullscreen_dnd: boolean;
  shuffle_options: boolean;
  study_order: StudyOrder;
  interleave_due_wrong: boolean;
  instant_single_choice: boolean;
  sound_enabled: boolean;
  global_shortcut: string;
  popup_position: "center" | "bottom_right";
  theme: "system" | "light" | "dark";
  auto_start: boolean;
  data_dir: string;
}

export interface TimerStatus {
  remaining_secs: number;
  total_secs: number;
  paused_manual: boolean;
  paused_auto_reason: string | null;
  popup_open: boolean;
  round_progress: RoundProgress;
}

export interface ImportSummary {
  inserted: number;
  updated: number;
  skipped: number;
  errors: string[];
}

export interface DashboardStats {
  total_questions: number;
  active_questions: number;
  cleared_questions: number;
  due_now: number;
  today_answered: number;
  today_accuracy: number;
  round_progress: RoundProgress;
}

export const QUESTION_TYPE_LABELS: Record<QuestionType, string> = {
  single_choice: "单选题",
  multi_choice: "多选题",
  fill_blank: "填空题",
  short_answer: "简答题",
};
