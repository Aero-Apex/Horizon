//! Controlling for Horizon (`docs/extras.md`): a port of Jaredlll08's
//! Controlling (MIT) and the Searchables library it searches with, for the
//! Key Binds page. The list is searched with `name`, `category:` and `key:`
//! terms, can show only unbound or conflicting keys, can be sorted, and a
//! list of the keys no binding uses can be shown in its place. `options_ui`
//! draws it; this holds the rules.
use std::cmp::Ordering;

/// What the search box matches an entry against (`SEARCHABLE_KEYBINDINGS`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The binding's translated name: the default component.
    pub name: String,
    /// The category's translated name.
    pub category: String,
    /// The bound key's display name, none while it is unbound.
    pub key: Option<String>,
}

/// A term of a search: a bare word for the name, or `component:value`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    Literal(String),
    Component(String, String),
}

/// `SLScanner` and `SLParser`: words split at spaces, `key:value` pairs, and
/// values quoted with `'`, `"` or a backtick when they hold spaces.
pub fn parse(search: &str) -> Vec<Term> {
    #[derive(Clone, Debug, PartialEq)]
    enum Tok {
        Space,
        Colon,
        Text(String),
        Quoted(String),
    }
    let chars: Vec<char> = search.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ' ' => {
                tokens.push(Tok::Space);
                i += 1;
            }
            ':' => {
                tokens.push(Tok::Colon);
                i += 1;
            }
            quote @ ('"' | '\'' | '`') => {
                let start = i + 1;
                i = start;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                tokens.push(Tok::Quoted(chars[start..i].iter().collect()));
                // The closing quote, when there is one.
                i = (i + 1).min(chars.len().max(i + 1));
            }
            _ => {
                let start = i;
                while i < chars.len() && chars[i] != ' ' && chars[i] != ':' {
                    i += 1;
                }
                tokens.push(Tok::Text(chars[start..i].iter().collect()));
            }
        }
    }
    let mut terms = Vec::new();
    let mut at = 0;
    // `literal()`: one operand, as the parser reads it.
    fn literal(tokens: &[Tok], at: &mut usize, terms: &mut Vec<Term>) {
        match tokens.get(*at) {
            Some(Tok::Colon) => {
                *at += 1;
                if let Some(Tok::Text(word)) = tokens.get(*at) {
                    *at += 1;
                    terms.push(Term::Literal(":".into()));
                    terms.push(Term::Literal(word.clone()));
                } else {
                    terms.push(Term::Literal(String::new()));
                }
            }
            Some(Tok::Text(word)) => {
                *at += 1;
                if matches!(tokens.get(*at), Some(Tok::Colon)) {
                    *at += 1;
                    // The value is the next operand, read the same way.
                    let mut value = Vec::new();
                    literal(tokens, at, &mut value);
                    match value.into_iter().next() {
                        Some(Term::Literal(v)) => {
                            terms.push(Term::Component(word.clone(), v));
                        }
                        // A nested `a:b:c` is not understood: as the library
                        // does, it adds nothing.
                        _ => {}
                    }
                } else {
                    terms.push(Term::Literal(word.clone()));
                }
            }
            Some(Tok::Quoted(text)) => {
                *at += 1;
                terms.push(Term::Literal(text.clone()));
            }
            _ => terms.push(Term::Literal(String::new())),
        }
    }
    if tokens.is_empty() {
        return terms;
    }
    literal(&tokens, &mut at, &mut terms);
    while matches!(tokens.get(at), Some(Tok::Space)) {
        at += 1;
        literal(&tokens, &mut at, &mut terms);
    }
    terms
}

fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// Whether `entry` passes every term (`SearchableType.filterEntries`): a bare
/// word looks in the name, a component in its own text; components the type
/// does not have let everything through.
pub fn matches(terms: &[Term], entry: &Entry) -> bool {
    terms.iter().all(|term| match term {
        Term::Literal(word) => contains_ignore_case(&entry.name, word),
        Term::Component(key, value) => match key.as_str() {
            "name" => contains_ignore_case(&entry.name, value),
            "category" => contains_ignore_case(&entry.category, value),
            "key" => entry
                .key
                .as_deref()
                .is_some_and(|k| contains_ignore_case(k, value)),
            _ => true,
        },
    })
}

/// `DisplayMode`: which keys the list holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    All,
    /// Keys without a key bound.
    Unbound,
    /// Keys another binding also uses.
    Conflicting,
}

/// `SortOrder`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Sort {
    #[default]
    None,
    Az,
    Za,
    KeyAz,
    KeyZa,
}
impl Sort {
    /// `options.sortNone` and the like.
    pub fn label(self) -> &'static str {
        match self {
            Sort::None => "None",
            Sort::Az => "A->Z",
            Sort::Za => "Z->A",
            Sort::KeyAz => "Key A->Z",
            Sort::KeyZa => "Key Z->A",
        }
    }
    /// `SortOrder.cycle`.
    pub fn cycle(self) -> Self {
        match self {
            Sort::None => Sort::Az,
            Sort::Az => Sort::Za,
            Sort::Za => Sort::KeyAz,
            Sort::KeyAz => Sort::KeyZa,
            Sort::KeyZa => Sort::None,
        }
    }
    /// How two keys' (name, key display) order; Java's `String.compareTo`
    /// compares UTF-16 units, which for these names is the code points'
    /// order.
    pub fn compare(self, a: (&str, &str), b: (&str, &str)) -> Ordering {
        let by = |x: &str, y: &str| x.encode_utf16().cmp(y.encode_utf16());
        match self {
            Sort::None => Ordering::Equal,
            Sort::Az => by(a.0, b.0),
            Sort::Za => by(b.0, a.0),
            Sort::KeyAz => by(a.1, b.1).then_with(|| by(a.0, b.0)),
            Sort::KeyZa => by(b.1, a.1).then_with(|| by(b.0, a.0)),
        }
    }
}

/// The `options.*` button labels of the page.
pub fn show_none_label(mode: Mode) -> &'static str {
    if mode == Mode::Unbound {
        "Show All"
    } else {
        "Show Unbound"
    }
}
pub fn show_conflicts_label(mode: Mode) -> &'static str {
    if mode == Mode::Conflicting {
        "Show All"
    } else {
        "Show Conflicts"
    }
}
/// What pressing Show Unbound does to the mode (`PRESS_NONE`).
pub fn press_unbound(mode: Mode) -> Mode {
    if mode == Mode::Unbound { Mode::All } else { Mode::Unbound }
}
/// What pressing Show Conflicts does (`PRESS_CONFLICTING`).
pub fn press_conflicting(mode: Mode) -> Mode {
    if mode == Mode::Conflicting {
        Mode::All
    } else {
        Mode::Conflicting
    }
}

/// Whether the list is shown as it is, with its category headers: nothing is
/// searched, filtered or sorted (`filterKeys`).
pub fn is_plain(search: &str, mode: Mode, sort: Sort) -> bool {
    search.is_empty() && mode == Mode::All && sort == Sort::None
}

/// The GLFW key code of a `key.*` name, as `InputConstants.Key.getValue`.
pub fn input_value(name: &str) -> Option<i32> {
    if let Some(n) = name.strip_prefix("key.mouse.") {
        return Some(match n {
            "left" => 0,
            "right" => 1,
            "middle" => 2,
            n => n.parse::<i32>().ok()? - 1,
        });
    }
    let k = name.strip_prefix("key.keyboard.")?;
    if k.len() == 1 {
        let c = k.chars().next()?;
        return Some(match c {
            'a'..='z' => 65 + (c as i32 - 'a' as i32),
            '0'..='9' => 48 + (c as i32 - '0' as i32),
            _ => return None,
        });
    }
    if let Some(n) = k.strip_prefix('f').and_then(|n| n.parse::<i32>().ok())
        && (1..=25).contains(&n)
    {
        return Some(289 + n);
    }
    if let Some(n) = k.strip_prefix("keypad.").and_then(|n| n.parse::<i32>().ok()) {
        return Some(320 + n);
    }
    Some(match k {
        "space" => 32,
        "apostrophe" => 39,
        "comma" => 44,
        "minus" => 45,
        "period" => 46,
        "slash" => 47,
        "semicolon" => 59,
        "equal" => 61,
        "left.bracket" => 91,
        "backslash" => 92,
        "right.bracket" => 93,
        "grave.accent" => 96,
        "escape" => 256,
        "enter" => 257,
        "tab" => 258,
        "backspace" => 259,
        "insert" => 260,
        "delete" => 261,
        "right" => 262,
        "left" => 263,
        "down" => 264,
        "up" => 265,
        "page.up" => 266,
        "page.down" => 267,
        "home" => 268,
        "end" => 269,
        "caps.lock" => 280,
        "scroll.lock" => 281,
        "num.lock" => 282,
        "print.screen" => 283,
        "pause" => 284,
        "keypad.period" => 330,
        "keypad.divide" => 331,
        "keypad.multiply" => 332,
        "keypad.subtract" => 333,
        "keypad.add" => 334,
        "keypad.enter" => 335,
        "keypad.equal" => 336,
        "left.shift" => 340,
        "left.control" => 341,
        "left.alt" => 342,
        "left.win" => 343,
        "right.shift" => 344,
        "right.control" => 345,
        "right.alt" => 346,
        "right.win" => 347,
        "menu" => 348,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, category: &str, key: Option<&str>) -> Entry {
        Entry {
            name: name.into(),
            category: category.into(),
            key: key.map(str::to_owned),
        }
    }

    #[test]
    fn searches_parse_like_the_library() {
        assert_eq!(parse(""), vec![]);
        assert_eq!(parse("jump"), vec![Term::Literal("jump".into())]);
        assert_eq!(
            parse("key:space jump"),
            vec![
                Term::Component("key".into(), "space".into()),
                Term::Literal("jump".into())
            ]
        );
        assert_eq!(
            parse("jump category:'Game Play'"),
            vec![
                Term::Literal("jump".into()),
                Term::Component("category".into(), "Game Play".into())
            ]
        );
        assert_eq!(parse("\"two words\""), vec![Term::Literal("two words".into())]);
        // An unclosed quote runs to the end.
        assert_eq!(parse("name:`abc"), vec![Term::Component("name".into(), "abc".into())]);
    }

    #[test]
    fn terms_filter_entries() {
        let jump = entry("Jump", "Movement", Some("Space"));
        let attack = entry("Attack/Destroy", "Gameplay", Some("Left Button"));
        let unbound = entry("Toggle Perspective", "Miscellaneous", None);
        let keep = |q: &str, e: &Entry| matches(&parse(q), e);
        assert!(keep("", &jump));
        assert!(keep("jum", &jump));
        assert!(!keep("jum", &attack));
        assert!(keep("category:game", &attack));
        assert!(!keep("category:game", &jump));
        assert!(keep("key:space", &jump));
        // A key component needs a key bound.
        assert!(!keep("key:", &unbound));
        assert!(keep("key:", &jump));
        assert!(keep("jump key:space", &jump));
        assert!(!keep("attack key:space", &jump));
        // An unknown component filters nothing.
        assert!(keep("colour:red", &jump));
    }

    #[test]
    fn the_buttons_toggle_their_mode() {
        assert_eq!(press_unbound(Mode::All), Mode::Unbound);
        assert_eq!(press_unbound(Mode::Unbound), Mode::All);
        assert_eq!(press_unbound(Mode::Conflicting), Mode::Unbound);
        assert_eq!(press_conflicting(Mode::Unbound), Mode::Conflicting);
        assert_eq!(press_conflicting(Mode::Conflicting), Mode::All);
        assert_eq!(show_none_label(Mode::Unbound), "Show All");
        assert_eq!(show_none_label(Mode::All), "Show Unbound");
        assert_eq!(show_conflicts_label(Mode::Conflicting), "Show All");
        assert_eq!(show_conflicts_label(Mode::Unbound), "Show Conflicts");
    }

    #[test]
    fn sorting_cycles_through_five_orders() {
        let mut sort = Sort::None;
        let mut labels = Vec::new();
        for _ in 0..5 {
            labels.push(sort.label());
            sort = sort.cycle();
        }
        assert_eq!(labels, vec!["None", "A->Z", "Z->A", "Key A->Z", "Key Z->A"]);
        assert_eq!(sort, Sort::None);
        let a = ("Attack", "Left Button");
        let b = ("Jump", "Space");
        assert_eq!(Sort::Az.compare(a, b), Ordering::Less);
        assert_eq!(Sort::Za.compare(a, b), Ordering::Greater);
        assert_eq!(Sort::KeyAz.compare(a, b), Ordering::Less);
        assert_eq!(Sort::KeyZa.compare(a, b), Ordering::Greater);
        assert_eq!(Sort::None.compare(a, b), Ordering::Equal);
        // The same key falls back to the name.
        assert_eq!(
            Sort::KeyAz.compare(("A", "W"), ("B", "W")),
            Ordering::Less
        );
        // Upper case sorts before lower case, as Java's compareTo does.
        assert_eq!(Sort::Az.compare(("Z", ""), ("a", "")), Ordering::Less);
    }

    #[test]
    fn plain_means_nothing_is_applied() {
        assert!(is_plain("", Mode::All, Sort::None));
        assert!(!is_plain("a", Mode::All, Sort::None));
        assert!(!is_plain("", Mode::Unbound, Sort::None));
        assert!(!is_plain("", Mode::All, Sort::Az));
    }

    #[test]
    fn key_values_are_glfws() {
        assert_eq!(input_value("key.keyboard.a"), Some(65));
        assert_eq!(input_value("key.keyboard.9"), Some(57));
        assert_eq!(input_value("key.keyboard.space"), Some(32));
        assert_eq!(input_value("key.keyboard.f5"), Some(294));
        assert_eq!(input_value("key.keyboard.keypad.3"), Some(323));
        assert_eq!(input_value("key.keyboard.left.shift"), Some(340));
        assert_eq!(input_value("key.mouse.left"), Some(0));
        assert_eq!(input_value("key.mouse.4"), Some(3));
        assert_eq!(input_value("key.keyboard.nope"), None);
    }
}
