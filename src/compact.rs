pub fn estimate_tokens(s: &str) -> usize {
    let words = s.split_whitespace().count();
    (words * 4 / 3).max(1)
}

pub fn render_ai_cards(query: &str, budget: usize, used: usize, cards: &[AiCard]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# mneme recall\nquery: {query}\ntokens: {used}/{budget}\ncards: {}\n",
        cards.len()
    ));
    for c in cards {
        out.push('\n');
        out.push_str(&format!(
            "## {}  act={:.2}  {}  lang={}\n",
            c.title, c.activation, c.path, c.lang
        ));
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
    pub lang: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_estimate_grows_with_words() {
        assert!(estimate_tokens("one two three four") >= 4);
    }

    #[test]
    fn ai_card_includes_lang() {
        let cards = [AiCard {
            title: "Quote".into(),
            path: "02-memory/Quote.md".into(),
            summary: "Não traduza.".into(),
            activation: 0.5,
            links: vec![],
            lang: "pt".into(),
        }];
        let out = render_ai_cards("cue", 100, 40, &cards);
        assert!(out.contains("lang=pt"));
        assert!(out.contains("Não traduza"));
    }
}
