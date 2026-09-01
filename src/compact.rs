pub fn estimate_tokens(s: &str) -> usize {
    let words = s.split_whitespace().count();
    (words * 4 / 3).max(1)
}

pub fn render_ai_cards(query: &str, budget: usize, used: usize, cards: &[AiCard]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# mneme recall\nquery: {query}\ntokens: {used}/{budget}\ncards: {}\n", cards.len()));
    for c in cards {
        out.push('\n');
        out.push_str(&format!("## {}  act={:.2}  {}\n", c.title, c.activation, c.path));
        if !c.summary.is_empty() {
            out.push_str(&c.summary);
            out.push('\n');
        }
        if !c.links.is_empty() {
            out.push_str("links: ");
            out.push_str(&c.links.join(", "));
            out.push('\n');
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct AiCard {
    pub title: String,
    pub path: String,
    pub summary: String,
    pub activation: f64,
    pub links: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_estimate_grows_with_words() {
        assert!(estimate_tokens("one two three four") >= 4);
    }
}
