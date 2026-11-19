use crate::db::{ensure_bank_by_name, list_all_questions_by_bank, now_iso, random_shuffle_key};
use crate::models::{ChoiceOption, ImportSummary, QuestionType};
use calamine::{open_workbook, Reader, Xlsx};
use rusqlite::{params, Connection, OptionalExtension};
use rust_xlsxwriter::{Color, DataValidation, Format, FormatAlign, FormatBorder, Workbook};
use std::collections::HashSet;
use uuid::Uuid;

const HEADERS: [&str; 14] = [
    "ID（新增留空，修改勿动）",
    "题库分类",
    "题型",
    "题干",
    "选项A",
    "选项B",
    "选项C",
    "选项D",
    "选项E",
    "选项F",
    "标准答案",
    "简答采分关键词（分号隔点，斜杠同义词）",
    "多空不限顺序",
    "解析",
];

/// Export either a blank standard template (4 examples) or ALL questions of an existing bank (zero LIMIT truncation).
pub fn export_excel_template(
    conn: &Connection,
    target_path: &str,
    bank_id: Option<&str>,
) -> Result<usize, String> {
    let mut workbook = Workbook::new();

    let header_fmt = Format::new()
        .set_bold()
        .set_font_name("Microsoft YaHei")
        .set_font_size(11)
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x1E3A8A))
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);

    let cell_fmt = Format::new()
        .set_font_name("Microsoft YaHei")
        .set_font_size(10.5)
        .set_text_wrap()
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);

    let id_fmt = Format::new()
        .set_font_name("Consolas")
        .set_font_size(9.5)
        .set_font_color(Color::RGB(0x64748B))
        .set_background_color(Color::RGB(0xF8FAFC))
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);

    let sheet = workbook.add_worksheet();
    sheet.set_name("题库模板").map_err(|e| e.to_string())?;
    sheet.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;
    sheet.set_row_height(0, 28).map_err(|e| e.to_string())?;

    let col_widths = [24.0, 18.0, 10.0, 42.0, 20.0, 20.0, 20.0, 20.0, 14.0, 14.0, 24.0, 34.0, 14.0, 36.0];
    for (col_idx, (title, width)) in HEADERS.iter().zip(col_widths.iter()).enumerate() {
        sheet
            .write_string_with_format(0, col_idx as u16, *title, &header_fmt)
            .map_err(|e| e.to_string())?;
        sheet
            .set_column_width(col_idx as u16, *width)
            .map_err(|e| e.to_string())?;
    }

    let exported_count: usize;
    let max_dv_row: u32;
    if let Some(bid) = bank_id.filter(|s| !s.is_empty()) {
        let questions = list_all_questions_by_bank(conn, bid)?;
        exported_count = questions.len();
        max_dv_row = (exported_count + 2000) as u32;
        for (i, q) in questions.iter().enumerate() {
            let row = (i + 1) as u32;
            let opt_by_key = |k: &str| -> String {
                q.options
                    .iter()
                    .find(|o| o.key.eq_ignore_ascii_case(k))
                    .map(|o| o.text.clone())
                    .unwrap_or_default()
            };
            let ans_str = match q.q_type {
                QuestionType::SingleChoice | QuestionType::MultiChoice => q.answers.join(""),
                QuestionType::FillBlank => q.answers.join(";"),
                QuestionType::ShortAnswer => q.answers.first().cloned().unwrap_or_default(),
            };
            let kw_str = q.keywords.join(";");
            let order_str = if q.order_insensitive { "是" } else { "否" };

            let vals = [
                q.id.as_str(),
                q.bank_name.as_str(),
                q.q_type.to_cn(),
                q.stem.as_str(),
                &opt_by_key("A"),
                &opt_by_key("B"),
                &opt_by_key("C"),
                &opt_by_key("D"),
                &opt_by_key("E"),
                &opt_by_key("F"),
                &ans_str,
                &kw_str,
                order_str,
                q.explanation.as_str(),
            ];
            for (c, val) in vals.iter().enumerate() {
                let fmt = if c == 0 { &id_fmt } else { &cell_fmt };
                sheet
                    .write_string_with_format(row, c as u16, *val, fmt)
                    .map_err(|e| e.to_string())?;
            }
        }
    } else {
        let examples = [
            [
                "",
                "示例题库",
                "单选",
                "HTTPS 默认使用的端口号是多少？",
                "80",
                "443",
                "8080",
                "22",
                "",
                "",
                "B",
                "",
                "否",
                "HTTP 默认端口 80，HTTPS 默认端口 443。",
            ],
            [
                "",
                "示例题库",
                "多选",
                "以下哪些属于常见的关系型数据库？",
                "PostgreSQL",
                "SQLite",
                "Redis",
                "MySQL",
                "",
                "",
                "ABD",
                "",
                "否",
                "Redis 属于内存键值型非关系型数据库。",
            ],
            [
                "",
                "示例题库",
                "填空",
                "传输层最常见的两个核心协议是 ____ 和 ____ 。",
                "",
                "",
                "",
                "",
                "",
                "",
                "TCP/传输控制协议;UDP/用户数据报协议",
                "",
                "是",
                "每空用分号 ; 隔开，同一空的同义答案用斜杠 / 隔开；设置“多空不限顺序=是”后两空顺序互换也算对。",
            ],
            [
                "",
                "示例题库",
                "简答",
                "简述数据库索引的优点与代价。",
                "",
                "",
                "",
                "",
                "",
                "",
                "优点是大幅加快查询与排序速度；代价是占用额外磁盘空间，且会降低写入（增删改）性能。",
                "加快查询/查询速度/检索速度;占用空间/磁盘空间/存储开销;降低写入/影响增删改/写性能",
                "否",
                "每个采分点用分号 ; 隔开，同一采分点的同义关键词用斜杠 / 隔开，命中任意一个即算该点得分。",
            ],
        ];
        exported_count = examples.len();
        max_dv_row = 2000;
        for (i, row_vals) in examples.iter().enumerate() {
            let row = (i + 1) as u32;
            for (c, val) in row_vals.iter().enumerate() {
                let fmt = if c == 0 { &id_fmt } else { &cell_fmt };
                sheet
                    .write_string_with_format(row, c as u16, *val, fmt)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    let type_dv = DataValidation::new()
        .allow_list_strings(&["单选", "多选", "填空", "简答"])
        .map_err(|e| e.to_string())?;
    sheet
        .add_data_validation(1, 2, max_dv_row, 2, &type_dv)
        .map_err(|e| e.to_string())?;

    let order_dv = DataValidation::new()
        .allow_list_strings(&["否", "是"])
        .map_err(|e| e.to_string())?;
    sheet
        .add_data_validation(1, 12, max_dv_row, 12, &order_dv)
        .map_err(|e| e.to_string())?;

    let guide = workbook.add_worksheet();
    guide.set_name("填写说明").map_err(|e| e.to_string())?;
    guide.set_column_width(0, 24.0).map_err(|e| e.to_string())?;
    guide.set_column_width(1, 86.0).map_err(|e| e.to_string())?;

    let guide_rows = [
        ("列名称", "填写规则说明"),
        ("ID（新增留空，修改勿动）", "新增题目时请留空；导出现有题库修改后重新导入时，保留原 ID 可实现“无损更新”，不会清除艾宾浩斯复习进度与已通关状态。"),
        ("题库分类", "填写题库名称（如“考研政治”“英语高频词”）。不存在的题库会在导入时自动创建。"),
        ("题型", "支持下拉选择：单选、多选、填空、简答。"),
        ("题干", "题目正文内容。填空题建议用 ____ 标出空格位置。"),
        ("选项A ~ 选项F", "仅单选、多选题填写；填空和简答题留空即可。弹窗出题时支持自动打乱选项顺序。"),
        ("标准答案", "单选填字母如 B；多选填字母如 ABD（或 A,B,D）；填空题多空用分号 ; 隔开，单空同义答案用斜杠 / 隔开；简答题填完整参考答案。"),
        ("简答采分关键词", "仅简答题填写。不同采分点用分号 ;（中英文均可）隔开；同一采分点的同义表达用斜杠 / 隔开，例如：三次握手/3次握手;流量控制/滑动窗口。"),
        ("多空不限顺序", "仅多空填空题有效。选“是”表示各空答案顺序互换均判对；默认选“否”。"),
        ("解析", "选填，答题提交后展示在卡片底部辅助巩固记忆。"),
    ];
    for (r, (k, v)) in guide_rows.iter().enumerate() {
        let fmt = if r == 0 { &header_fmt } else { &cell_fmt };
        guide.write_string_with_format(r as u32, 0, *k, fmt).map_err(|e| e.to_string())?;
        guide.write_string_with_format(r as u32, 1, *v, fmt).map_err(|e| e.to_string())?;
    }

    workbook.save(target_path).map_err(|e| format!("保存 Excel 失败: {e}"))?;
    Ok(exported_count)
}

fn cell_str(row: &[calamine::Data], idx: usize) -> String {
    row.get(idx)
        .map(|d| d.to_string().trim().to_string())
        .unwrap_or_default()
}

fn split_points(raw: &str) -> Vec<String> {
    raw.split(|c| c == ';' || c == '；' || c == '\n')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// Strictly extract unique A-F option keys in order (fixes accidental extraction of arbitrary English words).
fn parse_choice_answers(raw: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for ch in raw.chars() {
        let up = ch.to_ascii_uppercase();
        if matches!(up, 'A' | 'B' | 'C' | 'D' | 'E' | 'F') && seen.insert(up) {
            out.push(up.to_string());
        }
    }
    out
}

/// Lossless incremental Excel import inside a single SQLite transaction.
pub fn import_excel_file(
    conn: &mut Connection,
    file_path: &str,
    fallback_bank_name: &str,
) -> Result<ImportSummary, String> {
    let mut workbook: Xlsx<_> =
        open_workbook(file_path).map_err(|e| format!("无法读取 Excel 文件: {e}"))?;

    let range = workbook
        .worksheet_range("题库模板")
        .or_else(|_| {
            workbook
                .worksheet_range_at(0)
                .ok_or_else(|| calamine::XlsxError::WorksheetNotFound("Sheet1".into()))?
        })
        .map_err(|e| format!("读取工作表失败: {e}"))?;

    let mut summary = ImportSummary {
        inserted: 0,
        updated: 0,
        skipped: 0,
        errors: Vec::new(),
    };

    let tx = conn.transaction().map_err(|e| format!("开启事务失败: {e}"))?;
    let now = now_iso();

    for (row_idx, row) in range.rows().enumerate().skip(1) {
        let excel_row_num = row_idx + 1;
        let raw_id = cell_str(row, 0);
        let bank_name_cell = cell_str(row, 1);
        let q_type_cell = cell_str(row, 2);
        let stem = cell_str(row, 3);

        if raw_id.is_empty() && q_type_cell.is_empty() && stem.is_empty() {
            continue;
        }

        if stem.is_empty() {
            summary.skipped += 1;
            summary.errors.push(format!("第 {excel_row_num} 行缺少题干，已跳过"));
            continue;
        }

        let Some(q_type) = QuestionType::from_cn_or_en(&q_type_cell) else {
            summary.skipped += 1;
            summary.errors.push(format!(
                "第 {excel_row_num} 行题型「{q_type_cell}」无法识别（请填 单选/多选/填空/简答）"
            ));
            continue;
        };

        let bank_name = if bank_name_cell.is_empty() {
            fallback_bank_name
        } else {
            bank_name_cell.as_str()
        };
        let bank_id = ensure_bank_by_name(&tx, bank_name)?;

        let option_keys = ["A", "B", "C", "D", "E", "F"];
        let mut options = Vec::new();
        for (i, key) in option_keys.iter().enumerate() {
            let txt = cell_str(row, 4 + i);
            if !txt.is_empty() {
                options.push(ChoiceOption {
                    key: (*key).to_string(),
                    text: txt,
                });
            }
        }

        let raw_answer = cell_str(row, 10);
        let raw_keywords = cell_str(row, 11);
        let raw_order = cell_str(row, 12);
        let explanation = cell_str(row, 13);

        let answers = match q_type {
            QuestionType::SingleChoice => {
                let parsed = parse_choice_answers(&raw_answer);
                parsed.into_iter().take(1).collect()
            }
            QuestionType::MultiChoice => parse_choice_answers(&raw_answer),
            QuestionType::FillBlank => split_points(&raw_answer),
            QuestionType::ShortAnswer => {
                if raw_answer.is_empty() {
                    Vec::new()
                } else {
                    vec![raw_answer]
                }
            }
        };
        let keywords = split_points(&raw_keywords);
        let order_insensitive = matches!(raw_order.as_str(), "是" | "true" | "TRUE" | "1" | "yes");

        if matches!(q_type, QuestionType::SingleChoice | QuestionType::MultiChoice)
            && (options.len() < 2 || answers.is_empty())
        {
            summary.skipped += 1;
            summary.errors.push(format!(
                "第 {excel_row_num} 行选择题至少需要 2 个选项且填写标准答案（A-F）"
            ));
            continue;
        }
        if matches!(q_type, QuestionType::FillBlank) && answers.is_empty() {
            summary.skipped += 1;
            summary.errors.push(format!("第 {excel_row_num} 行填空题未填写标准答案"));
            continue;
        }
        if matches!(q_type, QuestionType::ShortAnswer) && keywords.is_empty() && answers.is_empty() {
            summary.skipped += 1;
            summary.errors.push(format!(
                "第 {excel_row_num} 行简答题未填写采分关键词或参考答案"
            ));
            continue;
        }

        let opts_json = serde_json::to_string(&options).unwrap_or_else(|_| "[]".into());
        let ans_json = serde_json::to_string(&answers).unwrap_or_else(|_| "[]".into());
        let kw_json = serde_json::to_string(&keywords).unwrap_or_else(|_| "[]".into());

        let exists = if !raw_id.is_empty() {
            tx.query_row(
                "SELECT id FROM questions WHERE id = ?1",
                params![raw_id],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .is_some()
        } else {
            false
        };

        if exists {
            tx.execute(
                "UPDATE questions
                 SET bank_id = ?1, q_type = ?2, stem = ?3, options_json = ?4,
                     answer_json = ?5, keywords_json = ?6, explanation = ?7,
                     order_insensitive = ?8, updated_at = ?9
                 WHERE id = ?10",
                params![
                    bank_id,
                    q_type.as_str(),
                    stem,
                    opts_json,
                    ans_json,
                    kw_json,
                    explanation,
                    i64::from(order_insensitive),
                    now,
                    raw_id
                ],
            )
            .map_err(|e| e.to_string())?;
            summary.updated += 1;
        } else {
            let new_id = if raw_id.is_empty() {
                format!("q_{}", &Uuid::new_v4().simple().to_string()[..12])
            } else {
                raw_id
            };
            let sk = random_shuffle_key();
            tx.execute(
                "INSERT INTO questions (
                    id, bank_id, q_type, stem, options_json, answer_json, keywords_json,
                    explanation, order_insensitive, next_review_at, status, round_num, shuffle_key,
                    created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'active', 0, ?11, ?10, ?10)",
                params![
                    new_id,
                    bank_id,
                    q_type.as_str(),
                    stem,
                    opts_json,
                    ans_json,
                    kw_json,
                    explanation,
                    i64::from(order_insensitive),
                    now,
                    sk
                ],
            )
            .map_err(|e| e.to_string())?;
            summary.inserted += 1;
        }
    }

    tx.commit().map_err(|e| format!("提交导入事务失败: {e}"))?;
    Ok(summary)
}
