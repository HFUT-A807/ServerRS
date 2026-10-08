use std::collections::HashSet;

use serde_json::Value;

use crate::domain::depression::DepressionScale;
use crate::shared::error::AppError;

fn config_error(message: &str) -> AppError {
    AppError::internal(format!("invalid depression scale configuration: {message}"))
}

/// Answers contain configured option scores, never option indexes.
/// Reversed items must encode their final contribution in options[].score.
pub(super) fn compute_score(scale: &DepressionScale, answers: &Value) -> Result<i32, AppError> {
    let questions = scale
        .questions
        .as_array()
        .filter(|q| !q.is_empty())
        .ok_or_else(|| config_error("questions must be a non-empty array"))?;
    let mut ids = HashSet::new();
    let mut total = 0_i32;
    match answers {
        Value::Array(a) if a.len() == questions.len() => {}
        Value::Object(a) if a.len() == questions.len() => {}
        _ => {
            return Err(AppError::Validation(format!(
                "answers must contain exactly {} answers, as an array in question order or an object keyed by question ID",
                questions.len()
            )));
        }
    }
    for (index, question) in questions.iter().enumerate() {
        let id = question
            .get("id")
            .and_then(Value::as_u64)
            .filter(|id| *id > 0)
            .ok_or_else(|| config_error("question IDs must be positive integers"))?
            .to_string();
        if !ids.insert(id.clone()) {
            return Err(config_error("duplicate question ID"));
        }
        let options = question
            .get("options")
            .and_then(Value::as_array)
            .filter(|o| !o.is_empty())
            .ok_or_else(|| config_error("each question must have options"))?;
        let allowed = options
            .iter()
            .map(|o| {
                o.get("score")
                    .and_then(Value::as_i64)
                    .and_then(|s| i32::try_from(s).ok())
                    .ok_or_else(|| config_error("option scores must be 32-bit integers"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let answer = match answers {
            Value::Array(a) => &a[index],
            Value::Object(a) => a
                .get(&id)
                .ok_or_else(|| AppError::Validation(format!("missing answer for question {id}")))?,
            _ => unreachable!(),
        };
        let score = answer
            .as_i64()
            .and_then(|s| i32::try_from(s).ok())
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "answer for question {id} must be an integer option score"
                ))
            })?;
        if !allowed.contains(&score) {
            return Err(AppError::Validation(format!(
                "answer for question {id} is not a configured option score"
            )));
        }
        total = total
            .checked_add(score)
            .ok_or_else(|| AppError::Validation("assessment score overflow".into()))?;
    }
    if total < i32::from(scale.min_score) || total > i32::from(scale.max_score) {
        return Err(AppError::Validation(format!(
            "assessment score {total} is outside scale range {}..={}",
            scale.min_score, scale.max_score
        )));
    }
    Ok(total)
}

fn bounds(range: &Value) -> Result<(i32, i32), AppError> {
    let pair = if range.get("min").is_some() || range.get("max").is_some() {
        let min = range.get("min").and_then(Value::as_i64);
        let max = range.get("max").and_then(Value::as_i64);
        min.zip(max)
            .ok_or_else(|| config_error("both min and max must be integers"))?
    } else {
        let text = range
            .get("range")
            .and_then(Value::as_str)
            .ok_or_else(|| config_error("range requires min/max or a range string"))?;
        // Legacy seed data uses e.g. "0-4"; explicit min/max also supports negative scores.
        let (min, max) = text
            .split_once('-')
            .ok_or_else(|| config_error("invalid range string"))?;
        let min = min
            .trim()
            .parse::<i64>()
            .map_err(|_| config_error("invalid range minimum"))?;
        let max = max
            .trim()
            .parse::<i64>()
            .map_err(|_| config_error("invalid range maximum"))?;
        (min, max)
    };
    let min = i32::try_from(pair.0).map_err(|_| config_error("range minimum overflow"))?;
    let max = i32::try_from(pair.1).map_err(|_| config_error("range maximum overflow"))?;
    if min > max {
        return Err(config_error("range minimum exceeds maximum"));
    }
    Ok((min, max))
}

/// Invalid or ambiguous configuration must not silently classify everyone in the first range.
pub(super) fn determine_severity(scale: &DepressionScale, score: i32) -> Result<String, AppError> {
    let ranges = scale
        .severity_ranges
        .as_array()
        .filter(|r| !r.is_empty())
        .ok_or_else(|| config_error("severity ranges must be a non-empty array"))?;
    let mut parsed = ranges
        .iter()
        .map(|range| {
            let (min, max) = bounds(range)?;
            let label = range
                .get("level")
                .or_else(|| range.get("label"))
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| config_error("severity label is missing"))?;
            Ok((min, max, label))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    parsed.sort_by_key(|r| r.0);
    for window in parsed.windows(2) {
        if i64::from(window[0].1) + 1 != i64::from(window[1].0) {
            return Err(config_error("severity ranges overlap or contain a gap"));
        }
    }
    if scale.min_score > scale.max_score
        || parsed[0].0 != i32::from(scale.min_score)
        || parsed[parsed.len() - 1].1 != i32::from(scale.max_score)
    {
        return Err(config_error(
            "severity ranges must cover the configured score bounds exactly",
        ));
    }
    // Historical scores outside the configured range are unclassified, never given a false label.
    Ok(parsed
        .iter()
        .find(|(min, max, _)| (*min..=*max).contains(&score))
        .map(|(_, _, label)| (*label).to_owned())
        .unwrap_or_else(|| "unknown".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scale() -> DepressionScale {
        DepressionScale {
            scale_id: 1,
            scale_name: "fixture".into(),
            scale_description: None,
            min_score: 0,
            max_score: 6,
            created_at: None,
            updated_at: None,
            questions: json!([
                {"id": 2, "options": [{"score":0},{"score":1},{"score":2},{"score":3}]},
                {"id": 7, "reverse_scored":true, "options": [{"score":3},{"score":2},{"score":1},{"score":0}]}
            ]),
            severity_ranges: json!([{"range":"0-2","label":"minimal"},{"range":"3-6","label":"elevated"}]),
        }
    }

    #[test]
    fn accepts_complete_arrays_and_objects_with_nonsequential_ids() {
        assert_eq!(compute_score(&scale(), &json!([1, 3])).unwrap(), 4);
        assert_eq!(compute_score(&scale(), &json!({"7":3,"2":1})).unwrap(), 4);
        // Submitted option scores already contain the reverse item's contribution.
        assert_eq!(compute_score(&scale(), &json!([0, 3])).unwrap(), 3);
    }

    #[test]
    fn rejects_incomplete_extra_unknown_and_invalid_answers() {
        for answer in [
            json!([]),
            json!([0]),
            json!([0, 0, 0]),
            json!({}),
            json!({"2":0,"99":0}),
            json!({"2":0,"7":0,"99":0}),
            json!({"02":0,"7":0}),
            json!([4, -1]),
            json!([-1, 1]),
            json!([0.5, 0]),
            json!(["0", 0]),
            json!([null, 0]),
            json!([true, 0]),
            json!([9223372036854775807_i64, 0]),
            json!("invalid"),
        ] {
            assert!(
                matches!(
                    compute_score(&scale(), &answer),
                    Err(AppError::Validation(_))
                ),
                "{answer}"
            );
        }
    }

    #[test]
    fn rejects_option_indexes_when_they_are_not_scores() {
        let mut s = scale();
        s.questions = json!([{"id":1,"options":[{"score":1},{"score":2}]}]);
        s.min_score = 1;
        s.max_score = 2;
        assert!(compute_score(&s, &json!([0])).is_err());
        assert_eq!(compute_score(&s, &json!([2])).unwrap(), 2);
    }

    #[test]
    fn rejects_broken_question_config_and_total_outside_range() {
        for questions in [
            json!([]),
            json!([{"id":1,"options":[]}]),
            json!([{"id":1,"options":[{"score":"0"}]}]),
            json!([{"id":1,"options":[{"score":0}]},{"id":1,"options":[{"score":0}]}]),
        ] {
            let mut s = scale();
            s.questions = questions;
            let answers = Value::Array(vec![json!(0); s.questions.as_array().unwrap().len()]);
            assert!(matches!(
                compute_score(&s, &answers),
                Err(AppError::Internal(_))
            ));
        }
        let mut s = scale();
        s.max_score = 2;
        assert!(matches!(
            compute_score(&s, &json!([3, 3])),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn legacy_and_explicit_ranges_match_all_boundaries() {
        let mut s = scale();
        for (score, expected) in [
            (0, "minimal"),
            (2, "minimal"),
            (3, "elevated"),
            (6, "elevated"),
            (7, "unknown"),
        ] {
            assert_eq!(determine_severity(&s, score).unwrap(), expected);
        }
        s.severity_ranges =
            json!([{"min":3,"max":6,"level":"elevated"},{"min":0,"max":2,"label":"minimal"}]);
        assert_eq!(determine_severity(&s, 3).unwrap(), "elevated");
    }

    #[test]
    fn rejects_malformed_overlapping_gapped_or_unbounded_ranges() {
        for ranges in [
            json!([]),
            json!([{"label":"all"}]),
            json!([{"min":0,"label":"all"}]),
            json!([{"range":"oops","label":"all"}]),
            json!([{"range":"0-6","label":""}]),
            json!([{"range":"3-0","label":"all"}]),
            json!([{"range":"0-3","label":"a"},{"range":"3-6","label":"b"}]),
            json!([{"range":"0-2","label":"a"},{"range":"4-6","label":"b"}]),
            json!([{"range":"0-5","label":"all"}]),
            json!([{"min":0,"max":9223372036854775807_i64,"level":"all"}]),
        ] {
            let mut s = scale();
            s.severity_ranges = ranges;
            assert!(matches!(
                determine_severity(&s, 1),
                Err(AppError::Internal(_))
            ));
        }
    }

    #[test]
    fn phq9_seed_range_regression_covers_every_score() {
        let mut s = scale();
        s.max_score = 27;
        s.severity_ranges = json!([
            {"range":"0-4","label":"minimal"},{"range":"5-9","label":"mild"},
            {"range":"10-14","label":"moderate"},{"range":"15-19","label":"moderately_severe"},
            {"range":"20-27","label":"severe"}
        ]);
        for score in 0..=27 {
            let expected = match score {
                0..=4 => "minimal",
                5..=9 => "mild",
                10..=14 => "moderate",
                15..=19 => "moderately_severe",
                _ => "severe",
            };
            assert_eq!(determine_severity(&s, score).unwrap(), expected);
        }
    }
}
