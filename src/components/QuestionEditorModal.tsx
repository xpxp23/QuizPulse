import { useEffect, useState } from "react";
import { Plus, Trash2, X } from "lucide-react";
import { api } from "../lib/api";
import type {
  Bank,
  ChoiceOption,
  Question,
  QuestionType,
} from "../types";

interface QuestionEditorModalProps {
  open: boolean;
  banks: Bank[];
  initialQuestion: Question | null;
  defaultBankId?: string;
  onSaved: (saved: Question) => void;
  onClose: () => void;
}

const OPTION_KEYS = ["A", "B", "C", "D", "E", "F"];

export function QuestionEditorModal({
  open,
  banks,
  initialQuestion,
  defaultBankId,
  onSaved,
  onClose,
}: QuestionEditorModalProps) {
  const [bankId, setBankId] = useState("");
  const [qType, setQType] = useState<QuestionType>("single_choice");
  const [stem, setStem] = useState("");
  const [options, setOptions] = useState<ChoiceOption[]>([]);
  const [choiceAnswers, setChoiceAnswers] = useState<string[]>([]);
  const [blankAnswers, setBlankAnswers] = useState<string[]>([""]);
  const [shortAnswerRef, setShortAnswerRef] = useState("");
  // Each scoring point is an array of synonym aliases
  const [keywordGroups, setKeywordGroups] = useState<string[][]>([[""]]);
  const [aliasDrafts, setAliasDrafts] = useState<string[]>([""]);
  const [orderInsensitive, setOrderInsensitive] = useState(false);
  const [explanation, setExplanation] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!open) return;
    setError("");
    if (initialQuestion) {
      setBankId(initialQuestion.bank_id);
      setQType(initialQuestion.q_type);
      setStem(initialQuestion.stem);
      const mappedOpts = OPTION_KEYS.map((k) => {
        const found = initialQuestion.options.find(
          (o) => o.key.toUpperCase() === k
        );
        return { key: k, text: found?.text ?? "" };
      });
      setOptions(mappedOpts);
      setChoiceAnswers(
        initialQuestion.q_type === "single_choice" ||
          initialQuestion.q_type === "multi_choice"
          ? initialQuestion.answers.map((a) => a.toUpperCase())
          : []
      );
      setBlankAnswers(
        initialQuestion.q_type === "fill_blank" &&
          initialQuestion.answers.length > 0
          ? [...initialQuestion.answers]
          : [""]
      );
      setShortAnswerRef(
        initialQuestion.q_type === "short_answer"
          ? initialQuestion.answers[0] ?? ""
          : ""
      );
      const groups =
        initialQuestion.keywords.length > 0
          ? initialQuestion.keywords.map((pt) =>
              pt
                .split("/")
                .map((s) => s.trim())
                .filter(Boolean)
            )
          : [[]];
      setKeywordGroups(groups);
      setAliasDrafts(new Array(groups.length).fill(""));
      setOrderInsensitive(initialQuestion.order_insensitive);
      setExplanation(initialQuestion.explanation);
    } else {
      setBankId(defaultBankId || banks[0]?.id || "");
      setQType("single_choice");
      setStem("");
      setOptions(OPTION_KEYS.map((k) => ({ key: k, text: "" })));
      setChoiceAnswers(["A"]);
      setBlankAnswers([""]);
      setShortAnswerRef("");
      setKeywordGroups([[]]);
      setAliasDrafts([""]);
      setOrderInsensitive(false);
      setExplanation("");
    }
  }, [open, initialQuestion, banks, defaultBankId]);

  if (!open) return null;

  const toggleChoiceAnswer = (key: string) => {
    if (qType === "single_choice") {
      setChoiceAnswers([key]);
    } else {
      setChoiceAnswers((prev) =>
        prev.includes(key) ? prev.filter((k) => k !== key) : [...prev, key]
      );
    }
  };

  const addAliasToPoint = (pointIdx: number) => {
    const val = (aliasDrafts[pointIdx] ?? "").trim();
    if (!val) return;
    setKeywordGroups((prev) =>
      prev.map((grp, i) =>
        i === pointIdx && !grp.includes(val) ? [...grp, val] : grp
      )
    );
    setAliasDrafts((prev) => prev.map((v, i) => (i === pointIdx ? "" : v)));
  };

  const removeAlias = (pointIdx: number, alias: string) => {
    setKeywordGroups((prev) =>
      prev.map((grp, i) =>
        i === pointIdx ? grp.filter((a) => a !== alias) : grp
      )
    );
  };

  const handleSave = async () => {
    if (!bankId) {
      setError("请先选择所属题库");
      return;
    }
    if (!stem.trim()) {
      setError("题干不能为空");
      return;
    }

    const activeOptions = options.filter((o) => o.text.trim() !== "");
    let answers: string[] = [];
    let keywords: string[] = [];

    if (qType === "single_choice" || qType === "multi_choice") {
      if (activeOptions.length < 2) {
        setError("选择题至少需要填写 2 个选项");
        return;
      }
      const validKeys = new Set(activeOptions.map((o) => o.key));
      answers = choiceAnswers.filter((k) => validKeys.has(k));
      if (answers.length === 0) {
        setError("请勾选至少一个正确答案选项");
        return;
      }
    } else if (qType === "fill_blank") {
      answers = blankAnswers.map((s) => s.trim()).filter(Boolean);
      if (answers.length === 0) {
        setError("请填写至少一个空格的标准答案");
        return;
      }
    } else {
      // Flush any uncommitted alias drafts automatically so user doesn't lose typed keywords!
      const mergedGroups = keywordGroups
        .map((grp, idx) => {
          const draft = (aliasDrafts[idx] ?? "").trim();
          const combined = draft && !grp.includes(draft) ? [...grp, draft] : grp;
          return combined.filter(Boolean);
        })
        .filter((grp) => grp.length > 0);
      keywords = mergedGroups.map((grp) => grp.join("/"));
      if (shortAnswerRef.trim()) {
        answers = [shortAnswerRef.trim()];
      }
      if (keywords.length === 0 && answers.length === 0) {
        setError("简答题请至少添加一个采分关键词或填写参考答案");
        return;
      }
    }

    setSaving(true);
    setError("");
    try {
      const saved = await api.upsertQuestion({
        id: initialQuestion?.id ?? null,
        bank_id: bankId,
        q_type: qType,
        stem: stem.trim(),
        options:
          qType === "single_choice" || qType === "multi_choice"
            ? activeOptions
            : [],
        answers,
        keywords,
        explanation: explanation.trim(),
        order_insensitive: qType === "fill_blank" ? orderInsensitive : false,
      });
      onSaved(saved);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/50 backdrop-blur-[2px] p-4"
      onClick={onClose}
    >
      <div
        className="w-full max-w-xl max-h-[88vh] flex flex-col rounded-2xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700/80 shadow-2xl overflow-hidden animate-fade-up"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="px-5 py-3.5 border-b border-slate-100 dark:border-slate-800 flex items-center justify-between">
          <h3 className="text-sm font-bold text-slate-900 dark:text-slate-100">
            {initialQuestion ? "快速编辑题目（无损保留复习进度）" : "新建单道题目"}
          </h3>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-slate-400 hover:text-slate-700 dark:hover:text-slate-200"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-5 space-y-4">
          {error && (
            <div className="px-3 py-2 rounded-xl bg-rose-50 dark:bg-rose-500/15 text-rose-700 dark:text-rose-300 text-xs border border-rose-200 dark:border-rose-500/30">
              {error}
            </div>
          )}

          {/* Bank & Type */}
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="text-xs font-semibold block mb-1">所属题库</label>
              <select
                value={bankId}
                onChange={(e) => setBankId(e.target.value)}
                className="w-full px-3 py-1.5 text-xs rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800"
              >
                {banks.map((b) => (
                  <option key={b.id} value={b.id}>
                    {b.name}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="text-xs font-semibold block mb-1">题型</label>
              <select
                value={qType}
                onChange={(e) => {
                  const nextType = e.target.value as QuestionType;
                  setQType(nextType);
                  if (nextType === "single_choice" && choiceAnswers.length > 1) {
                    setChoiceAnswers([choiceAnswers[0]]);
                  }
                }}
                className="w-full px-3 py-1.5 text-xs rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800"
              >
                <option value="single_choice">单选题</option>
                <option value="multi_choice">多选题</option>
                <option value="fill_blank">填空题</option>
                <option value="short_answer">简答题（按点给分）</option>
              </select>
            </div>
          </div>

          {/* Stem */}
          <div>
            <label className="text-xs font-semibold block mb-1">题干</label>
            <textarea
              rows={3}
              value={stem}
              onChange={(e) => setStem(e.target.value)}
              placeholder="输入题目内容（填空题可用 ____ 标出空格）..."
              className="w-full px-3 py-2 text-xs rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 outline-none focus:border-blue-500 resize-none"
            />
          </div>

          {/* Choice Options */}
          {(qType === "single_choice" || qType === "multi_choice") && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <label className="text-xs font-semibold">
                  选项与标准答案（点击左侧字母标为正确）
                </label>
                <span className="text-[11px] text-slate-400">
                  {qType === "single_choice" ? "单选" : "可多选"}
                </span>
              </div>
              {options.map((opt, idx) => {
                const isAns = choiceAnswers.includes(opt.key);
                return (
                  <div key={opt.key} className="flex items-center gap-2">
                    <button
                      type="button"
                      onClick={() => toggleChoiceAnswer(opt.key)}
                      className={`w-7 h-7 rounded-lg text-xs font-bold shrink-0 transition-colors ${
                        isAns
                          ? "bg-emerald-600 text-white"
                          : "bg-slate-100 dark:bg-slate-800 text-slate-500 hover:bg-slate-200"
                      }`}
                      title="点击设为正确答案"
                    >
                      {opt.key}
                    </button>
                    <input
                      type="text"
                      value={opt.text}
                      onChange={(e) => {
                        const next = [...options];
                        next[idx] = { ...opt, text: e.target.value };
                        setOptions(next);
                      }}
                      placeholder={`选项 ${opt.key}${idx >= 4 ? "（选填）" : ""}`}
                      className="flex-1 px-3 py-1.5 text-xs rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 outline-none focus:border-blue-500"
                    />
                  </div>
                );
              })}
            </div>
          )}

          {/* Fill Blank */}
          {qType === "fill_blank" && (
            <div className="space-y-2.5">
              <div className="flex items-center justify-between">
                <label className="text-xs font-semibold">
                  各空答案（同义答案用 / 隔开，如 TCP/传输控制协议）
                </label>
                <button
                  type="button"
                  onClick={() => setBlankAnswers([...blankAnswers, ""])}
                  className="text-xs text-blue-600 hover:underline flex items-center gap-0.5"
                >
                  <Plus className="w-3.5 h-3.5" /> 加一空
                </button>
              </div>
              {blankAnswers.map((val, idx) => (
                <div key={idx} className="flex items-center gap-2">
                  <span className="text-[11px] text-slate-500 w-12">
                    第 {idx + 1} 空
                  </span>
                  <input
                    type="text"
                    value={val}
                    onChange={(e) => {
                      const next = [...blankAnswers];
                      next[idx] = e.target.value;
                      setBlankAnswers(next);
                    }}
                    className="flex-1 px-3 py-1.5 text-xs rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 outline-none focus:border-blue-500"
                  />
                  {blankAnswers.length > 1 && (
                    <button
                      type="button"
                      onClick={() =>
                        setBlankAnswers(blankAnswers.filter((_, i) => i !== idx))
                      }
                      className="p-1 text-slate-400 hover:text-rose-500"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  )}
                </div>
              ))}
              <label className="flex items-center gap-2 text-xs text-slate-600 dark:text-slate-300 cursor-pointer pt-1">
                <input
                  type="checkbox"
                  checked={orderInsensitive}
                  onChange={(e) => setOrderInsensitive(e.target.checked)}
                  className="accent-blue-600"
                />
                多空不限顺序（各空答案互换位置均判对）
              </label>
            </div>
          )}

          {/* Short Answer Visual Tag-Chip Editor */}
          {qType === "short_answer" && (
            <div className="space-y-3">
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <label className="text-xs font-semibold">
                    采分点关键词组（每组命中任意一个同义词即得分）
                  </label>
                  <button
                    type="button"
                    onClick={() => {
                      setKeywordGroups([...keywordGroups, []]);
                      setAliasDrafts([...aliasDrafts, ""]);
                    }}
                    className="text-xs text-blue-600 hover:underline flex items-center gap-0.5"
                  >
                    <Plus className="w-3.5 h-3.5" /> 添加采分点
                  </button>
                </div>

                {keywordGroups.map((grp, pIdx) => (
                  <div
                    key={pIdx}
                    className="p-2.5 rounded-xl bg-slate-50 dark:bg-slate-800/70 border border-slate-200/80 dark:border-slate-700 space-y-2"
                  >
                    <div className="flex items-center justify-between text-[11px] text-slate-500">
                      <span>采分点 {pIdx + 1}</span>
                      {keywordGroups.length > 1 && (
                        <button
                          type="button"
                          onClick={() => {
                            setKeywordGroups(
                              keywordGroups.filter((_, i) => i !== pIdx)
                            );
                            setAliasDrafts(
                              aliasDrafts.filter((_, i) => i !== pIdx)
                            );
                          }}
                          className="text-slate-400 hover:text-rose-500"
                        >
                          删除此点
                        </button>
                      )}
                    </div>
                    <div className="flex flex-wrap gap-1.5 items-center">
                      {grp.map((alias) => (
                        <span
                          key={alias}
                          className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-xs bg-blue-50 dark:bg-blue-500/20 text-blue-700 dark:text-blue-200 border border-blue-200 dark:border-blue-500/30"
                        >
                          {alias}
                          <button
                            type="button"
                            onClick={() => removeAlias(pIdx, alias)}
                            className="hover:text-rose-500"
                          >
                            <X className="w-3 h-3" />
                          </button>
                        </span>
                      ))}
                      <input
                        type="text"
                        value={aliasDrafts[pIdx] ?? ""}
                        onChange={(e) =>
                          setAliasDrafts(
                            aliasDrafts.map((v, i) =>
                              i === pIdx ? e.target.value : v
                            )
                          )
                        }
                        onKeyDown={(e) => {
                          if (e.key === "Enter") {
                            e.preventDefault();
                            addAliasToPoint(pIdx);
                          }
                        }}
                        placeholder="输入关键词回车添加同义词..."
                        className="flex-1 min-w-[140px] px-2 py-1 text-xs bg-transparent outline-none"
                      />
                    </div>
                  </div>
                ))}
              </div>

              <div>
                <label className="text-xs font-semibold block mb-1">
                  完整参考答案（提交后展示）
                </label>
                <textarea
                  rows={2}
                  value={shortAnswerRef}
                  onChange={(e) => setShortAnswerRef(e.target.value)}
                  className="w-full px-3 py-2 text-xs rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 outline-none focus:border-blue-500 resize-none"
                />
              </div>
            </div>
          )}

          {/* Explanation */}
          <div>
            <label className="text-xs font-semibold block mb-1">解析（选填）</label>
            <textarea
              rows={2}
              value={explanation}
              onChange={(e) => setExplanation(e.target.value)}
              placeholder="补充记忆口诀或原理解析..."
              className="w-full px-3 py-2 text-xs rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 outline-none focus:border-blue-500 resize-none"
            />
          </div>
        </div>

        <div className="px-5 py-3 border-t border-slate-100 dark:border-slate-800 flex justify-end gap-2">
          <button
            onClick={onClose}
            className="px-4 py-1.5 rounded-xl text-xs font-medium border border-slate-200 dark:border-slate-700"
          >
            取消
          </button>
          <button
            onClick={handleSave}
            disabled={saving}
            className="px-5 py-1.5 rounded-xl text-xs font-semibold bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white shadow-sm"
          >
            {saving ? "保存中..." : "保存题目"}
          </button>
        </div>
      </div>
    </div>
  );
}
