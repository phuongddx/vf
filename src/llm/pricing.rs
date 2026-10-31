//! USD per 1M tokens. Source: docs.z.ai pricing page, fetched 2026-10-31.
//! Unknown models report tokens only (estimate_cost -> None).

use super::Usage;

pub fn estimate_cost(model: &str, usage: &Usage) -> Option<f64> {
    let (in_per_m, out_per_m): (f64, f64) = match model {
        "glm-5.2" => (1.4, 4.4),
        _ => return None,
    };
    Some(
        usage.input_tokens as f64 / 1_000_000.0 * in_per_m
            + usage.output_tokens as f64 / 1_000_000.0 * out_per_m,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Usage;

    #[test]
    fn glm_52_priced_from_docs() {
        let u = Usage { input_tokens: 1_000_000, output_tokens: 1_000_000 };
        let cost = estimate_cost("glm-5.2", &u).unwrap();
        assert!((cost - 5.8).abs() < 1e-9); // $1.4 in + $4.4 out per 1M
    }

    #[test]
    fn unknown_model_returns_none() {
        let u = Usage { input_tokens: 10, output_tokens: 10 };
        assert!(estimate_cost("mystery-9", &u).is_none());
    }
}
