//! Parsing Markdown minimaliste pour la génération Word :
//! paragraphes, mise en forme en ligne (`**gras**`, `*italique*`) et notes `[^x]`.

/// Fragment de texte en ligne avec sa mise en forme.
#[derive(Debug, Clone, PartialEq)]
pub struct Inline {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
}

/// Découpe un corps de texte en paragraphes (une ligne non vide = un paragraphe).
pub fn split_paragraphs(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect()
}

/// Retire un éventuel bloc de **métadonnées YAML** (frontmatter Obsidian) placé
/// en tête de fichier.
///
/// Le bloc est délimité par une ligne `---` ouvrant le tout premier contenu,
/// puis refermé par une autre ligne `---` (ou `...`). Il est **entièrement
/// supprimé** (métadonnées telles que `title:`, `tome:`, `acte:`, `periode:`,
/// `lieu:`, `pov:`, `statut:`…) afin de ne jamais apparaître dans le document
/// Word.
///
/// Si aucun bloc fermé n'est détecté, le texte est renvoyé **inchangé** : un
/// simple séparateur de scène `---` en tête de chapitre reste donc préservé.
pub fn strip_frontmatter(text: &str) -> String {
    let mut lines = text.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return text.to_string();
    };
    // Le frontmatter doit débuter dès la toute première ligne par `---`.
    if first.trim() != "---" {
        return text.to_string();
    }

    let mut body = String::new();
    let mut closed = false;
    for line in lines {
        if !closed {
            let marker = line.trim();
            if marker == "---" || marker == "..." {
                closed = true;
            }
            continue;
        }
        body.push_str(line);
    }

    if closed {
        body
    } else {
        // Frontmatter non refermé : on préserve le texte d'origine (prudence).
        text.to_string()
    }
}

/// Analyse la syntaxe en ligne `**gras**` et `*italique*` en fragments typés.
pub fn parse_inline(input: &str) -> Vec<Inline> {
    let chars: Vec<char> = input.chars().collect();
    let mut spans: Vec<Inline> = Vec::new();
    let mut buffer = String::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '*' {
            let is_bold = i + 1 < chars.len() && chars[i + 1] == '*';
            let marker_len = if is_bold { 2 } else { 1 };
            if let Some(close) = find_close(&chars, i + marker_len, marker_len) {
                if !buffer.is_empty() {
                    spans.push(Inline {
                        text: std::mem::take(&mut buffer),
                        bold: false,
                        italic: false,
                    });
                }
                let inner: String = chars[i + marker_len..close].iter().collect();
                spans.push(Inline {
                    text: inner,
                    bold: is_bold,
                    italic: !is_bold,
                });
                i = close + marker_len;
                continue;
            }
        }
        buffer.push(chars[i]);
        i += 1;
    }

    if !buffer.is_empty() {
        spans.push(Inline {
            text: buffer,
            bold: false,
            italic: false,
        });
    }
    spans
}

/// Trouve la position du marqueur de fermeture (`*` ou `**`) à partir de `from`.
fn find_close(chars: &[char], from: usize, marker_len: usize) -> Option<usize> {
    let mut j = from;
    while j + marker_len <= chars.len() {
        let matches =
            chars[j] == '*' && (marker_len == 1 || (j + 1 < chars.len() && chars[j + 1] == '*'));
        if matches {
            return Some(j);
        }
        j += 1;
    }
    None
}

/// Définition de note extraite d'un fichier source.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteDefinition {
    pub label: String,
    pub text: String,
}

/// Retire les lignes de définition `[^label]: …` et renvoie (corps, définitions).
pub fn extract_definitions(text: &str) -> (String, Vec<NoteDefinition>) {
    let mut body_lines: Vec<&str> = Vec::new();
    let mut definitions: Vec<NoteDefinition> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("[^") {
            if let Some(close) = rest.find("]:") {
                let label = rest[..close].to_string();
                let definition = rest[close + 2..].trim().to_string();
                definitions.push(NoteDefinition {
                    label,
                    text: definition,
                });
                continue;
            }
        }
        body_lines.push(line);
    }

    (body_lines.join("\n"), definitions)
}

/// Supprime **toutes** les balises de renvoi `[^…]` d'un texte (cas A, sans glossaire).
pub fn strip_refs(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' && i + 1 < chars.len() && chars[i + 1] == '^' {
            if let Some(rel) = chars[i..].iter().position(|c| *c == ']') {
                i += rel + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Fragment de corps : texte normal ou référence de note (numéro en exposant).
#[derive(Debug, Clone, PartialEq)]
pub enum BodyPiece {
    Text(String),
    NoteRef(u32),
}

/// Remplace les renvois `[^label]` par des numéros séquentiels **par chapitre**.
/// `numbers` associe chaque label à son numéro (créé au premier renvoi rencontré).
pub fn number_refs(text: &str, numbers: &mut Vec<(String, u32)>) -> Vec<BodyPiece> {
    let chars: Vec<char> = text.chars().collect();
    let mut pieces: Vec<BodyPiece> = Vec::new();
    let mut buffer = String::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '[' && i + 1 < chars.len() && chars[i + 1] == '^' {
            if let Some(rel) = chars[i..].iter().position(|c| *c == ']') {
                let label: String = chars[i + 2..i + rel].iter().collect();
                if !buffer.is_empty() {
                    pieces.push(BodyPiece::Text(std::mem::take(&mut buffer)));
                }
                let number = match numbers.iter().find(|(l, _)| *l == label) {
                    Some((_, n)) => *n,
                    None => {
                        let n = numbers.len() as u32 + 1;
                        numbers.push((label, n));
                        n
                    }
                };
                pieces.push(BodyPiece::NoteRef(number));
                i += rel + 1;
                continue;
            }
        }
        buffer.push(chars[i]);
        i += 1;
    }

    if !buffer.is_empty() {
        pieces.push(BodyPiece::Text(buffer));
    }
    pieces
}

/// Bloc Markdown de niveau paragraphe.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Paragraph(String),
    Heading1(String),
    Heading2(String),
    SceneBreak,
    Quote(String),
    ListItem { ordered: bool, text: String },
    Image { target: String },
}

/// Découpe un texte en blocs Markdown (titres, séparateurs, citations, listes, images).
pub fn parse_blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line == "---" || line == "***" || line == "___" {
            blocks.push(Block::SceneBreak);
            continue;
        }
        if line.len() > 1 && line.starts_with("## ") {
            blocks.push(Block::Heading2(line[3..].trim().to_string()));
            continue;
        }
        if line.len() > 1 && line.starts_with("# ") {
            blocks.push(Block::Heading1(line[2..].trim().to_string()));
            continue;
        }
        if line == "#" {
            blocks.push(Block::Heading1(String::new()));
            continue;
        }
        if line.len() > 1 && line.starts_with("> ") {
            blocks.push(Block::Quote(line[2..].trim().to_string()));
            continue;
        }
        if line == ">" {
            blocks.push(Block::Quote(String::new()));
            continue;
        }
        if line.len() > 1 && (line.starts_with("- ") || line.starts_with("* ")) {
            blocks.push(Block::ListItem {
                ordered: false,
                text: line[2..].trim().to_string(),
            });
            continue;
        }
        if let Some(text) = ordered_item(line) {
            blocks.push(Block::ListItem {
                ordered: true,
                text,
            });
            continue;
        }
        if let Some(target) = parse_image(line) {
            blocks.push(Block::Image { target });
            continue;
        }
        blocks.push(Block::Paragraph(line.to_string()));
    }

    blocks
}

/// Détecte un item de liste ordonnée (`1. texte`) et renvoie son contenu.
fn ordered_item(line: &str) -> Option<String> {
    let digits: String = line.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = &line[digits.len()..];
    rest.strip_prefix(". ").map(|text| text.trim().to_string())
}

/// Détecte une image isolée et renvoie sa cible.
///
/// Deux syntaxes sont reconnues :
/// - **Markdown** : `![alt](cible)` ;
/// - **Obsidian** : `![[fichier.png]]` ou `![[fichier.png|taille/alt]]`.
fn parse_image(line: &str) -> Option<String> {
    // Obsidian (wiki-embed) : `![[...]]` — la cible précède un éventuel `|`.
    if let Some(rest) = line.strip_prefix("![[") {
        let end = rest.find("]]")?;
        let inner = &rest[..end];
        let target = inner.split('|').next().unwrap_or(inner).trim();
        if target.is_empty() {
            return None;
        }
        return Some(target.to_string());
    }

    // Markdown : `![alt](cible)`.
    let rest = line.strip_prefix("![")?;
    let bracket = rest.find("](")?;
    let after = &rest[bracket + 2..];
    let paren = after.find(')')?;
    let target = after[..paren].trim();
    if target.is_empty() {
        return None;
    }
    Some(target.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_frontmatter_removes_obsidian_metadata() {
        let source = "---\ntitle: Hildegarde\ntome: 1\npov: Ada\nstatut: brouillon\n---\n# Chapitre\n\nCorps du texte.\n";
        let stripped = strip_frontmatter(source);
        assert!(!stripped.contains("title:"), "métadonnée supprimée");
        assert!(!stripped.contains("pov:"), "métadonnée supprimée");
        assert!(stripped.contains("# Chapitre"), "titre conservé");
        assert!(stripped.contains("Corps du texte."), "corps conservé");
    }

    #[test]
    fn strip_frontmatter_supports_dots_terminator() {
        let source = "---\nkey: value\n...\nPremier paragraphe.\n";
        let stripped = strip_frontmatter(source);
        assert!(!stripped.contains("key: value"));
        assert!(stripped.contains("Premier paragraphe."));
    }

    #[test]
    fn strip_frontmatter_preserves_scene_break_and_plain_text() {
        // `---` isolé (séparateur de scène) sans bloc fermé : préservé.
        let scene = "Texte d'ouverture.\n\n---\n\nSuite.\n";
        assert_eq!(strip_frontmatter(scene), scene);

        // Texte sans frontmatter : inchangé.
        let plain = "# Titre\n\nUn paragraphe.\n";
        assert_eq!(strip_frontmatter(plain), plain);
    }

    #[test]
    fn parse_blocks_recognises_markdown_and_obsidian_images() {
        let blocks = parse_blocks("![alt](image.png)\n\n![[planche.png]]\n\n![[carte.png|300]]\n");
        let targets: Vec<String> = blocks
            .iter()
            .filter_map(|block| match block {
                Block::Image { target } => Some(target.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            targets,
            vec![
                "image.png".to_string(),
                "planche.png".to_string(),
                "carte.png".to_string()
            ],
            "images Markdown et embeds Obsidian (`![[]]`) reconnus"
        );
    }

    #[test]
    fn split_paragraphs_trims_and_drops_blank_lines() {
        let paragraphs = split_paragraphs("  Premier  \n\n\tDeuxième\t\n   \nTroisième");
        assert_eq!(
            paragraphs,
            vec![
                "Premier".to_string(),
                "Deuxième".to_string(),
                "Troisième".to_string()
            ]
        );
        // Aucun contenu → aucune ligne.
        assert!(split_paragraphs("\n\n   \n").is_empty());
    }

    #[test]
    fn parse_inline_recognises_bold_italic_and_keeps_unclosed() {
        let spans = parse_inline("a **gras** b *ital* c");
        assert_eq!(
            spans,
            vec![
                Inline {
                    text: "a ".to_string(),
                    bold: false,
                    italic: false
                },
                Inline {
                    text: "gras".to_string(),
                    bold: true,
                    italic: false
                },
                Inline {
                    text: " b ".to_string(),
                    bold: false,
                    italic: false
                },
                Inline {
                    text: "ital".to_string(),
                    bold: false,
                    italic: true
                },
                Inline {
                    text: " c".to_string(),
                    bold: false,
                    italic: false
                },
            ]
        );
        // Marqueur non refermé → texte brut conservé tel quel.
        assert_eq!(
            parse_inline("**non refermé"),
            vec![Inline {
                text: "**non refermé".to_string(),
                bold: false,
                italic: false
            }]
        );
    }

    #[test]
    fn extract_definitions_separates_notes_from_body() {
        let (body, defs) = extract_definitions("Para.[^1]\n\n[^1]: Note un.\n[^2]: Note deux.\n");
        assert!(!body.contains("[^1]:"), "définitions retirées du corps");
        assert!(body.contains("Para.[^1]"), "corps conservé");
        assert_eq!(
            defs,
            vec![
                NoteDefinition {
                    label: "1".to_string(),
                    text: "Note un.".to_string()
                },
                NoteDefinition {
                    label: "2".to_string(),
                    text: "Note deux.".to_string()
                },
            ]
        );
    }

    #[test]
    fn strip_refs_removes_all_markers() {
        assert_eq!(strip_refs("Texte[^a] et[^b] suite."), "Texte et suite.");
        // Un `[` non suivi de `^` est conservé tel quel.
        assert_eq!(strip_refs("crochet [normal] ici"), "crochet [normal] ici");
    }

    #[test]
    fn number_refs_assigns_sequential_and_reuses_existing() {
        let mut numbers: Vec<(String, u32)> = Vec::new();
        let pieces = number_refs("a[^x] b[^y] c[^x]", &mut numbers);
        assert_eq!(
            pieces,
            vec![
                BodyPiece::Text("a".to_string()),
                BodyPiece::NoteRef(1),
                BodyPiece::Text(" b".to_string()),
                BodyPiece::NoteRef(2),
                BodyPiece::Text(" c".to_string()),
                // `x` a déjà reçu le numéro 1 : il est réutilisé.
                BodyPiece::NoteRef(1),
            ]
        );
        assert_eq!(numbers, vec![("x".to_string(), 1), ("y".to_string(), 2)]);
    }

    #[test]
    fn parse_blocks_covers_every_block_type() {
        let source = "# Titre 1\n\n## Titre 2\n\n> Citation\n\n- puce\n\n1. num\n\n---\n\n![alt](img.png)\n\nParagraphe simple.\n";
        let blocks = parse_blocks(source);
        assert!(blocks.contains(&Block::Heading1("Titre 1".to_string())));
        assert!(blocks.contains(&Block::Heading2("Titre 2".to_string())));
        assert!(blocks.contains(&Block::Quote("Citation".to_string())));
        assert!(blocks.contains(&Block::ListItem {
            ordered: false,
            text: "puce".to_string()
        }));
        assert!(blocks.contains(&Block::ListItem {
            ordered: true,
            text: "num".to_string()
        }));
        assert!(blocks.contains(&Block::SceneBreak));
        assert!(blocks.contains(&Block::Image {
            target: "img.png".to_string()
        }));
        assert!(blocks.contains(&Block::Paragraph("Paragraphe simple.".to_string())));
    }

    #[test]
    fn parse_blocks_handles_bare_markers() {
        // `#` et `>` seuls produisent des blocs vides (titres/citations sans texte).
        assert_eq!(
            parse_blocks("#\n\n>\n"),
            vec![Block::Heading1(String::new()), Block::Quote(String::new())]
        );
    }
}
