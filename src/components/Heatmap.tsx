import { useMemo } from "react";
import type { HeatmapDay } from "../types";

interface HeatmapProps {
  data: HeatmapDay[];
  days?: number;
}

export function Heatmap({ data, days = 91 }: HeatmapProps) {
  const { cells, totalAnswered, activeDays } = useMemo(() => {
    const map = new Map<string, HeatmapDay>();
    let total = 0;
    for (const d of data) {
      map.set(d.date, d);
      total += d.total;
    }

    const today = new Date();
    const list: { date: string; total: number; correct: number }[] = [];
    for (let i = days - 1; i >= 0; i--) {
      const dt = new Date(today);
      dt.setDate(today.getDate() - i);
      const yyyy = dt.getFullYear();
      const mm = String(dt.getMonth() + 1).padStart(2, "0");
      const dd = String(dt.getDate()).padStart(2, "0");
      const key = `${yyyy}-${mm}-${dd}`;
      const hit = map.get(key);
      list.push({
        date: key,
        total: hit?.total ?? 0,
        correct: hit?.correct ?? 0,
      });
    }

    return {
      cells: list,
      totalAnswered: total,
      activeDays: data.filter((d) => d.total > 0).length,
    };
  }, [data, days]);

  const colorClass = (count: number) => {
    if (count <= 0) return "bg-slate-100 dark:bg-slate-800/90";
    if (count <= 3) return "bg-emerald-200 dark:bg-emerald-900/80";
    if (count <= 8) return "bg-emerald-400 dark:bg-emerald-700";
    if (count <= 16) return "bg-emerald-500 dark:bg-emerald-500";
    return "bg-emerald-600 dark:bg-emerald-400";
  };

  // Group into columns of 7 days (13 weeks)
  const weeks: typeof cells[] = [];
  for (let i = 0; i < cells.length; i += 7) {
    weeks.push(cells.slice(i, i + 7));
  }

  return (
    <div className="rounded-2xl p-4 bg-white/95 dark:bg-slate-900/90 border border-slate-200/80 dark:border-slate-800 space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h3 className="text-xs font-semibold text-slate-700 dark:text-slate-200">
            近 90 天学习热力图
          </h3>
          <p className="text-[11px] text-slate-400 mt-0.5">
            累计作答 {totalAnswered} 题 · 打卡 {activeDays} 天
          </p>
        </div>
        <div className="flex items-center gap-1 text-[10px] text-slate-400">
          <span>少</span>
          <span className="w-2.5 h-2.5 rounded-[3px] bg-slate-100 dark:bg-slate-800" />
          <span className="w-2.5 h-2.5 rounded-[3px] bg-emerald-200 dark:bg-emerald-900/80" />
          <span className="w-2.5 h-2.5 rounded-[3px] bg-emerald-400 dark:bg-emerald-700" />
          <span className="w-2.5 h-2.5 rounded-[3px] bg-emerald-500" />
          <span className="w-2.5 h-2.5 rounded-[3px] bg-emerald-600 dark:bg-emerald-400" />
          <span>多</span>
        </div>
      </div>

      <div className="flex gap-1.5 overflow-x-auto pb-1">
        {weeks.map((week, wIdx) => (
          <div key={wIdx} className="flex flex-col gap-1.5">
            {week.map((day) => {
              const acc =
                day.total > 0
                  ? Math.round((day.correct * 100) / day.total)
                  : 0;
              return (
                <div
                  key={day.date}
                  title={
                    day.total > 0
                      ? `${day.date}：作答 ${day.total} 题（正确率 ${acc}%）`
                      : `${day.date}：暂无答题记录`
                  }
                  className={`w-3.5 h-3.5 rounded-[4px] transition-transform hover:scale-125 ${colorClass(
                    day.total
                  )}`}
                />
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}
