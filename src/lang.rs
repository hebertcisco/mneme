//! Language tags for notes. Vault default is English; source text is not translated.

/// Normalize a language tag to ISO 639-1 when possible (`pt-BR` → `pt`).
pub fn normalize(tag: &str) -> String {
    let t = tag.trim().to_ascii_lowercase().replace('_', "-");
    if t.is_empty() {
        return String::new();
    }
    let primary = t.split('-').next().unwrap_or(&t);
    match primary {
        "english" => "en".into(),
        "portuguese" | "portugues" => "pt".into(),
        "spanish" | "espanol" | "español" => "es".into(),
        "french" | "francais" | "français" => "fr".into(),
        "german" | "deutsch" => "de".into(),
        "italian" | "italiano" => "it".into(),
        "chinese" => "zh".into(),
        "japanese" => "ja".into(),
        "korean" => "ko".into(),
        "arabic" => "ar".into(),
        "russian" => "ru".into(),
        other if other.len() == 2 && other.chars().all(|c| c.is_ascii_alphabetic()) => {
            other.to_string()
        }
        other if other.len() == 3 && other.chars().all(|c| c.is_ascii_alphabetic()) => {
            other.to_string()
        }
        _ => t,
    }
}

/// Use an explicit tag if present, otherwise `fallback` (usually vault `language`).
pub fn resolve(explicit: &str, fallback: &str) -> String {
    let n = normalize(explicit);
    if n.is_empty() {
        let fb = normalize(fallback);
        if fb.is_empty() {
            "en".into()
        } else {
            fb
        }
    } else {
        n
    }
}

/// Guess ISO 639-1 from text. Low confidence → `fallback`. Does not translate.
pub fn detect(text: &str, fallback: &str) -> String {
    let fallback = resolve("", fallback);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return fallback;
    }

    if let Some(script) = detect_script(trimmed) {
        return script;
    }

    let scores = latin_scores(trimmed);
    let mut ranked: Vec<(&str, i32)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));

    let Some((best, best_n)) = ranked.first().copied() else {
        return fallback;
    };
    let second = ranked.get(1).map(|s| s.1).unwrap_or(0);
    if best_n >= 3 && best_n > second {
        best.to_string()
    } else {
        fallback
    }
}

fn detect_script(text: &str) -> Option<String> {
    let mut cyrillic = 0u32;
    let mut arabic = 0u32;
    let mut hangul = 0u32;
    let mut kana = 0u32;
    let mut cjk = 0u32;
    let mut latin = 0u32;
    for c in text.chars() {
        if c.is_ascii_alphabetic() {
            latin += 1;
        } else if ('\u{0400}'..='\u{052F}').contains(&c) {
            cyrillic += 1;
        } else if ('\u{0600}'..='\u{06FF}').contains(&c) || ('\u{0750}'..='\u{077F}').contains(&c) {
            arabic += 1;
        } else if ('\u{AC00}'..='\u{D7AF}').contains(&c) {
            hangul += 1;
        } else if ('\u{3040}'..='\u{30FF}').contains(&c) {
            kana += 1;
        } else if ('\u{4E00}'..='\u{9FFF}').contains(&c) {
            cjk += 1;
        }
    }
    if cyrillic >= 8 && cyrillic > latin {
        return Some("ru".into());
    }
    if arabic >= 8 && arabic > latin {
        return Some("ar".into());
    }
    if hangul >= 8 {
        return Some("ko".into());
    }
    if kana >= 6 {
        return Some("ja".into());
    }
    if cjk >= 12 && kana == 0 && hangul == 0 {
        return Some("zh".into());
    }
    None
}

fn latin_scores(text: &str) -> Vec<(&'static str, i32)> {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphabetic() && c != '\'')
        .filter(|w| w.len() > 1)
        .collect();

    let mut en = 0i32;
    let mut pt = 0i32;
    let mut es = 0i32;
    let mut fr = 0i32;
    let mut de = 0i32;
    let mut it = 0i32;

    if lower.contains('ã') || lower.contains('õ') || lower.contains("ção") || lower.contains("ões")
    {
        pt += 4;
    }
    if lower.contains('ñ') || lower.contains("ción") {
        es += 4;
    }
    if lower.contains('ç') && !lower.contains("ción") {
        pt += 2;
        fr += 1;
    }
    if lower.contains('ß') || lower.contains("sch") {
        de += 2;
    }
    if lower.contains("ção") || lower.contains("nh") || lower.contains("lh") {
        pt += 2;
    }

    for w in &words {
        if EN.contains(w) {
            en += 1;
        }
        if PT.contains(w) {
            pt += 1;
        }
        if ES.contains(w) {
            es += 1;
        }
        if FR.contains(w) {
            fr += 1;
        }
        if DE.contains(w) {
            de += 1;
        }
        if IT.contains(w) {
            it += 1;
        }
    }

    vec![
        ("en", en),
        ("pt", pt),
        ("es", es),
        ("fr", fr),
        ("de", de),
        ("it", it),
    ]
}

const EN: &[&str] = &[
    "the", "and", "that", "this", "with", "from", "have", "were", "been", "will", "would", "there",
    "their", "about", "which", "when", "what", "into", "also", "note", "vault", "index", "memory",
];
const PT: &[&str] = &[
    "não", "nao", "você", "voce", "vocês", "voces", "então", "entao", "também", "tambem", "pelo",
    "pela", "pelos", "pelas", "uma", "isso", "isto", "aqui", "muito", "mais", "porque", "quando",
    "onde", "nós", "nos", "eles", "elas", "meu", "minha", "está", "esta", "estão", "estao", "são",
    "sao", "foi", "pra", "dos", "das", "num", "numa", "ninguém", "ninguem", "alguém", "alguem",
    "ainda", "hoje", "ontem", "amanhã", "amanha", "depois", "antes", "agora", "para", "com", "que",
    "uma", "este", "essa", "esse", "desta", "deste", "nesta", "neste",
];
const ES: &[&str] = &[
    "usted", "ustedes", "entonces", "también", "tambien", "una", "unos", "unas", "eso", "esto",
    "aquí", "aqui", "muy", "más", "mas", "pero", "porque", "cuando", "donde", "nosotros", "ellos",
    "ellas", "está", "están", "estan", "los", "las", "del", "fue", "hay", "como", "para", "con",
    "este", "esta", "ese", "esa",
];
const FR: &[&str] = &[
    "les", "des", "une", "est", "pas", "que", "pour", "dans", "avec", "plus", "cette", "tout",
    "fait", "être", "etre", "aussi", "mais", "nous", "vous", "sont", "été", "ete",
];
const DE: &[&str] = &[
    "und", "der", "die", "das", "nicht", "ein", "eine", "ist", "von", "mit", "den", "auf", "für",
    "fur", "sich", "auch", "als", "werden", "wird", "haben",
];
const IT: &[&str] = &[
    "che", "non", "una", "per", "con", "del", "della", "sono", "questo", "come", "più", "piu",
    "anche", "delle", "nel", "alla", "gli",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_variants() {
        assert_eq!(normalize("pt-BR"), "pt");
        assert_eq!(normalize("EN"), "en");
        assert_eq!(normalize("portuguese"), "pt");
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn detect_portuguese_body() {
        let t = "Não traduza este parágrafo; grave no idioma original com metadata.";
        assert_eq!(detect(t, "en"), "pt");
    }

    #[test]
    fn detect_english_body() {
        let t = "The index under the vault can be rebuilt from markdown notes with recall.";
        assert_eq!(detect(t, "en"), "en");
    }

    #[test]
    fn detect_falls_back_when_empty() {
        assert_eq!(detect("   ", "en"), "en");
        assert_eq!(detect("", "pt"), "pt");
    }

    #[test]
    fn explicit_tag_wins() {
        assert_eq!(resolve("es", "en"), "es");
        assert_eq!(resolve("  ", "en"), "en");
    }
}
