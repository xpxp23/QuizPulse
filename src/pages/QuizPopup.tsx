import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  CheckCircle2,
  Clock,
  FastForward,
  Sparkles,
  Trophy,
  XCircle,
  Check,
  GripHorizontal,
} from "lucide-react";
import { api } from "../lib/api";
import { applyTheme } from "../lib/theme";
import {
  QUESTION_TYPE_LABELS,
  type GradeResult,
  type QuizSessionQuestion,
} from "../types";

export default function QuizPopup() {
  const [session, setSession] = useState<QuizSessionQuestion[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [selectedKeys, setSelectedKeys] = useState<string[]>([]);
  const [blankInputs, setBlankInputs] = useState<string[]>([]);
  const [shortInput, setShortInput] = useState("");
  const [result, setResult] = useState<GradeResult | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [clearedManual, setClearedManual] = useState(false);
  const [snoozeMinutes, setSnoozeMinutes] = useState(10);

  const firstInputRef = useRef<HTMLInputElement | HTMLTextAreaElement | null>(
    null
  );

  const currentItem = session[currentIndex] ?? null;
  const question = currentItem?.question ?? null;

  const resetInputsForQuestion = useCallback(
    (item: QuizSessionQuestion | null) => {
      setSelectedKeys([]);
      setShortInput("");
      setResult(null);
      setClearedManual(false);
      if (item && item.question.q_type === "fill_blank") {
        const count = Math.max(1, item.question.answers.length);
        setBlankInputs(new Array(count).fill(""));
      } else {
        setBlankInputs([]);
      }
      setTimeout(() => {
        firstInputRef.current?.focus();
      }, 60);
    },
    []
  );

  // Sync theme and snooze preference
  useEffect(() => {
    let cleanupTheme = () => {};
    api
      .getSettings()
      .then((s) => {
        setSnoozeMinutes(s.snooze_minutes || 10);
        cleanupTheme = applyTheme(s.theme);
      })
      .catch(() => {
        cleanupTheme = applyTheme("system");
      });
    return () => cleanupTheme();
  }, []);

  // Load current session on mount & listen for backend quiz:start events
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;
    let active = true;

    api
      .getCurrentPopupSession()
      .then((items) => {
        if (active && items.length > 0) {
          setSession(items);
          setCurrentIndex(0);
          resetInputsForQuestion(items[0]);
        }
      })
      .catch(() => {});

    listen<QuizSessionQuestion[]>("quiz:start", (event) => {
      const items = event.payload || [];
      setSession(items);
      setCurrentIndex(0);
      resetInputsForQuestion(items[0] ?? null);
      // Refresh theme/snooze in case settings changed
      api
        .getSettings()
        .then((s) => {
          setSnoozeMinutes(s.snooze_minutes || 10);
          applyTheme(s.theme);
        })
        .catch(() => {});
    }).then((fn) => {
      if (active) {
        unlistenFn = fn;
      } else {
        fn();
      }
    });

    return () => {
      active = false;
      if (unlistenFn) unlistenFn();
    };
  }, [resetInputsForQuestion]);

  const handleToggleOption = useCallback(
    (originalKey: string) => {
      if (!question || result) return;
      if (question.q_type === "single_choice") {
        setSelectedKeys([originalKey]);
      } else if (question.q_type === "multi_choice") {
        setSelectedKeys((prev) =>
          prev.includes(originalKey)
            ? prev.filter((k) => k !== originalKey)
            : [...prev, originalKey]
        );
      }
    },
    [question, result]
  );

  const handleSubmit = useCallback(async () => {
    if (!question || result || submitting) return;
    let payload: string[] = [];
    if (
      question.q_type === "single_choice" ||
      question.q_type === "multi_choice"
    ) {
      if (selectedKeys.length === 0) return;
      payload = selectedKeys;
    } else if (question.q_type === "fill_blank") {
      if (blankInputs.every((v) => v.trim() === "")) return;
      payload = blankInputs;
    } else {
      if (shortInput.trim() === "") return;
      payload = [shortInput];
    }

    setSubmitting(true);
    try {
      const res = await api.submitAnswer(question.id, payload);
      setResult(res);
    } finally {
      setSubmitting(false);
    }
  }, [question, result, submitting, selectedKeys, blankInputs, shortInput]);

  const handleNextOrFinish = useCallback(async () => {
    if (currentIndex + 1 < session.length) {
      const nextIdx = currentIndex + 1;
      setCurrentIndex(nextIdx);
      resetInputsForQuestion(session[nextIdx]);
    } else {
      await api.finishPopup();
    }
  }, [currentIndex, session, resetInputsForQuestion]);

  const handleSnooze = useCallback(async () => {
    await api.finishPopup(snoozeMinutes);
  }, [snoozeMinutes]);

  const handleSkip = useCallback(async () => {
    await handleNextOrFinish();
  }, [handleNextOrFinish]);

  const handleManualOverride = useCallback(async () => {
    if (!question) return;
    const res = await api.manualOverridePass(question.id);
    setResult((prev) =>
      prev
        ? {
            ...prev,
            is_correct: true,
            score_percent: 100,
            auto_cleared: res.auto_cleared,
            next_review_at: res.next_review_at,
            ebbinghaus_stage: res.ebbinghaus_stage,
            streak: res.streak,
          }
        : res
    );
  }, [question]);

  const handleMarkCleared = useCallback(async () => {
    if (!question) return;
    await api.setQuestionStatus(question.id, "cleared");
    setClearedManual(true);
  }, [question]);

  // Global keyboard shortcuts: 1-6 pick options, Enter submit/next, Esc snooze
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!currentItem || !question) return;

      if (e.key === "Escape") {
        e.preventDefault();
        handleSnooze();
        return;
      }

      const targetTag = (e.target as HTMLElement)?.tagName;
      const isTextarea = targetTag === "TEXTAREA";

      if (e.key === "Enter") {
        if (result) {
          e.preventDefault();
          handleNextOrFinish();
          return;
        }
        if (isTextarea) {
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            handleSubmit();
          }
          return;
        }
        e.preventDefault();
        handleSubmit();
        return;
      }

      if (
        !result &&
        (question.q_type === "single_choice" ||
          question.q_type === "multi_choice")
      ) {
        const num = Number(e.key);
        if (num >= 1 && num <= currentItem.display_options.length) {
          e.preventDefault();
          handleToggleOption(currentItem.display_options[num - 1].key);
        }
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    currentItem,
    question,
    result,
    handleSnooze,
    handleSubmit,
    handleNextOrFinish,
    handleToggleOption,
  ]);

  if (!currentItem || !question) {
    return (
      <div className="h-full w-full p-3 flex items-center justify-center">
        <div className="w-full rounded-2xl bg-white/95 dark:bg-slate-900/95 backdrop-blur-xl border border-slate-200/80 dark:border-slate-700/80 p-6 text-center shadow-card">
          <p className="text-sm text-slate-500 dark:text-slate-400">
            正在加载题目...
          </p>
        </div>
      </div>
    );
  }

  const progressPct = ((currentIndex + (result ? 1 : 0)) / session.length) * 100;
  const isLast = currentIndex + 1 >= session.length;

  return (
    <div className="h-full w-full p-3 flex flex-col">
      <div className="flex-1 flex flex-col rounded-2xl bg-white/95 dark:bg-slate-900/95 backdrop-blur-xl border border-slate-200/90 dark:border-slate-700/80 shadow-card dark:shadow-card-dark overflow-hidden animate-fade-up">
        {/* Top Progress Hairline */}
        <div className="h-1 w-full bg-slate-100 dark:bg-slate-800">
          <div
            className="h-full bg-gradient-to-r from-blue-500 to-indigo-500 transition-all duration-300"
            style={{ width: `${Math.max(8, progressPct)}%` }}
          />
        </div>

        {/* Draggable Header */}
        <div
          data-tauri-drag-region
          className="px-4 py-3 flex items-center justify-between border-b border-slate-100 dark:border-slate-800/90 cursor-move"
        >
          <div className="flex items-center gap-2 pointer-events-none">
            <GripHorizontal className="w-4 h-4 text-slate-400" />
            <span className="px-2 py-0.5 text-xs font-semibold rounded-md bg-blue-50 text-blue-600 dark:bg-blue-500/15 dark:text-blue-300">
              {QUESTION_TYPE_LABELS[question.q_type]}
            </span>
            <span className="text-xs text-slate-500 dark:text-slate-400 truncate max-w-[160px]">
              {question.bank_name}
            </span>
            <span className="text-xs font-medium text-slate-400 dark:text-slate-500">
              {currentIndex + 1} / {session.length}
            </span>
          </div>

          <div className="flex items-center gap-1.5">
            <button
              onClick={handleSnooze}
              title={`延后 ${snoozeMinutes} 分钟 (Esc)`}
              className="px-2.5 py-1 text-xs font-medium rounded-lg text-slate-500 hover:text-slate-800 hover:bg-slate-100 dark:text-slate-400 dark:hover:text-slate-100 dark:hover:bg-slate-800 transition-colors flex items-center gap-1"
            >
              <Clock className="w-3.5 h-3.5" />
              延后{snoozeMinutes}分
            </button>
            <button
              onClick={handleSkip}
              title="跳过此题"
              className="px-2 py-1 text-xs font-medium rounded-lg text-slate-400 hover:text-slate-700 hover:bg-slate-100 dark:hover:text-slate-200 dark:hover:bg-slate-800 transition-colors flex items-center gap-1"
            >
              <FastForward className="w-3.5 h-3.5" />
              跳过
            </button>
          </div>
        </div>

        {/* Scrollable Content */}
        <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4 select-text">
          {/* Question Stem */}
          <div className="text-[15px] leading-relaxed font-medium text-slate-800 dark:text-slate-100 whitespace-pre-wrap">
            {question.stem}
          </div>

          {/* Choice Options */}
          {(question.q_type === "single_choice" ||
            question.q_type === "multi_choice") && (
            <div className="space-y-2">
              {currentItem.display_options.map((opt, idx) => {
                const isSelected = selectedKeys.includes(opt.key);
                const isCorrectKey = question.answers
                  .map((a) => a.toUpperCase())
                  .includes(opt.key.toUpperCase());

                let styleClass =
                  "border-slate-200/90 dark:border-slate-700/80 hover:border-blue-400 dark:hover:border-blue-500 hover:bg-blue-50/40 dark:hover:bg-blue-500/10";
                if (result) {
                  if (isCorrectKey) {
                    styleClass =
                      "border-emerald-500 bg-emerald-50/80 dark:bg-emerald-500/15 text-emerald-900 dark:text-emerald-200";
                  } else if (isSelected && !isCorrectKey) {
                    styleClass =
                      "border-rose-400 bg-rose-50/80 dark:bg-rose-500/15 text-rose-900 dark:text-rose-200";
                  } else {
                    styleClass =
                      "border-slate-200/60 dark:border-slate-800 opacity-60";
                  }
                } else if (isSelected) {
                  styleClass =
                    "border-blue-500 bg-blue-50/90 dark:bg-blue-500/20 ring-1 ring-blue-500/30";
                }

                return (
                  <button
                    key={opt.key}
                    disabled={!!result}
                    onClick={() => handleToggleOption(opt.key)}
                    className={`w-full text-left px-3.5 py-2.5 rounded-xl border transition-all flex items-start gap-3 ${styleClass}`}
                  >
                    <span
                      className={`mt-0.5 w-6 h-6 shrink-0 rounded-lg text-xs font-semibold flex items-center justify-center transition-colors ${
                        isSelected
                          ? "bg-blue-600 text-white"
                          : "bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300"
                      }`}
                    >
                      {idx + 1}
                    </span>
                    <span className="text-sm leading-snug flex-1 text-slate-700 dark:text-slate-200">
                      {opt.text}
                    </span>
                    {result && isCorrectKey && (
                      <CheckCircle2 className="w-4 h-4 text-emerald-500 shrink-0 mt-0.5" />
                    )}
                    {result && isSelected && !isCorrectKey && (
                      <XCircle className="w-4 h-4 text-rose-500 shrink-0 mt-0.5" />
                    )}
                  </button>
                );
              })}
            </div>
          )}

          {/* Fill-in-the-blank Inputs */}
          {question.q_type === "fill_blank" && (
            <div className="space-y-2.5">
              {question.order_insensitive && (
                <div className="text-xs text-indigo-600 dark:text-indigo-400 flex items-center gap-1">
                  <Sparkles className="w-3.5 h-3.5" />
                  本题多空不限顺序，忽略大小写与首尾空格
                </div>
              )}
              {blankInputs.map((val, idx) => {
                const isBlankOk = result ? result.blank_results[idx] : null;
                return (
                  <div key={idx} className="flex items-center gap-2.5">
                    <span className="text-xs font-medium text-slate-500 dark:text-slate-400 w-12 shrink-0">
                      第 {idx + 1} 空
                    </span>
                    <input
                      ref={
                        idx === 0
                          ? (el) => {
                              firstInputRef.current = el;
                            }
                          : undefined
                      }
                      type="text"
                      disabled={!!result}
                      value={val}
                      onChange={(e) => {
                        const next = [...blankInputs];
                        next[idx] = e.target.value;
                        setBlankInputs(next);
                      }}
                      placeholder="输入答案..."
                      className={`flex-1 px-3.5 py-2 text-sm rounded-xl border bg-slate-50/70 dark:bg-slate-800/70 text-slate-900 dark:text-slate-100 outline-none transition-all ${
                        isBlankOk === true
                          ? "border-emerald-500 bg-emerald-50/50 dark:bg-emerald-500/10"
                          : isBlankOk === false
                          ? "border-rose-400 bg-rose-50/50 dark:bg-rose-500/10"
                          : "border-slate-200 dark:border-slate-700 focus:border-blue-500 focus:ring-2 focus:ring-blue-500/20"
                      }`}
                    />
                  </div>
                );
              })}
            </div>
          )}

          {/* Short Answer Input */}
          {question.q_type === "short_answer" && (
            <div className="space-y-1.5">
              <textarea
                ref={(el) => {
                  firstInputRef.current = el;
                }}
                rows={4}
                disabled={!!result}
                value={shortInput}
                onChange={(e) => setShortInput(e.target.value)}
                placeholder="输入核心要点即可（按点给分，Ctrl + Enter 快速提交）..."
                className="w-full px-3.5 py-2.5 text-sm rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50/70 dark:bg-slate-800/70 text-slate-900 dark:text-slate-100 outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-500/20 resize-none leading-relaxed"
              />
              <div className="flex justify-between text-[11px] text-slate-400">
                <span>支持同义词模糊匹配，提交后可高亮查看采分点</span>
                <span>{shortInput.length} 字</span>
              </div>
            </div>
          )}

          {/* Grading Feedback Panel */}
          {result && (
            <div
              className={`rounded-xl p-3.5 border space-y-2.5 animate-fade-up ${
                result.is_correct
                  ? "bg-emerald-50/70 dark:bg-emerald-500/10 border-emerald-200 dark:border-emerald-500/30"
                  : "bg-rose-50/70 dark:bg-rose-500/10 border-rose-200 dark:border-rose-500/30"
              }`}
            >
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  {result.is_correct ? (
                    <CheckCircle2 className="w-5 h-5 text-emerald-600 dark:text-emerald-400" />
                  ) : (
                    <XCircle className="w-5 h-5 text-rose-600 dark:text-rose-400" />
                  )}
                  <span
                    className={`text-sm font-semibold ${
                      result.is_correct
                        ? "text-emerald-800 dark:text-emerald-300"
                        : "text-rose-800 dark:text-rose-300"
                    }`}
                  >
                    {result.is_correct ? "回答正确" : "还差一点"} ·{" "}
                    {result.score_percent}分
                  </span>
                </div>

                {!result.is_correct &&
                  question.q_type === "short_answer" && (
                    <button
                      onClick={handleManualOverride}
                      className="px-2.5 py-1 text-xs font-medium rounded-lg bg-white dark:bg-slate-800 text-emerald-700 dark:text-emerald-300 border border-emerald-300 dark:border-emerald-600/50 hover:bg-emerald-50 dark:hover:bg-emerald-500/20 transition-colors flex items-center gap-1"
                    >
                      <Check className="w-3.5 h-3.5" />
                      我答对意思了（算我过）
                    </button>
                  )}
              </div>

              {/* Short Answer Keyword Pills */}
              {result.keyword_details.length > 0 && (
                <div className="space-y-1.5">
                  <div className="text-xs font-medium text-slate-600 dark:text-slate-300">
                    采分点命中情况（{result.earned_points} /{" "}
                    {result.total_points}）：
                  </div>
                  <div className="flex flex-wrap gap-1.5">
                    {result.keyword_details.map((kw, i) => (
                      <span
                        key={i}
                        className={`px-2 py-0.5 text-xs rounded-md font-medium border ${
                          kw.is_hit
                            ? "bg-emerald-100/90 dark:bg-emerald-500/20 text-emerald-800 dark:text-emerald-200 border-emerald-300 dark:border-emerald-500/40"
                            : "bg-rose-100/80 dark:bg-rose-500/20 text-rose-800 dark:text-rose-200 border-rose-300 dark:border-rose-500/40"
                        }`}
                      >
                        {kw.is_hit ? "✓ " : "✗ "}
                        {kw.point_label}
                        {kw.matched_alias &&
                        kw.matched_alias !== kw.point_label
                          ? ` (${kw.matched_alias})`
                          : ""}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {/* Reference Answer for FillBlank / ShortAnswer */}
              {(question.q_type === "fill_blank" ||
                question.q_type === "short_answer") &&
                question.answers.length > 0 && (
                  <div className="text-xs text-slate-700 dark:text-slate-300 leading-relaxed">
                    <span className="font-semibold">参考答案：</span>
                    {question.answers.join("；")}
                  </div>
                )}

              {/* Explanation */}
              {question.explanation && (
                <div className="text-xs text-slate-600 dark:text-slate-400 leading-relaxed pt-1 border-t border-slate-200/60 dark:border-slate-700/60">
                  <span className="font-semibold text-slate-700 dark:text-slate-300">
                    解析：
                  </span>
                  {question.explanation}
                </div>
              )}

              {/* Ebbinghaus Schedule Info */}
              <div className="text-[11px] text-slate-500 dark:text-slate-400 flex items-center justify-between pt-0.5">
                {result.auto_cleared ? (
                  <span className="text-amber-600 dark:text-amber-400 font-semibold flex items-center gap-1">
                    <Trophy className="w-3.5 h-3.5" />
                    连续答对达标，已自动移入「已通关题库」！
                  </span>
                ) : (
                  <span>
                    艾宾浩斯阶段 Lv.{result.ebbinghaus_stage} · 连续答对{" "}
                    {result.streak} 次 · 下次复习：{result.next_review_at}
                  </span>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Bottom Action Footer */}
        <div className="px-5 py-3 border-t border-slate-100 dark:border-slate-800/90 bg-slate-50/70 dark:bg-slate-900/70 flex items-center justify-between gap-2">
          {result ? (
            <>
              <button
                onClick={handleMarkCleared}
                disabled={clearedManual || result.auto_cleared}
                className={`px-3 py-1.5 text-xs font-medium rounded-xl border transition-all flex items-center gap-1.5 ${
                  clearedManual || result.auto_cleared
                    ? "bg-amber-50 dark:bg-amber-500/15 text-amber-700 dark:text-amber-300 border-amber-200 dark:border-amber-500/30 cursor-default"
                    : "bg-white dark:bg-slate-800 text-slate-600 dark:text-slate-300 border-slate-200 dark:border-slate-700 hover:border-amber-400 hover:text-amber-600"
                }`}
              >
                <Trophy className="w-3.5 h-3.5" />
                {clearedManual || result.auto_cleared
                  ? "已移入通关库"
                  : "已熟练 · 一键通关"}
              </button>

              <button
                onClick={handleNextOrFinish}
                className="px-5 py-2 text-sm font-semibold rounded-xl bg-blue-600 hover:bg-blue-700 active:scale-[0.98] text-white shadow-sm transition-all flex items-center gap-1.5"
              >
                {isLast ? "完成本轮" : "下一题"}
                <kbd className="text-[10px] px-1.5 py-0.5 rounded bg-blue-500/80 font-mono">
                  Enter
                </kbd>
              </button>
            </>
          ) : (
            <>
              <div className="text-[11px] text-slate-400 dark:text-slate-500">
                {question.q_type === "single_choice" ||
                question.q_type === "multi_choice"
                  ? "快捷键：数字键 1-4 选择 · Enter 提交 · Esc 延后"
                  : question.q_type === "short_answer"
                  ? "快捷键：Ctrl + Enter 提交 · Esc 延后"
                  : "快捷键：Enter 提交 · Esc 延后"}
              </div>
              <button
                onClick={handleSubmit}
                disabled={submitting}
                className="px-5 py-2 text-sm font-semibold rounded-xl bg-blue-600 hover:bg-blue-700 disabled:opacity-50 active:scale-[0.98] text-white shadow-sm transition-all"
              >
                提交答案
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
