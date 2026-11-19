import React, { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  BookOpen,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  Download,
  Flame,
  FolderOpen,
  Layers,
  Moon,
  Pause,
  Pencil,
  Play,
  Plus,
  RotateCcw,
  Search,
  Settings as SettingsIcon,
  Shuffle,
  Sparkles,
  Trash2,
  Trophy,
  Upload,
  Zap,
} from "lucide-react";
import { api } from "../lib/api";
import { applyTheme, formatCountdown } from "../lib/theme";
import { ConfirmModal } from "../components/ConfirmModal";
import { FocusPractice } from "../components/FocusPractice";
import { Heatmap } from "../components/Heatmap";
import { QuestionEditorModal } from "../components/QuestionEditorModal";
import {
  QUESTION_TYPE_LABELS,
  type AppSettings,
  type Bank,
  type DashboardStats,
  type HeatmapDay,
  type ImportSummary,
  type Question,
  type StudyOrder,
  type TimerStatus,
} from "../types";

type TabKey = "banks" | "questions" | "focus" | "settings";

function getInitialTab(): TabKey {
  const param = new URLSearchParams(window.location.search).get("tab");
  if (
    param === "banks" ||
    param === "questions" ||
    param === "focus" ||
    param === "settings"
  ) {
    return param;
  }
  return "banks";
}

export default function Dashboard() {
  const [tab, setTab] = useState<TabKey>(getInitialTab);
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [heatmap, setHeatmap] = useState<HeatmapDay[]>([]);
  const [banks, setBanks] = useState<Bank[]>([]);
  const [questions, setQuestions] = useState<Question[]>([]);
  const [totalQuestions, setTotalQuestions] = useState(0);
  const [page, setPage] = useState(1);
  const pageSize = 40;

  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [timer, setTimer] = useState<TimerStatus | null>(null);

  // Filters
  const [filterBankId, setFilterBankId] = useState("all");
  const [filterStatus, setFilterStatus] = useState("active");
  const [searchKeyword, setSearchKeyword] = useState("");

  // Create bank inline state
  const [newBankName, setNewBankName] = useState("");
  const [newBankDesc, setNewBankDesc] = useState("");
  const [showNewBank, setShowNewBank] = useState(false);

  // Modals
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingQuestion, setEditingQuestion] = useState<Question | null>(null);
  const [confirmState, setConfirmState] = useState<{
    open: boolean;
    title: string;
    message: string;
    confirmText?: string;
    danger?: boolean;
    onConfirm: () => void;
  }>({
    open: false,
    title: "",
    message: "",
    onConfirm: () => {},
  });

  // Notification banner
  const [toast, setToast] = useState<{
    type: "success" | "error";
    text: string;
  } | null>(null);
  const [importReport, setImportReport] = useState<ImportSummary | null>(null);

  const notify = useCallback((type: "success" | "error", text: string) => {
    setToast({ type, text });
    setTimeout(() => {
      setToast((prev) => (prev?.text === text ? null : prev));
    }, 4500);
  }, []);

  const refreshCore = useCallback(async () => {
    const [st, bk, cfg, tm, hm] = await Promise.all([
      api.getDashboardStats(),
      api.listBanks(),
      api.getSettings(),
      api.getTimerStatus(),
      api.getActivityHeatmap(91),
    ]);
    setStats(st);
    setBanks(bk);
    setSettings(cfg);
    setTimer(tm);
    setHeatmap(hm);
  }, []);

  const refreshQuestions = useCallback(async () => {
    const res = await api.listQuestions({
      bankId: filterBankId,
      status: filterStatus,
      keyword: searchKeyword,
      page,
      pageSize,
    });
    setQuestions(res.items);
    setTotalQuestions(res.total);
  }, [filterBankId, filterStatus, searchKeyword, page]);

  useEffect(() => {
    refreshCore().catch((e) => notify("error", String(e)));
  }, [refreshCore, notify]);

  useEffect(() => {
    setPage(1);
  }, [filterBankId, filterStatus, searchKeyword]);

  useEffect(() => {
    if (tab === "questions") {
      refreshQuestions().catch((e) => notify("error", String(e)));
    }
  }, [tab, refreshQuestions, notify]);

  useEffect(() => {
    if (!settings) return;
    return applyTheme(settings.theme);
  }, [settings]);

  // Listen for live timer updates, data:updated broadcasts from Popup, and tray nav events
  useEffect(() => {
    let unlistenTick: (() => void) | null = null;
    let unlistenData: (() => void) | null = null;
    let unlistenNav: (() => void) | null = null;
    let active = true;

    listen<TimerStatus>("timer:tick", (e) => {
      if (active) setTimer(e.payload);
    }).then((fn) => {
      if (active) unlistenTick = fn;
      else fn();
    });

    listen("data:updated", () => {
      if (!active) return;
      refreshCore().catch(() => {});
      if (tab === "questions") {
        refreshQuestions().catch(() => {});
      }
    }).then((fn) => {
      if (active) unlistenData = fn;
      else fn();
    });

    listen<string>("nav:tab", (e) => {
      if (
        active &&
        (e.payload === "banks" ||
          e.payload === "questions" ||
          e.payload === "focus" ||
          e.payload === "settings")
      ) {
        setTab(e.payload);
      }
    }).then((fn) => {
      if (active) unlistenNav = fn;
      else fn();
    });

    return () => {
      active = false;
      if (unlistenTick) unlistenTick();
      if (unlistenData) unlistenData();
      if (unlistenNav) unlistenNav();
    };
  }, [tab, refreshCore, refreshQuestions]);

  const handleExportTemplate = async (bankId?: string, bankName?: string) => {
    const defaultName = bankName
      ? `QuizPulse_${bankName}_可无损更新.xlsx`
      : "QuizPulse_题库标准模板.xlsx";
    const path = await save({
      defaultPath: defaultName,
      filters: [{ name: "Excel 工作簿", extensions: ["xlsx"] }],
    });
    if (!path) return;
    try {
      const count = await api.exportExcelTemplate(path, bankId);
      notify("success", `已成功导出全部 ${count} 条记录到 Excel 模板`);
    } catch (e) {
      notify("error", String(e));
    }
  };

  const handleImportExcel = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Excel 工作簿", extensions: ["xlsx", "xls"] }],
    });
    if (!selected || Array.isArray(selected)) return;
    try {
      const summary = await api.importExcelFile(selected, "默认导入题库");
      setImportReport(summary);
      notify(
        "success",
        `导入完成：新增 ${summary.inserted} 题，无损更新 ${summary.updated} 题，跳过 ${summary.skipped} 行`
      );
      await refreshCore();
      if (tab === "questions") await refreshQuestions();
    } catch (e) {
      notify("error", String(e));
    }
  };

  const handleCreateBank = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newBankName.trim()) return;
    try {
      await api.createBank(newBankName, newBankDesc);
      setNewBankName("");
      setNewBankDesc("");
      setShowNewBank(false);
      notify("success", "已创建新题库");
    } catch (err) {
      notify("error", String(err));
    }
  };

  const handleDeleteBank = (bank: Bank) => {
    setConfirmState({
      open: true,
      title: "删除题库",
      message: `确定永久删除题库「${bank.name}」及其全部 ${bank.total_count} 道题目吗？此操作不可撤销。`,
      confirmText: "确认删除",
      danger: true,
      onConfirm: async () => {
        setConfirmState((s) => ({ ...s, open: false }));
        await api.deleteBank(bank.id);
        notify("success", `已删除题库「${bank.name}」`);
      },
    });
  };

  const handleResetShuffleRound = () => {
    setConfirmState({
      open: true,
      title: "重新洗牌并开启新一轮",
      message:
        "将把所有活跃题目重新随机打乱，并开启新一轮不放回抽题（不会清除艾宾浩斯复习记录和已通关状态）。是否继续？",
      confirmText: "立即洗牌",
      danger: false,
      onConfirm: async () => {
        setConfirmState((s) => ({ ...s, open: false }));
        const rp = await api.startNewRound();
        notify("success", `已重新洗牌，当前进入第 ${rp.current_round} 轮`);
      },
    });
  };

  const handleSaveSettings = async () => {
    if (!settings) return;
    try {
      const saved = await api.saveSettings(settings);
      setSettings(saved);
      notify("success", "设置已保存并立即生效");
    } catch (e) {
      notify("error", String(e));
    }
  };

  const handlePickDataDir = async () => {
    if (!settings) return;
    const picked = await open({
      directory: true,
      multiple: false,
      title: "选择本地数据存储目录（可选网盘同步文件夹）",
    });
    if (picked && !Array.isArray(picked)) {
      setSettings({ ...settings, data_dir: picked });
    }
  };

  const handleTriggerNow = async () => {
    try {
      const ok = await api.triggerPopupNow();
      if (!ok) {
        notify("error", "当前启用的题库中没有待学习题目，请先启用题库或导入新题");
      }
    } catch (e) {
      notify("error", String(e));
    }
  };

  const timerReasonText = () => {
    if (!timer) return "初始化中";
    if (timer.popup_open) return "答题窗口已弹出";
    if (timer.paused_manual) return "已手动暂停";
    if (timer.paused_auto_reason === "fullscreen") return "全屏/锁屏自动勿扰";
    if (timer.paused_auto_reason === "idle") return "键鼠闲置自动暂停";
    return "后台静默计时中";
  };

  const rp = timer?.round_progress ?? stats?.round_progress ?? null;
  const totalPages = Math.max(1, Math.ceil(totalQuestions / pageSize));

  return (
    <div className="h-screen w-screen flex bg-transparent text-slate-900 dark:text-slate-100 overflow-hidden">
      {/* Sidebar */}
      <aside className="w-64 shrink-0 border-r border-slate-200/80 dark:border-slate-800/80 bg-white/75 dark:bg-slate-900/75 backdrop-blur-md flex flex-col p-4 justify-between">
        <div className="space-y-5">
          {/* Brand */}
          <div className="flex items-center justify-between px-2">
            <div className="flex items-center gap-2.5">
              <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-blue-600 to-indigo-600 text-white flex items-center justify-center shadow-sm">
                <Zap className="w-5 h-5" />
              </div>
              <div>
                <div className="font-bold text-base tracking-tight leading-none">
                  QuizPulse
                </div>
                <div className="text-[11px] text-slate-500 dark:text-slate-400 mt-1">
                  v1.3.0 · 定时学习助手
                </div>
              </div>
            </div>
          </div>

          {/* Live Timer & Round Card */}
          <div className="rounded-2xl p-4 bg-gradient-to-br from-slate-900 to-slate-800 dark:from-slate-800 dark:to-slate-900 text-white shadow-sm space-y-3">
            <div className="flex items-center justify-between text-xs text-slate-300">
              <span>距离下次抽题</span>
              <span className="px-2 py-0.5 rounded-full bg-white/10 text-[10px]">
                {timerReasonText()}
              </span>
            </div>
            <div className="text-3xl font-mono font-bold tracking-wider">
              {timer ? formatCountdown(timer.remaining_secs) : "--:--"}
            </div>

            {/* Shuffle Round Progress */}
            {rp && rp.total_active > 0 && (
              <div className="space-y-1 pt-1 border-t border-white/10">
                <div className="flex items-center justify-between text-[11px] text-slate-300">
                  <span>第 {rp.current_round} 轮进度</span>
                  <span className="font-mono">
                    {rp.completed_in_round} / {rp.total_active}
                  </span>
                </div>
                <div className="h-1.5 rounded-full bg-white/15 overflow-hidden">
                  <div
                    className="h-full bg-emerald-400 transition-all duration-300"
                    style={{
                      width: `${Math.min(
                        100,
                        Math.round((rp.completed_in_round * 100) / rp.total_active)
                      )}%`,
                    }}
                  />
                </div>
              </div>
            )}

            <div className="flex gap-1.5">
              <button
                onClick={handleTriggerNow}
                className="flex-1 py-1.5 px-2.5 rounded-xl bg-blue-500 hover:bg-blue-400 text-xs font-semibold transition-colors flex items-center justify-center gap-1"
              >
                <Sparkles className="w-3.5 h-3.5" />
                立即抽题
              </button>
              <button
                onClick={async () => {
                  await api.toggleTimerPause();
                }}
                className="py-1.5 px-2.5 rounded-xl bg-white/10 hover:bg-white/20 text-xs font-medium transition-colors"
                title={timer?.paused_manual ? "恢复计时" : "暂停计时"}
              >
                {timer?.paused_manual ? (
                  <Play className="w-3.5 h-3.5" />
                ) : (
                  <Pause className="w-3.5 h-3.5" />
                )}
              </button>
              <button
                onClick={async () => {
                  await api.snoozeDnd(60);
                  notify("success", "已开启 1 小时勿扰，60 分钟后再提醒");
                }}
                className="py-1.5 px-2.5 rounded-xl bg-white/10 hover:bg-white/20 text-xs font-medium transition-colors"
                title="一键勿扰 1 小时"
              >
                <Moon className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          {/* Nav Menu */}
          <nav className="space-y-1">
            {[
              { key: "banks", label: "总览与题库管理", icon: Layers },
              { key: "questions", label: "题目 / 错题 / 已通关", icon: BookOpen },
              { key: "focus", label: "专注刷题与突击", icon: Flame },
              { key: "settings", label: "定时与偏好设置", icon: SettingsIcon },
            ].map((item) => {
              const Icon = item.icon;
              const active = tab === item.key;
              return (
                <button
                  key={item.key}
                  onClick={() => setTab(item.key as TabKey)}
                  className={`w-full px-3 py-2.5 rounded-xl text-sm font-medium flex items-center gap-2.5 transition-all ${
                    active
                      ? "bg-blue-50 text-blue-700 dark:bg-blue-500/15 dark:text-blue-300"
                      : "text-slate-600 dark:text-slate-400 hover:bg-slate-200/50 dark:hover:bg-slate-800/70"
                  }`}
                >
                  <Icon className="w-4 h-4" />
                  {item.label}
                </button>
              );
            })}
          </nav>
        </div>

        <div className="px-2.5 py-2 rounded-xl bg-slate-200/50 dark:bg-slate-800/50 text-[11px] text-slate-500 dark:text-slate-400 leading-relaxed">
          全局快捷键{" "}
          <kbd className="px-1 py-0.5 rounded bg-white dark:bg-slate-700 font-mono text-[10px]">
            {settings?.global_shortcut || "未启用"}
          </kbd>{" "}
          随时呼出抽题；关闭主窗口自动释放内存。
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto p-6 space-y-6">
        {toast && (
          <div
            className={`px-4 py-3 rounded-xl text-sm font-medium flex items-center justify-between border animate-fade-up ${
              toast.type === "success"
                ? "bg-emerald-50 dark:bg-emerald-500/15 text-emerald-800 dark:text-emerald-200 border-emerald-200 dark:border-emerald-500/30"
                : "bg-rose-50 dark:bg-rose-500/15 text-rose-800 dark:text-rose-200 border-rose-200 dark:border-rose-500/30"
            }`}
          >
            <span>{toast.text}</span>
            <button
              onClick={() => setToast(null)}
              className="text-xs opacity-70 hover:opacity-100"
            >
              关闭
            </button>
          </div>
        )}

        {/* TAB 1: OVERVIEW & BANKS */}
        {tab === "banks" && (
          <div className="space-y-6">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <h1 className="text-xl font-bold tracking-tight">
                  题库管理与学习总览
                </h1>
                <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
                  勾选参与定时抽题的题库，或导出标准 Excel 模板批量编辑后无损导入
                </p>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={handleResetShuffleRound}
                  className="px-3 py-2 rounded-xl text-xs font-semibold border border-slate-200 dark:border-slate-700 bg-white/95 dark:bg-slate-900 hover:bg-slate-50 dark:hover:bg-slate-800 transition-colors flex items-center gap-1.5"
                  title="重新打乱所有活跃题顺序并开启新一轮"
                >
                  <Shuffle className="w-3.5 h-3.5 text-indigo-600" />
                  重新洗牌
                </button>
                <button
                  onClick={() => handleExportTemplate()}
                  className="px-3 py-2 rounded-xl text-xs font-semibold border border-slate-200 dark:border-slate-700 bg-white/95 dark:bg-slate-900 hover:bg-slate-50 dark:hover:bg-slate-800 transition-colors flex items-center gap-1.5"
                >
                  <Download className="w-3.5 h-3.5 text-blue-600" />
                  导出空白模板
                </button>
                <button
                  onClick={handleImportExcel}
                  className="px-4 py-2 rounded-xl text-xs font-semibold bg-blue-600 hover:bg-blue-700 text-white shadow-sm transition-colors flex items-center gap-1.5"
                >
                  <Upload className="w-3.5 h-3.5" />
                  导入 Excel 题库
                </button>
              </div>
            </div>

            {stats && (
              <div className="grid grid-cols-4 gap-3.5">
                {[
                  {
                    label: "待复习（已到点）",
                    value: stats.due_now,
                    sub: `活跃题库共 ${stats.active_questions} 题`,
                    accent: "text-blue-600 dark:text-blue-400",
                  },
                  {
                    label: "已通关题库",
                    value: stats.cleared_questions,
                    sub:
                      stats.total_questions > 0
                        ? `总掌握率 ${Math.round(
                            (stats.cleared_questions * 100) /
                              stats.total_questions
                          )}%`
                        : "不再自动弹出",
                    accent: "text-amber-600 dark:text-amber-400",
                  },
                  {
                    label: "今日已答",
                    value: stats.today_answered,
                    sub: `今日正确率 ${stats.today_accuracy}%`,
                    accent: "text-emerald-600 dark:text-emerald-400",
                  },
                  {
                    label: "当前轮次进度",
                    value: `${stats.round_progress.completed_in_round}/${stats.round_progress.total_active}`,
                    sub: `第 ${stats.round_progress.current_round} 轮 · 共 ${banks.length} 个题库`,
                    accent: "text-indigo-600 dark:text-indigo-400",
                  },
                ].map((card, i) => (
                  <div
                    key={i}
                    className="rounded-2xl p-4 bg-white/95 dark:bg-slate-900/90 border border-slate-200/80 dark:border-slate-800 shadow-sm"
                  >
                    <div className="text-xs text-slate-500 dark:text-slate-400">
                      {card.label}
                    </div>
                    <div className={`text-2xl font-bold mt-1.5 ${card.accent}`}>
                      {card.value}
                    </div>
                    <div className="text-[11px] text-slate-400 mt-1">
                      {card.sub}
                    </div>
                  </div>
                ))}
              </div>
            )}

            {/* 90-Day Heatmap */}
            <Heatmap data={heatmap} />

            {importReport && importReport.errors.length > 0 && (
              <div className="rounded-xl p-4 bg-amber-50 dark:bg-amber-500/10 border border-amber-200 dark:border-amber-500/30 space-y-1.5">
                <div className="text-xs font-semibold text-amber-800 dark:text-amber-300 flex justify-between">
                  <span>以下行因格式问题被跳过（其余已成功导入）：</span>
                  <button
                    onClick={() => setImportReport(null)}
                    className="underline"
                  >
                    收起
                  </button>
                </div>
                <ul className="text-xs text-amber-700 dark:text-amber-200 list-disc pl-4 space-y-0.5 max-h-28 overflow-y-auto">
                  {importReport.errors.map((err, idx) => (
                    <li key={idx}>{err}</li>
                  ))}
                </ul>
              </div>
            )}

            {/* Banks List with Mastery Progress Bars */}
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-200">
                  我的题库列表
                </h2>
                <button
                  onClick={() => setShowNewBank((v) => !v)}
                  className="text-xs font-medium text-blue-600 dark:text-blue-400 hover:underline flex items-center gap-1"
                >
                  <Plus className="w-3.5 h-3.5" />
                  新建题库
                </button>
              </div>

              {showNewBank && (
                <form
                  onSubmit={handleCreateBank}
                  className="rounded-2xl p-4 bg-white dark:bg-slate-900 border border-blue-200 dark:border-blue-500/40 flex flex-wrap gap-2.5 items-center"
                >
                  <input
                    type="text"
                    required
                    value={newBankName}
                    onChange={(e) => setNewBankName(e.target.value)}
                    placeholder="题库名称（如：考研政治、英语高频词）"
                    className="flex-1 min-w-[180px] px-3 py-1.5 text-sm rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 outline-none focus:border-blue-500"
                  />
                  <input
                    type="text"
                    value={newBankDesc}
                    onChange={(e) => setNewBankDesc(e.target.value)}
                    placeholder="简介（选填）"
                    className="flex-1 min-w-[180px] px-3 py-1.5 text-sm rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 outline-none focus:border-blue-500"
                  />
                  <button
                    type="submit"
                    className="px-4 py-1.5 rounded-xl text-xs font-semibold bg-blue-600 text-white hover:bg-blue-700"
                  >
                    确认创建
                  </button>
                </form>
              )}

              <div className="grid grid-cols-1 gap-3">
                {banks.map((b) => {
                  const masteryPct =
                    b.total_count > 0
                      ? Math.round((b.cleared_count * 100) / b.total_count)
                      : 0;
                  return (
                    <div
                      key={b.id}
                      className="rounded-2xl p-4 bg-white/95 dark:bg-slate-900/90 border border-slate-200/80 dark:border-slate-800 flex items-center justify-between gap-4 hover:border-slate-300 dark:hover:border-slate-700 transition-colors"
                    >
                      <div className="flex items-center gap-3.5 min-w-0 flex-1">
                        <button
                          onClick={() => api.toggleBank(b.id, !b.enabled)}
                          title={b.enabled ? "点击停用定时抽题" : "点击启用定时抽题"}
                          className={`w-11 h-6 rounded-full transition-colors p-0.5 shrink-0 ${
                            b.enabled
                              ? "bg-blue-600"
                              : "bg-slate-300 dark:bg-slate-700"
                          }`}
                        >
                          <div
                            className={`w-5 h-5 rounded-full bg-white shadow transition-transform ${
                              b.enabled ? "translate-x-5" : "translate-x-0"
                            }`}
                          />
                        </button>
                        <div className="min-w-0 flex-1 space-y-1.5">
                          <div className="font-semibold text-sm flex items-center gap-2">
                            <span className="truncate">{b.name}</span>
                            {b.due_count > 0 && b.enabled && (
                              <span className="px-2 py-0.5 text-[10px] font-semibold rounded-full bg-blue-50 dark:bg-blue-500/15 text-blue-600 dark:text-blue-300 shrink-0">
                                {b.due_count} 题待复习
                              </span>
                            )}
                          </div>
                          <div className="text-xs text-slate-500 dark:text-slate-400 truncate">
                            共 {b.total_count} 题 · 学习中 {b.active_count} · 已通关{" "}
                            {b.cleared_count}（掌握度 {masteryPct}%）
                            {b.description ? ` · ${b.description}` : ""}
                          </div>
                          <div className="h-1.5 w-full max-w-xs rounded-full bg-slate-100 dark:bg-slate-800 overflow-hidden">
                            <div
                              className="h-full bg-amber-500 rounded-full transition-all duration-300"
                              style={{ width: `${masteryPct}%` }}
                            />
                          </div>
                        </div>
                      </div>

                      <div className="flex items-center gap-1.5 shrink-0">
                        <button
                          onClick={() => handleExportTemplate(b.id, b.name)}
                          className="px-3 py-1.5 text-xs font-medium rounded-xl border border-slate-200 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-300 flex items-center gap-1"
                          title="导出该题库全部题目（含 ID，修改后重导不丢进度）"
                        >
                          <Download className="w-3.5 h-3.5" />
                          导出编辑
                        </button>
                        <button
                          onClick={() => handleDeleteBank(b)}
                          className="p-2 rounded-xl text-slate-400 hover:text-rose-600 hover:bg-rose-50 dark:hover:bg-rose-500/10 transition-colors"
                          title="删除题库"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        )}

        {/* TAB 2: QUESTIONS / WRONG / CLEARED WITH PAGINATION & INLINE EDITOR */}
        {tab === "questions" && (
          <div className="space-y-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-center gap-1.5 bg-slate-200/60 dark:bg-slate-800/80 p-1 rounded-xl">
                {[
                  { key: "active", label: "学习中" },
                  { key: "wrong", label: "错题本" },
                  { key: "cleared", label: "已通关题库" },
                  { key: "all", label: "全部" },
                ].map((f) => (
                  <button
                    key={f.key}
                    onClick={() => setFilterStatus(f.key)}
                    className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                      filterStatus === f.key
                        ? "bg-white dark:bg-slate-900 text-blue-600 dark:text-blue-400 shadow-sm"
                        : "text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200"
                    }`}
                  >
                    {f.label}
                  </button>
                ))}
              </div>

              <div className="flex flex-wrap items-center gap-2">
                <select
                  value={filterBankId}
                  onChange={(e) => setFilterBankId(e.target.value)}
                  className="px-3 py-1.5 text-xs rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 outline-none"
                >
                  <option value="all">全部题库</option>
                  {banks.map((b) => (
                    <option key={b.id} value={b.id}>
                      {b.name}
                    </option>
                  ))}
                </select>

                <div className="relative">
                  <Search className="w-3.5 h-3.5 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
                  <input
                    type="text"
                    value={searchKeyword}
                    onChange={(e) => setSearchKeyword(e.target.value)}
                    placeholder="搜索题干或解析..."
                    className="pl-8 pr-3 py-1.5 text-xs rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 outline-none focus:border-blue-500 w-48"
                  />
                </div>

                <button
                  onClick={() => {
                    setEditingQuestion(null);
                    setEditorOpen(true);
                  }}
                  className="px-3.5 py-1.5 rounded-xl text-xs font-semibold bg-blue-600 hover:bg-blue-700 text-white flex items-center gap-1 shadow-sm"
                >
                  <Plus className="w-3.5 h-3.5" />
                  新增题目
                </button>
              </div>
            </div>

            <div className="space-y-2.5">
              {questions.length === 0 ? (
                <div className="rounded-2xl p-10 text-center bg-white/95 dark:bg-slate-900/90 border border-slate-200/80 dark:border-slate-800 text-sm text-slate-400">
                  当前筛选条件下暂无题目
                </div>
              ) : (
                questions.map((q) => (
                  <div
                    key={q.id}
                    className="rounded-2xl p-4 bg-white/95 dark:bg-slate-900/90 border border-slate-200/80 dark:border-slate-800 flex items-start justify-between gap-4"
                  >
                    <div className="space-y-1.5 min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-1.5">
                        <span className="px-2 py-0.5 text-[11px] font-semibold rounded-md bg-blue-50 dark:bg-blue-500/15 text-blue-600 dark:text-blue-300">
                          {QUESTION_TYPE_LABELS[q.q_type]}
                        </span>
                        <span className="text-[11px] text-slate-500 dark:text-slate-400">
                          {q.bank_name}
                        </span>
                        {q.status === "cleared" ? (
                          <span className="px-2 py-0.5 text-[11px] font-semibold rounded-md bg-amber-50 dark:bg-amber-500/15 text-amber-700 dark:text-amber-300 flex items-center gap-1">
                            <Trophy className="w-3 h-3" /> 已通关
                          </span>
                        ) : (
                          <span className="text-[11px] text-slate-400">
                            Lv.{q.ebbinghaus_stage} · 连对 {q.streak} · 错{" "}
                            {q.wrong_count} 次 · 轮次 R{q.round_num}
                          </span>
                        )}
                      </div>
                      <div className="text-sm font-medium text-slate-800 dark:text-slate-100">
                        {q.stem}
                      </div>
                      <div className="text-xs text-slate-500 dark:text-slate-400">
                        答案：{q.answers.join(" / ")}
                        {q.keywords.length > 0
                          ? ` ｜ 采分点：${q.keywords.join("；")}`
                          : ""}
                      </div>
                    </div>

                    <div className="flex items-center gap-1 shrink-0">
                      <button
                        onClick={() => {
                          setEditingQuestion(q);
                          setEditorOpen(true);
                        }}
                        className="p-2 rounded-xl text-slate-400 hover:text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-500/10"
                        title="快速编辑此题"
                      >
                        <Pencil className="w-4 h-4" />
                      </button>
                      {q.status === "cleared" ? (
                        <button
                          onClick={async () => {
                            await api.setQuestionStatus(q.id, "active");
                          }}
                          className="px-2.5 py-1.5 text-xs font-medium rounded-xl border border-slate-200 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800 flex items-center gap-1 text-blue-600 dark:text-blue-400"
                        >
                          <RotateCcw className="w-3.5 h-3.5" />
                          移回复习
                        </button>
                      ) : (
                        <button
                          onClick={async () => {
                            await api.setQuestionStatus(q.id, "cleared");
                          }}
                          className="px-2.5 py-1.5 text-xs font-medium rounded-xl border border-slate-200 dark:border-slate-700 hover:bg-amber-50 dark:hover:bg-amber-500/10 flex items-center gap-1 text-amber-600 dark:text-amber-400"
                        >
                          <CheckCircle2 className="w-3.5 h-3.5" />
                          标记通关
                        </button>
                      )}
                      <button
                        onClick={async () => {
                          await api.deleteQuestion(q.id);
                        }}
                        className="p-2 rounded-xl text-slate-400 hover:text-rose-600 hover:bg-rose-50 dark:hover:bg-rose-500/10"
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>

            {/* Pagination Footer */}
            {totalQuestions > pageSize && (
              <div className="flex items-center justify-between pt-2 text-xs text-slate-500">
                <span>
                  共 {totalQuestions} 题 · 第 {page} / {totalPages} 页
                </span>
                <div className="flex items-center gap-1.5">
                  <button
                    disabled={page <= 1}
                    onClick={() => setPage((p) => Math.max(1, p - 1))}
                    className="px-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-700 disabled:opacity-40 flex items-center gap-1"
                  >
                    <ChevronLeft className="w-3.5 h-3.5" /> 上一页
                  </button>
                  <button
                    disabled={page >= totalPages}
                    onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                    className="px-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-700 disabled:opacity-40 flex items-center gap-1"
                  >
                    下一页 <ChevronRight className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            )}
          </div>
        )}

        {/* TAB 3: FOCUS PRACTICE */}
        {tab === "focus" && settings && (
          <FocusPractice banks={banks} settings={settings} onNotify={notify} />
        )}

        {/* TAB 4: SETTINGS */}
        {tab === "settings" && settings && (
          <div className="max-w-2xl space-y-5">
            <div className="flex items-center justify-between">
              <div>
                <h1 className="text-xl font-bold tracking-tight">
                  定时与偏好设置
                </h1>
                <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
                  所有参数保存后立即作用于后台调度器、全局快捷键与弹窗界面
                </p>
              </div>
              <button
                onClick={handleSaveSettings}
                className="px-5 py-2 rounded-xl text-xs font-semibold bg-blue-600 hover:bg-blue-700 text-white shadow-sm"
              >
                保存设置
              </button>
            </div>

            <div className="rounded-2xl p-5 bg-white/95 dark:bg-slate-900/90 border border-slate-200/80 dark:border-slate-800 space-y-4">
              {/* Study Order Mode (Shuffle-Bag Random!) */}
              <div className="space-y-2">
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  出题顺序策略
                </label>
                <div className="grid grid-cols-3 gap-2">
                  {[
                    {
                      k: "shuffle_round",
                      title: "随机不放回（推荐）",
                      desc: "全库打乱各答一遍后再洗牌",
                    },
                    {
                      k: "ebbinghaus",
                      title: "艾宾浩斯智能",
                      desc: "严格按到期复习与错题优先",
                    },
                    {
                      k: "sequential",
                      title: "顺序逐题刷",
                      desc: "按导入先后顺序逐轮推进",
                    },
                  ].map((m) => (
                    <button
                      key={m.k}
                      onClick={() =>
                        setSettings({
                          ...settings,
                          study_order: m.k as StudyOrder,
                        })
                      }
                      className={`p-3 rounded-xl text-left border transition-all ${
                        settings.study_order === m.k
                          ? "bg-blue-50 dark:bg-blue-500/15 border-blue-500 text-blue-800 dark:text-blue-200"
                          : "border-slate-200 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800"
                      }`}
                    >
                      <div className="text-xs font-bold">{m.title}</div>
                      <div className="text-[11px] opacity-75 mt-0.5">
                        {m.desc}
                      </div>
                    </button>
                  ))}
                </div>
              </div>

              {/* Interval */}
              <div className="space-y-2 pt-2 border-t border-slate-100 dark:border-slate-800">
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  定时抽题间隔（分钟）
                </label>
                <div className="flex flex-wrap items-center gap-2">
                  {[25, 40, 60, 90].map((m) => (
                    <button
                      key={m}
                      onClick={() =>
                        setSettings({ ...settings, interval_minutes: m })
                      }
                      className={`px-3 py-1.5 rounded-xl text-xs font-medium border transition-colors ${
                        settings.interval_minutes === m
                          ? "bg-blue-600 text-white border-blue-600"
                          : "border-slate-200 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800"
                      }`}
                    >
                      {m} 分钟
                    </button>
                  ))}
                  <input
                    type="number"
                    min={1}
                    max={480}
                    value={settings.interval_minutes}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        interval_minutes: Math.max(1, Number(e.target.value)),
                      })
                    }
                    className="w-24 px-3 py-1.5 text-xs rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 outline-none"
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4 pt-2 border-t border-slate-100 dark:border-slate-800">
                <div>
                  <label className="text-xs font-semibold block mb-1.5">
                    每次弹出题量（题）
                  </label>
                  <input
                    type="number"
                    min={1}
                    max={20}
                    value={settings.questions_per_session}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        questions_per_session: Math.max(
                          1,
                          Number(e.target.value)
                        ),
                      })
                    }
                    className="w-full px-3 py-2 text-sm rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800"
                  />
                </div>
                <div>
                  <label className="text-xs font-semibold block mb-1.5">
                    一键延后时长（分钟）
                  </label>
                  <input
                    type="number"
                    min={1}
                    max={60}
                    value={settings.snooze_minutes}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        snooze_minutes: Math.max(1, Number(e.target.value)),
                      })
                    }
                    className="w-full px-3 py-2 text-sm rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800"
                  />
                </div>
                <div>
                  <label className="text-xs font-semibold block mb-1.5">
                    键鼠无操作自动暂停（分钟，0关闭）
                  </label>
                  <input
                    type="number"
                    min={0}
                    max={120}
                    value={settings.idle_pause_minutes}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        idle_pause_minutes: Math.max(0, Number(e.target.value)),
                      })
                    }
                    className="w-full px-3 py-2 text-sm rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800"
                  />
                </div>
                <div>
                  <label className="text-xs font-semibold block mb-1.5">
                    连续答对自动通关阈值（次，0关闭）
                  </label>
                  <input
                    type="number"
                    min={0}
                    max={20}
                    value={settings.auto_clear_streak}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        auto_clear_streak: Math.max(0, Number(e.target.value)),
                      })
                    }
                    className="w-full px-3 py-2 text-sm rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800"
                  />
                </div>
                <div>
                  <label className="text-xs font-semibold block mb-1.5">
                    简答题及格得分率（%）
                  </label>
                  <input
                    type="number"
                    min={10}
                    max={100}
                    value={settings.short_answer_pass_percent}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        short_answer_pass_percent: Math.min(
                          100,
                          Math.max(10, Number(e.target.value))
                        ),
                      })
                    }
                    className="w-full px-3 py-2 text-sm rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800"
                  />
                </div>
                <div>
                  <label className="text-xs font-semibold block mb-1.5">
                    全局呼出快捷键（留空禁用，如 Alt+Q）
                  </label>
                  <input
                    type="text"
                    value={settings.global_shortcut}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        global_shortcut: e.target.value,
                      })
                    }
                    placeholder="Alt+Q 或 Ctrl+Shift+Q"
                    className="w-full px-3 py-2 text-sm font-mono rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800"
                  />
                </div>
                <div>
                  <label className="text-xs font-semibold block mb-1.5">
                    弹窗出现位置
                  </label>
                  <select
                    value={settings.popup_position}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        popup_position: e.target.value as
                          | "center"
                          | "bottom_right",
                      })
                    }
                    className="w-full px-3 py-2 text-sm rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800"
                  >
                    <option value="bottom_right">屏幕右下角（温和悬浮）</option>
                    <option value="center">屏幕正中央</option>
                  </select>
                </div>
                <div>
                  <label className="text-xs font-semibold block mb-1.5">
                    界面主题外观
                  </label>
                  <div className="flex gap-1.5">
                    {[
                      { k: "system", l: "系统" },
                      { k: "light", l: "浅色" },
                      { k: "dark", l: "深色" },
                    ].map((t) => (
                      <button
                        key={t.k}
                        onClick={() =>
                          setSettings({
                            ...settings,
                            theme: t.k as AppSettings["theme"],
                          })
                        }
                        className={`flex-1 py-2 rounded-xl text-xs font-medium border ${
                          settings.theme === t.k
                            ? "bg-blue-600 text-white border-blue-600"
                            : "border-slate-200 dark:border-slate-700"
                        }`}
                      >
                        {t.l}
                      </button>
                    ))}
                  </div>
                </div>
              </div>

              {/* Toggles */}
              <div className="space-y-3 pt-3 border-t border-slate-100 dark:border-slate-800">
                {[
                  {
                    key: "interleave_due_wrong",
                    label: "随机/顺序模式下仍优先穿插 1 道到期错题",
                  },
                  {
                    key: "instant_single_choice",
                    label: "单选题极速模式（点击或按数字键直接提交）",
                  },
                  {
                    key: "sound_enabled",
                    label: "开启轻柔答题音效（Web Audio 零资源合成）",
                  },
                  {
                    key: "fullscreen_dnd",
                    label: "全屏 / 游戏 / 投屏 / 锁屏自动勿扰暂停",
                  },
                  {
                    key: "shuffle_options",
                    label: "选择题自动随机打乱选项顺序",
                  },
                  {
                    key: "auto_start",
                    label: "Windows 开机静默自启（缩进系统托盘）",
                  },
                ].map((item) => {
                  const checked = settings[
                    item.key as keyof AppSettings
                  ] as boolean;
                  return (
                    <label
                      key={item.key}
                      className="flex items-center justify-between text-sm cursor-pointer"
                    >
                      <span className="text-slate-700 dark:text-slate-200">
                        {item.label}
                      </span>
                      <input
                        type="checkbox"
                        checked={checked}
                        onChange={(e) =>
                          setSettings({
                            ...settings,
                            [item.key]: e.target.checked,
                          })
                        }
                        className="w-4 h-4 accent-blue-600 rounded"
                      />
                    </label>
                  );
                })}
              </div>

              {/* Data Dir */}
              <div className="pt-3 border-t border-slate-100 dark:border-slate-800">
                <label className="text-xs font-semibold block mb-1.5">
                  本地数据存储目录（指向 OneDrive / 坚果云即可多端同步，采用 SQLite 安全快照迁移）
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={settings.data_dir}
                    onChange={(e) =>
                      setSettings({ ...settings, data_dir: e.target.value })
                    }
                    className="flex-1 px-3 py-2 text-xs font-mono rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800"
                  />
                  <button
                    onClick={handlePickDataDir}
                    className="px-3.5 py-2 text-xs font-medium rounded-xl border border-slate-200 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800 flex items-center gap-1"
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                    选择目录
                  </button>
                </div>
              </div>
            </div>
          </div>
        )}
      </main>

      {/* Modals */}
      <ConfirmModal
        open={confirmState.open}
        title={confirmState.title}
        message={confirmState.message}
        confirmText={confirmState.confirmText}
        danger={confirmState.danger}
        onConfirm={confirmState.onConfirm}
        onCancel={() => setConfirmState((s) => ({ ...s, open: false }))}
      />

      <QuestionEditorModal
        open={editorOpen}
        banks={banks}
        initialQuestion={editingQuestion}
        defaultBankId={filterBankId !== "all" ? filterBankId : banks[0]?.id}
        onSaved={() => {
          setEditorOpen(false);
          notify("success", editingQuestion ? "题目已无损更新" : "新题目已添加");
        }}
        onClose={() => setEditorOpen(false)}
      />
    </div>
  );
}
