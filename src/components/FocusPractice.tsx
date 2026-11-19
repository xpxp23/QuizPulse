import { useCallback, useEffect, useRef, useState } from "react";
import {
  Check,
  CheckCircle2,
  FastForward,
  Flame,
  Play,
  RotateCcw,
  Sparkles,
  Trophy,
  XCircle,
} from "lucide-react";
import { api } from "../lib/api";
import { playFeedbackSound } from "../lib/sound";
import {
  QUESTION_TYPE_LABELS,
  type AppSettings,
  type Bank,
  type GradeResult,
  type QuizSessionQuestion,
} from "../types";

interface FocusPracticeProps {
  banks: Bank[];
  settings: AppSettings;
  onNotify: (type: "success" | "error", text: string) => void;
}

export function FocusPractice({
  banks,
  settings,
  onNotify,
}: FocusPracticeProps) {
  const [selectedBankId, setSelectedBankId] = useState("all");
  const [wrongOnly, setWrongOnly] = useState(false);
  const [batchSize, setBatchSize] = useState(10);

  const [queue, setQueue] = useState<QuizSessionQuestion[]>([]);
  const [idx, setIdx] = useState(0);
  const [selectedKeys, setSelectedKeys] = useState<string[]>([]);
  const [blankInputs, setBlankInputs] = useState<string[]>([]);
  const [shortInput, setShortInput] = useState("");
  const [result, setResult] = useState<GradeResult | null>(null);
  const [clearedManual, setClearedManual] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [sessionStats, setSessionStats] = useState({ done: 0, correct: 0 });
  const [finished, setFinished] = useState(false);

  const firstInputRef = useRef<HTMLInputElement | HTMLTextAreaElement | null>(
    null
  );

  const currentItem = queue[idx] ?? null;
  const question = currentItem?.question ?? null;

  const resetInputs = useCallback((item: QuizSessionQuestion | null) => {
    setSelectedKeys([]);
    setShortInput("");
    setResult(null);
    setClearedManual(false);
    if (item && item.question.q_type === "fill_blank") {
      setBlankInputs(new Array(Math.max(1, item.question.answers.length)).fill(""));
    } else {
      setBlankInputs([]);
    }
    setTimeout(() => firstInputRef.current?.focus(), 40);
  }, []);

  const startSession = async () => {
    try {
      const items = await api.pickFocusQuestions({
        count: batchSize,
        bankId: selectedBankId,
        wrongOnly,
      });
      if (items.length === 0) {
        onNotify(
          "error",
          wrongOnly
            ? "当前条件下没有错题，太棒了！"
            : "当前条件下没有可练习的活跃题目"
        );
        return;
      }
      setQueue(items);
      setIdx(0);
      setFinished(false);
      setSessionStats({ done: 0, correct: 0 });
      resetInputs(items[0]);
    } catch (e) {
      onNotify("error", String(e));
    }
  };

  const submitPayload = useCallback(
    async (payload: string[]) => {
      if (!question || result || submitting) return;
      setSubmitting(true);
      try {
        const res = await api.submitAnswer(question.id, payload);
        setResult(res);
        setSessionStats((s) => ({
          done: s.done + 1,
          correct: s.correct + (res.is_correct ? 1 : 0),
        }));
        playFeedbackSound(
          res.auto_cleared ? "clear" : res.is_correct ? "correct" : "wrong",
          settings.sound_enabled
        );
      } finally {
        setSubmitting(false);
      }
    },
    [question, result, submitting, settings.sound_enabled]
  );

  const handleToggleOption = useCallback(
    (key: string) => {
      if (!question || result) return;
      if (question.q_type === "single_choice") {
        setSelectedKeys([key]);
        if (settings.instant_single_choice) {
          submitPayload([key]);
        }
      } else if (question.q_type === "multi_choice") {
        setSelectedKeys((prev) =>
          prev.includes(key) ? prev.filter((k) => k !== key) : [...prev, key]
        );
      }
    },
    [question, result, settings.instant_single_choice, submitPayload]
  );

  const canSubmit = (() => {
    if (!question || result || submitting) return false;
    if (
      question.q_type === "single_choice" ||
      question.q_type === "multi_choice"
    ) {
      return selectedKeys.length > 0;
    }
    if (question.q_type === "fill_blank") {
      return blankInputs.some((v) => v.trim() !== "");
    }
    return shortInput.trim() !== "";
  })();

  const handleSubmit = useCallback(() => {
    if (!question || !canSubmit) return;
    if (
      question.q_type === "single_choice" ||
      question.q_type === "multi_choice"
    ) {
      submitPayload(selectedKeys);
    } else if (question.q_type === "fill_blank") {
      submitPayload(blankInputs);
    } else {
      submitPayload([shortInput]);
    }
  }, [question, canSubmit, selectedKeys, blankInputs, shortInput, submitPayload]);

  const handleNext = useCallback(() => {
    if (idx + 1 < queue.length) {
      const next = idx + 1;
      setIdx(next);
      resetInputs(queue[next]);
    } else {
      setFinished(true);
    }
  }, [idx, queue, resetInputs]);

  const handleSkip = useCallback(async () => {
    if (question && !result) {
      await api.skipQuestion(question.id).catch(() => {});
    }
    handleNext();
  }, [question, result, handleNext]);

  const handleOverride = async () => {
    if (!question || !result?.snapshot) return;
    const res = await api.manualOverridePass(question.id, result.snapshot);
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
            snapshot: null,
          }
        : res
    );
    setSessionStats((s) => ({ ...s, correct: s.correct + 1 }));
    playFeedbackSound(
      res.auto_cleared ? "clear" : "correct",
      settings.sound_enabled
    );
  };

  useEffect(() => {
    if (!currentItem || !question || finished) return;
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName;
      if (e.key === "Enter") {
        if (result) {
          e.preventDefault();
          handleNext();
          return;
        }
        if (tag === "TEXTAREA") {
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
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [
    currentItem,
    question,
    finished,
    result,
    handleNext,
    handleSubmit,
    handleToggleOption,
  ]);

  if (queue.length === 0 || finished) {
    return (
      <div className="max-w-2xl space-y-5">
        <div>
          <h1 className="text-xl font-bold tracking-tight flex items-center gap-2">
            <Flame className="w-5 h-5 text-orange-500" />
            专注刷题与错题突击
          </h1>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
            无需等待定时弹窗，按当前轮次随机或错题本连续集中练习，答完同样计入艾宾浩斯与轮次进度
          </p>
        </div>

        {finished && (
          <div className="rounded-2xl p-5 bg-emerald-50/80 dark:bg-emerald-500/15 border border-emerald-200 dark:border-emerald-500/30 flex items-center justify-between animate-fade-up">
            <div>
              <div className="text-sm font-bold text-emerald-800 dark:text-emerald-200">
                本轮专注练习完成！
              </div>
              <div className="text-xs text-emerald-700 dark:text-emerald-300 mt-1">
                作答 {sessionStats.done} 题 · 正确 {sessionStats.correct} 题（正确率{" "}
                {sessionStats.done > 0
                  ? Math.round((sessionStats.correct * 100) / sessionStats.done)
                  : 0}
                %）
              </div>
            </div>
            <button
              onClick={startSession}
              className="px-4 py-2 rounded-xl text-xs font-semibold bg-emerald-600 hover:bg-emerald-700 text-white flex items-center gap-1.5"
            >
              <RotateCcw className="w-3.5 h-3.5" />
              再来一组
            </button>
          </div>
        )}

        <div className="rounded-2xl p-5 bg-white/95 dark:bg-slate-900/90 border border-slate-200/80 dark:border-slate-800 space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="text-xs font-semibold block mb-1.5">
                练习范围
              </label>
              <select
                value={selectedBankId}
                onChange={(e) => setSelectedBankId(e.target.value)}
                className="w-full px-3 py-2 text-xs rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800"
              >
                <option value="all">全部已启用题库</option>
                {banks.map((b) => (
                  <option key={b.id} value={b.id}>
                    {b.name}（活跃 {b.active_count} 题）
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="text-xs font-semibold block mb-1.5">
                本组题量
              </label>
              <div className="flex gap-1.5">
                {[5, 10, 20, 50].map((n) => (
                  <button
                    key={n}
                    onClick={() => setBatchSize(n)}
                    className={`flex-1 py-2 rounded-xl text-xs font-semibold border transition-colors ${
                      batchSize === n
                        ? "bg-blue-600 text-white border-blue-600"
                        : "border-slate-200 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800"
                    }`}
                  >
                    {n} 题
                  </button>
                ))}
              </div>
            </div>
          </div>

          <label className="flex items-center justify-between py-2 border-t border-slate-100 dark:border-slate-800 cursor-pointer">
            <div>
              <div className="text-sm font-medium">仅刷错题本（错题突击）</div>
              <div className="text-xs text-slate-400">
                专门抽取历史上答错过的活跃题目集中攻克
              </div>
            </div>
            <input
              type="checkbox"
              checked={wrongOnly}
              onChange={(e) => setWrongOnly(e.target.checked)}
              className="w-4 h-4 accent-orange-500"
            />
          </label>

          <button
            onClick={startSession}
            className="w-full py-2.5 rounded-xl text-sm font-semibold bg-blue-600 hover:bg-blue-700 text-white shadow-sm transition-colors flex items-center justify-center gap-2"
          >
            <Play className="w-4 h-4" />
            开始专注练习
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-2xl space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="px-2.5 py-1 rounded-lg text-xs font-semibold bg-blue-50 dark:bg-blue-500/15 text-blue-600 dark:text-blue-300">
            {QUESTION_TYPE_LABELS[question!.q_type]}
          </span>
          <span className="text-xs text-slate-500">{question!.bank_name}</span>
          <span className="text-xs font-mono text-slate-400">
            第 {idx + 1} / {queue.length} 题
          </span>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleSkip}
            className="px-2.5 py-1 rounded-lg text-xs text-slate-500 hover:bg-slate-200/60 dark:hover:bg-slate-800 flex items-center gap-1"
          >
            <FastForward className="w-3.5 h-3.5" /> 跳过
          </button>
          <button
            onClick={() => setQueue([])}
            className="px-2.5 py-1 rounded-lg text-xs text-slate-500 hover:bg-slate-200/60 dark:hover:bg-slate-800"
          >
            退出练习
          </button>
        </div>
      </div>

      {/* Progress bar */}
      <div className="h-1.5 rounded-full bg-slate-200 dark:bg-slate-800 overflow-hidden">
        <div
          className="h-full bg-gradient-to-r from-blue-500 to-indigo-500 transition-all duration-300"
          style={{ width: `${((idx + (result ? 1 : 0)) / queue.length) * 100}%` }}
        />
      </div>

      <div className="rounded-2xl p-6 bg-white/95 dark:bg-slate-900/90 border border-slate-200/80 dark:border-slate-800 shadow-sm space-y-4">
        <div className="text-base font-medium leading-relaxed whitespace-pre-wrap">
          {question!.stem}
        </div>

        {(question!.q_type === "single_choice" ||
          question!.q_type === "multi_choice") && (
          <div className="space-y-2">
            {currentItem!.display_options.map((opt, i) => {
              const isSelected = selectedKeys.includes(opt.key);
              const isCorrectKey = question!.answers
                .map((a) => a.toUpperCase())
                .includes(opt.key.toUpperCase());
              let cls =
                "border-slate-200 dark:border-slate-700 hover:border-blue-400";
              if (result) {
                if (isCorrectKey)
                  cls = "border-emerald-500 bg-emerald-50/70 dark:bg-emerald-500/15";
                else if (isSelected)
                  cls = "border-rose-400 bg-rose-50/70 dark:bg-rose-500/15";
                else cls = "border-slate-200/50 dark:border-slate-800 opacity-55";
              } else if (isSelected) {
                cls = "border-blue-500 bg-blue-50/80 dark:bg-blue-500/20";
              }
              return (
                <button
                  key={opt.key}
                  disabled={!!result}
                  onClick={() => handleToggleOption(opt.key)}
                  className={`w-full text-left px-4 py-3 rounded-xl border transition-all flex items-start gap-3 ${cls}`}
                >
                  <span
                    className={`w-6 h-6 rounded-lg text-xs font-semibold flex items-center justify-center shrink-0 ${
                      isSelected
                        ? "bg-blue-600 text-white"
                        : "bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300"
                    }`}
                  >
                    {i + 1}
                  </span>
                  <span className="text-sm flex-1">{opt.text}</span>
                </button>
              );
            })}
          </div>
        )}

        {question!.q_type === "fill_blank" && (
          <div className="space-y-2.5">
            {question!.order_insensitive && (
              <div className="text-xs text-indigo-600 dark:text-indigo-400 flex items-center gap-1">
                <Sparkles className="w-3.5 h-3.5" /> 多空不限顺序
              </div>
            )}
            {blankInputs.map((val, i) => (
              <div key={i} className="flex items-center gap-2.5">
                <span className="text-xs text-slate-500 w-12">第 {i + 1} 空</span>
                <input
                  ref={
                    i === 0
                      ? (el) => {
                          firstInputRef.current = el;
                        }
                      : undefined
                  }
                  disabled={!!result}
                  value={val}
                  onChange={(e) =>
                    setBlankInputs(
                      blankInputs.map((v, j) => (j === i ? e.target.value : v))
                    )
                  }
                  className="flex-1 px-3.5 py-2 text-sm rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 outline-none focus:border-blue-500"
                />
              </div>
            ))}
          </div>
        )}

        {question!.q_type === "short_answer" && (
          <textarea
            ref={(el) => {
              firstInputRef.current = el;
            }}
            rows={4}
            disabled={!!result}
            value={shortInput}
            onChange={(e) => setShortInput(e.target.value)}
            placeholder="输入核心要点（Ctrl + Enter 提交）..."
            className="w-full px-3.5 py-2.5 text-sm rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 outline-none focus:border-blue-500 resize-none"
          />
        )}

        {result && (
          <div
            className={`rounded-xl p-4 border space-y-2.5 animate-fade-up ${
              result.is_correct
                ? "bg-emerald-50/70 dark:bg-emerald-500/10 border-emerald-200 dark:border-emerald-500/30"
                : "bg-rose-50/70 dark:bg-rose-500/10 border-rose-200 dark:border-rose-500/30"
            }`}
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                {result.is_correct ? (
                  <CheckCircle2 className="w-5 h-5 text-emerald-600" />
                ) : (
                  <XCircle className="w-5 h-5 text-rose-600" />
                )}
                <span className="text-sm font-semibold">
                  {result.is_correct ? "回答正确" : "还差一点"} ·{" "}
                  {result.score_percent}分
                </span>
              </div>
              {!result.is_correct &&
                question!.q_type === "short_answer" &&
                result.snapshot && (
                  <button
                    onClick={handleOverride}
                    className="px-2.5 py-1 rounded-lg text-xs font-medium bg-white dark:bg-slate-800 text-emerald-700 border border-emerald-300 flex items-center gap-1"
                  >
                    <Check className="w-3.5 h-3.5" /> 我答对意思了（算我过）
                  </button>
                )}
            </div>

            {result.keyword_details.length > 0 && (
              <div className="flex flex-wrap gap-1.5">
                {result.keyword_details.map((kw, i) => (
                  <span
                    key={i}
                    className={`px-2 py-0.5 text-xs rounded-md font-medium border ${
                      kw.is_hit
                        ? "bg-emerald-100/90 dark:bg-emerald-500/20 text-emerald-800 dark:text-emerald-200 border-emerald-300"
                        : "bg-rose-100/80 dark:bg-rose-500/20 text-rose-800 dark:text-rose-200 border-rose-300"
                    }`}
                  >
                    {kw.is_hit ? "✓ " : "✗ "}
                    {kw.point_label}
                  </span>
                ))}
              </div>
            )}

            {(question!.q_type === "fill_blank" ||
              question!.q_type === "short_answer") &&
              question!.answers.length > 0 && (
                <div className="text-xs text-slate-700 dark:text-slate-300">
                  <strong>参考答案：</strong>
                  {question!.answers.join("；")}
                </div>
              )}

            {question!.explanation && (
              <div className="text-xs text-slate-600 dark:text-slate-400">
                <strong>解析：</strong>
                {question!.explanation}
              </div>
            )}
          </div>
        )}

        <div className="flex items-center justify-between pt-2">
          {result ? (
            <>
              <button
                onClick={async () => {
                  await api.setQuestionStatus(question!.id, "cleared");
                  setClearedManual(true);
                  playFeedbackSound("clear", settings.sound_enabled);
                }}
                disabled={clearedManual || result.auto_cleared}
                className="px-3 py-1.5 rounded-xl text-xs font-medium border border-slate-200 dark:border-slate-700 flex items-center gap-1.5 disabled:opacity-60"
              >
                <Trophy className="w-3.5 h-3.5 text-amber-500" />
                {clearedManual || result.auto_cleared
                  ? "已移入通关库"
                  : "已熟练 · 一键通关"}
              </button>
              <button
                onClick={handleNext}
                className="px-5 py-2 rounded-xl text-sm font-semibold bg-blue-600 hover:bg-blue-700 text-white"
              >
                {idx + 1 >= queue.length ? "查看练习小结" : "下一题 (Enter)"}
              </button>
            </>
          ) : (
            <>
              <span className="text-xs text-slate-400">
                快捷键：1-6 选择 · Enter 提交
              </span>
              <button
                onClick={handleSubmit}
                disabled={!canSubmit}
                className="px-5 py-2 rounded-xl text-sm font-semibold bg-blue-600 hover:bg-blue-700 disabled:opacity-40 text-white"
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
