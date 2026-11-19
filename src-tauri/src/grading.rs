use crate::models::{KeywordMatchDetail, Question, QuestionType};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct RawGradeOutcome {
    pub is_correct: bool,
    pub score_percent: u8,
    pub earned_points: usize,
    pub total_points: usize,
    pub keyword_details: Vec<KeywordMatchDetail>,
    pub blank_results: Vec<bool>,
}

/// Normalize text for tolerant matching: lowercase, strip ASCII/CJK punctuation and whitespace.
pub fn normalize_text(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace() && !is_punctuation(*c))
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn is_punctuation(c: char) -> bool {
    c.is_ascii_punctuation()
        || matches!(
            c,
            '，' | '。' | '、' | '；' | '：' | '？' | '！' | '“' | '”' | '‘' | '’'
                | '（' | '）' | '【' | '】' | '《' | '》' | '—' | '…' | '·'
        )
}

/// Evaluate user answers against a Question across all 4 question types.
pub fn evaluate_question(
    question: &Question,
    user_answers: &[String],
    short_answer_pass_percent: u8,
) -> RawGradeOutcome {
    match question.q_type {
        QuestionType::SingleChoice => grade_single_choice(question, user_answers),
        QuestionType::MultiChoice => grade_multi_choice(question, user_answers),
        QuestionType::FillBlank => grade_fill_blank(question, user_answers),
        QuestionType::ShortAnswer => {
            grade_short_answer(question, user_answers, short_answer_pass_percent)
        }
    }
}

fn grade_single_choice(question: &Question, user_answers: &[String]) -> RawGradeOutcome {
    let expected = question
        .answers
        .first()
        .map(|s| s.trim().to_uppercase())
        .unwrap_or_default();
    let actual = user_answers
        .first()
        .map(|s| s.trim().to_uppercase())
        .unwrap_or_default();
    let is_correct = !expected.is_empty() && expected == actual;

    RawGradeOutcome {
        is_correct,
        score_percent: if is_correct { 100 } else { 0 },
        earned_points: usize::from(is_correct),
        total_points: 1,
        keyword_details: Vec::new(),
        blank_results: Vec::new(),
    }
}

fn grade_multi_choice(question: &Question, user_answers: &[String]) -> RawGradeOutcome {
    let expected: HashSet<String> = question
        .answers
        .iter()
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();
    let actual: HashSet<String> = user_answers
        .iter()
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();

    let is_correct = !expected.is_empty() && expected == actual;
    RawGradeOutcome {
        is_correct,
        score_percent: if is_correct { 100 } else { 0 },
        earned_points: usize::from(is_correct),
        total_points: 1,
        keyword_details: Vec::new(),
        blank_results: Vec::new(),
    }
}

fn blank_matches(user_raw: &str, expected_spec: &str) -> bool {
    let user_norm = normalize_text(user_raw);
    if user_norm.is_empty() {
        return false;
    }
    expected_spec
        .split('/')
        .map(normalize_text)
        .filter(|a| !a.is_empty())
        .any(|alias| alias == user_norm)
}

fn grade_fill_blank(question: &Question, user_answers: &[String]) -> RawGradeOutcome {
    let total = question.answers.len();
    if total == 0 {
        return RawGradeOutcome {
            is_correct: false,
            score_percent: 0,
            earned_points: 0,
            total_points: 0,
            keyword_details: Vec::new(),
            blank_results: Vec::new(),
        };
    }

    let mut blank_results = vec![false; total];
    let mut earned = 0usize;

    if question.order_insensitive {
        // Greedy matching: each expected blank slot can only be consumed once.
        let mut used_expected = vec![false; total];
        for (i, user_val) in user_answers.iter().take(total).enumerate() {
            for (j, exp_spec) in question.answers.iter().enumerate() {
                if !used_expected[j] && blank_matches(user_val, exp_spec) {
                    used_expected[j] = true;
                    blank_results[i] = true;
                    earned += 1;
                    break;
                }
            }
        }
    } else {
        for (i, exp_spec) in question.answers.iter().enumerate() {
            let user_val = user_answers.get(i).map(String::as_str).unwrap_or("");
            if blank_matches(user_val, exp_spec) {
                blank_results[i] = true;
                earned += 1;
            }
        }
    }

    let score_percent = ((earned * 100) / total) as u8;
    let is_correct = earned == total;

    RawGradeOutcome {
        is_correct,
        score_percent,
        earned_points: earned,
        total_points: total,
        keyword_details: Vec::new(),
        blank_results,
    }
}

fn grade_short_answer(
    question: &Question,
    user_answers: &[String],
    pass_percent: u8,
) -> RawGradeOutcome {
    let user_text = user_answers.first().map(String::as_str).unwrap_or("");
    let user_norm = normalize_text(user_text);

    let points: Vec<&String> = question
        .keywords
        .iter()
        .filter(|p| !p.trim().is_empty())
        .collect();

    if points.is_empty() {
        // Fallback: compare against reference answer if no keywords were specified
        let ref_norm = question
            .answers
            .first()
            .map(|s| normalize_text(s))
            .unwrap_or_default();
        let is_hit = !ref_norm.is_empty() && !user_norm.is_empty() && user_norm.contains(&ref_norm);
        return RawGradeOutcome {
            is_correct: is_hit,
            score_percent: if is_hit { 100 } else { 0 },
            earned_points: usize::from(is_hit),
            total_points: 1,
            keyword_details: Vec::new(),
            blank_results: Vec::new(),
        };
    }

    let mut details = Vec::with_capacity(points.len());
    let mut earned = 0usize;

    for point_spec in &points {
        let aliases: Vec<&str> = point_spec
            .split('/')
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .collect();
        let label = aliases.first().copied().unwrap_or(point_spec.as_str()).to_string();

        let matched = aliases.iter().find_map(|alias| {
            let alias_norm = normalize_text(alias);
            if !alias_norm.is_empty() && user_norm.contains(&alias_norm) {
                Some((*alias).to_string())
            } else {
                None
            }
        });

        let is_hit = matched.is_some();
        if is_hit {
            earned += 1;
        }
        details.push(KeywordMatchDetail {
            point_label: label,
            matched_alias: matched,
            is_hit,
        });
    }

    let total = points.len();
    let score_percent = ((earned * 100) / total) as u8;
    let is_correct = score_percent >= pass_percent;

    RawGradeOutcome {
        is_correct,
        score_percent,
        earned_points: earned,
        total_points: total,
        keyword_details: details,
        blank_results: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_question(q_type: QuestionType) -> Question {
        Question {
            id: "q1".into(),
            bank_id: "b1".into(),
            bank_name: "Test".into(),
            q_type,
            stem: "Test stem".into(),
            options: vec![],
            answers: vec![],
            keywords: vec![],
            explanation: String::new(),
            order_insensitive: false,
            ebbinghaus_stage: 0,
            streak: 0,
            wrong_count: 0,
            correct_count: 0,
            next_review_at: String::new(),
            status: "active".into(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn test_short_answer_synonyms() {
        let mut q = sample_question(QuestionType::ShortAnswer);
        q.keywords = vec![
            "三次握手/3次握手".into(),
            "可靠性/可靠传输".into(),
            "流量控制/滑动窗口".into(),
        ];
        let res = evaluate_question(&q, &["TCP通过3次握手建立连接，并利用滑动窗口实现控制".into()], 60);
        assert!(res.is_correct);
        assert_eq!(res.earned_points, 2);
        assert_eq!(res.total_points, 3);
        assert_eq!(res.score_percent, 66);
    }

    #[test]
    fn test_fill_blank_order_insensitive() {
        let mut q = sample_question(QuestionType::FillBlank);
        q.answers = vec!["TCP/传输控制协议".into(), "UDP/用户数据报协议".into()];
        q.order_insensitive = true;
        let res = evaluate_question(&q, &["udp".into(), "传输控制协议".into()], 60);
        assert!(res.is_correct);
        assert_eq!(res.blank_results, vec![true, true]);
    }
}
