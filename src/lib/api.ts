import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  Bank,
  DashboardStats,
  GradeResult,
  HeatmapDay,
  ImportSummary,
  PaginatedQuestions,
  Question,
  QuestionUpsertRequest,
  QuizSessionQuestion,
  ReviewSnapshot,
  RoundProgress,
  TimerStatus,
} from "../types";

export const api = {
  getDashboardStats: () => invoke<DashboardStats>("get_dashboard_stats"),
  getActivityHeatmap: (days = 90) =>
    invoke<HeatmapDay[]>("get_activity_heatmap", { days }),

  listBanks: () => invoke<Bank[]>("list_banks"),
  createBank: (name: string, description: string) =>
    invoke<Bank>("create_bank", { name, description }),
  toggleBank: (bankId: string, enabled: boolean) =>
    invoke<void>("toggle_bank", { bankId, enabled }),
  deleteBank: (bankId: string) => invoke<void>("delete_bank", { bankId }),

  listQuestions: (params: {
    bankId?: string;
    status?: string;
    keyword?: string;
    page?: number;
    pageSize?: number;
  }) =>
    invoke<PaginatedQuestions>("list_questions", {
      bankId: params.bankId ?? null,
      status: params.status ?? null,
      keyword: params.keyword ?? null,
      page: params.page ?? 1,
      pageSize: params.pageSize ?? 50,
    }),
  upsertQuestion: (req: QuestionUpsertRequest) =>
    invoke<Question>("upsert_question", { req }),
  deleteQuestion: (questionId: string) =>
    invoke<void>("delete_question", { questionId }),
  setQuestionStatus: (questionId: string, status: "active" | "cleared") =>
    invoke<void>("set_question_status", { questionId, status }),
  skipQuestion: (questionId: string) =>
    invoke<void>("skip_question", { questionId }),
  startNewRound: () => invoke<RoundProgress>("start_new_round"),

  pickFocusQuestions: (params: {
    count: number;
    bankId?: string;
    wrongOnly: boolean;
  }) =>
    invoke<QuizSessionQuestion[]>("pick_focus_questions", {
      count: params.count,
      bankId: params.bankId ?? null,
      wrongOnly: params.wrongOnly,
    }),

  submitAnswer: (questionId: string, userAnswers: string[]) =>
    invoke<GradeResult>("submit_answer", {
      req: { question_id: questionId, user_answers: userAnswers },
    }),
  manualOverridePass: (questionId: string, snapshot: ReviewSnapshot) =>
    invoke<GradeResult>("manual_override_pass", {
      req: { question_id: questionId, snapshot },
    }),

  exportExcelTemplate: (targetPath: string, bankId?: string) =>
    invoke<number>("export_excel_template", {
      targetPath,
      bankId: bankId ?? null,
    }),
  importExcelFile: (filePath: string, fallbackBankName: string) =>
    invoke<ImportSummary>("import_excel_file", { filePath, fallbackBankName }),

  getSettings: () => invoke<AppSettings>("get_settings"),
  saveSettings: (settings: AppSettings) =>
    invoke<AppSettings>("save_settings", { settings }),

  getTimerStatus: () => invoke<TimerStatus>("get_timer_status"),
  getCurrentPopupSession: () =>
    invoke<QuizSessionQuestion[]>("get_current_popup_session"),
  toggleTimerPause: () => invoke<boolean>("toggle_timer_pause"),
  snoozeDnd: (minutes: number) => invoke<void>("snooze_dnd", { minutes }),
  triggerPopupNow: () => invoke<boolean>("trigger_popup_now"),
  finishPopup: (snoozeMinutes?: number) =>
    invoke<void>("finish_popup", { snoozeMinutes: snoozeMinutes ?? null }),
  repositionPopup: () => invoke<void>("reposition_popup"),
};
