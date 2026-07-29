use std::collections::HashMap;
use std::rc::Rc;

// ─── String Interner ─────────────────────────────────────────────
//
// A string interner maps strings to compact 4-byte integer handles
// called `Symbol`s. Two strings that are equal always get the same
// Symbol, so you can compare them with `==` on integers instead of
// byte-by-byte string comparison.
//
// Why this matters for a browser engine:
//
//   - Tag names like "div", "p", "span" appear thousands of times in
//     a typical page. Without interning, each occurrence is a separate
//     String allocation on the heap. With interning, they all resolve
//     to the same Symbol(3) or whatever index it gets.
//
//   - CSS property names ("color", "margin", "display") are compared
//     constantly during style resolution. Integer comparison is a
//     single CPU instruction vs. looping over bytes.
//
//   - The interner pre-seeds common names at startup, so the most
//     frequently used symbols have deterministic, known values you
//     can match on without any lookup.
//
// Usage:
//
//   let mut interner = Interner::new();  // pre-seeds common symbols
//   let sym = interner.intern("div");    // returns Symbol(index)
//   assert_eq!(sym, interner.intern("div"));  // same string → same symbol
//   assert_eq!(interner.resolve(sym), "div"); // symbol → string
//
//   // Case-insensitive interning (for HTML tags)
//   let sym2 = interner.intern_lower("DIV");
//   assert_eq!(sym2, interner.intern("div")); // "DIV" lowered → same as "div"

// ─── Symbol ──────────────────────────────────────────────────────

/// A 32-bit handle pointing to an interned string.
///
/// Symbols are cheap to copy (4 bytes), compare (single `==`), hash,
/// and sort. They are only meaningful within the `Interner` that
/// created them — never mix symbols from different interners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(pub u32);

// ─── Pre-seeded Symbol Constants ─────────────────────────────────
//
// These are the Symbol values assigned to common strings during
// Interner::new(). They're deterministic because pre-seeding
// happens in a fixed order.
//
// Having named constants lets you write fast pattern matches:
//
//   match tag_symbol {
//       SYM_DIV  => { /* block element */ }
//       SYM_SPAN => { /* inline element */ }
//       _ => { /* other */ }
//   }

// ── HTML Tags ────────────────────────────────────────────────────

pub const SYM_HTML:   Symbol = Symbol(0);
pub const SYM_HEAD:   Symbol = Symbol(1);
pub const SYM_BODY:   Symbol = Symbol(2);
pub const SYM_DIV:    Symbol = Symbol(3);
pub const SYM_P:      Symbol = Symbol(4);
pub const SYM_SPAN:   Symbol = Symbol(5);
pub const SYM_A:      Symbol = Symbol(6);
pub const SYM_H1:     Symbol = Symbol(7);
pub const SYM_H2:     Symbol = Symbol(8);
pub const SYM_H3:     Symbol = Symbol(9);
pub const SYM_H4:     Symbol = Symbol(10);
pub const SYM_H5:     Symbol = Symbol(11);
pub const SYM_H6:     Symbol = Symbol(12);
pub const SYM_UL:     Symbol = Symbol(13);
pub const SYM_OL:     Symbol = Symbol(14);
pub const SYM_LI:     Symbol = Symbol(15);
pub const SYM_TABLE:  Symbol = Symbol(16);
pub const SYM_TR:     Symbol = Symbol(17);
pub const SYM_TD:     Symbol = Symbol(18);
pub const SYM_TH:     Symbol = Symbol(19);
pub const SYM_IMG:    Symbol = Symbol(20);
pub const SYM_BR:     Symbol = Symbol(21);
pub const SYM_HR:     Symbol = Symbol(22);
pub const SYM_STYLE:  Symbol = Symbol(23);
pub const SYM_LINK:   Symbol = Symbol(24);
pub const SYM_SCRIPT: Symbol = Symbol(25);
pub const SYM_TITLE:  Symbol = Symbol(26);
pub const SYM_META:   Symbol = Symbol(27);
pub const SYM_INPUT:  Symbol = Symbol(28);
pub const SYM_FORM:   Symbol = Symbol(29);
pub const SYM_BUTTON: Symbol = Symbol(30);
pub const SYM_STRONG: Symbol = Symbol(31);
pub const SYM_EM:     Symbol = Symbol(32);
pub const SYM_SECTION: Symbol = Symbol(33);
pub const SYM_ARTICLE: Symbol = Symbol(34);
pub const SYM_NAV:    Symbol = Symbol(35);
pub const SYM_HEADER: Symbol = Symbol(36);
pub const SYM_FOOTER: Symbol = Symbol(37);
pub const SYM_MAIN:   Symbol = Symbol(38);

// ── HTML Attributes ──────────────────────────────────────────────

pub const SYM_CLASS:  Symbol = Symbol(39);
pub const SYM_ID:     Symbol = Symbol(40);
pub const SYM_HREF:   Symbol = Symbol(41);
pub const SYM_SRC:    Symbol = Symbol(42);
pub const SYM_REL:    Symbol = Symbol(43);
pub const SYM_TYPE:   Symbol = Symbol(44);
pub const SYM_ALT:    Symbol = Symbol(45);

// ── CSS Properties ───────────────────────────────────────────────

pub const SYM_COLOR:            Symbol = Symbol(46);
pub const SYM_BACKGROUND_COLOR: Symbol = Symbol(47);
pub const SYM_DISPLAY:          Symbol = Symbol(48);
pub const SYM_WIDTH:            Symbol = Symbol(49);
pub const SYM_HEIGHT:           Symbol = Symbol(50);
pub const SYM_MARGIN:           Symbol = Symbol(51);
pub const SYM_MARGIN_TOP:       Symbol = Symbol(52);
pub const SYM_MARGIN_RIGHT:     Symbol = Symbol(53);
pub const SYM_MARGIN_BOTTOM:    Symbol = Symbol(54);
pub const SYM_MARGIN_LEFT:      Symbol = Symbol(55);
pub const SYM_PADDING:          Symbol = Symbol(56);
pub const SYM_PADDING_TOP:      Symbol = Symbol(57);
pub const SYM_PADDING_RIGHT:    Symbol = Symbol(58);
pub const SYM_PADDING_BOTTOM:   Symbol = Symbol(59);
pub const SYM_PADDING_LEFT:     Symbol = Symbol(60);
pub const SYM_FONT_SIZE:        Symbol = Symbol(61);
pub const SYM_FONT_WEIGHT:      Symbol = Symbol(62);
pub const SYM_TEXT_ALIGN:        Symbol = Symbol(63);
pub const SYM_LINE_HEIGHT:      Symbol = Symbol(64);
pub const SYM_BORDER_WIDTH:     Symbol = Symbol(65);
pub const SYM_POSITION:         Symbol = Symbol(66);

/// The list of pre-seeded strings, in the exact order they must be
/// inserted to match the `SYM_*` constants above.
const PRE_SEEDED: &[&str] = &[
    // HTML tags (0..=38)
    "html", "head", "body", "div", "p", "span", "a",
    "h1", "h2", "h3", "h4", "h5", "h6",
    "ul", "ol", "li",
    "table", "tr", "td", "th",
    "img", "br", "hr",
    "style", "link", "script", "title", "meta",
    "input", "form", "button",
    "strong", "em",
    "section", "article", "nav", "header", "footer", "main",
    // HTML attributes (39..=45)
    "class", "id", "href", "src", "rel", "type", "alt",
    // CSS properties (46..=66)
    "color", "background-color", "display",
    "width", "height",
    "margin", "margin-top", "margin-right", "margin-bottom", "margin-left",
    "padding", "padding-top", "padding-right", "padding-bottom", "padding-left",
    "font-size", "font-weight", "text-align", "line-height",
    "border-width", "position",
];

// ─── Interner ────────────────────────────────────────────────────

/// A bidirectional string interner.
///
/// - `intern(name)` → gives back a `Symbol` (creating a new one if
///   the string hasn't been seen before).
/// - `resolve(sym)` → gives back the `&str` the symbol points to.
///
/// The interner pre-seeds common HTML tags and CSS property names
/// so they always have known, deterministic `Symbol` values.
pub struct Interner {
    /// Forward map: string → Symbol index
    map: HashMap<Rc<str>, Symbol>,
    /// Reverse table: Symbol index → owned string
    strings: Vec<Rc<str>>,
}

impl Interner {
    /// Create a new interner pre-seeded with common HTML tags,
    /// attribute names, and CSS property names.
    pub fn new() -> Self {
        let mut interner = Interner {
            map: HashMap::with_capacity(PRE_SEEDED.len() * 2),
            strings: Vec::with_capacity(PRE_SEEDED.len() * 2),
        };

        for &name in PRE_SEEDED {
            interner.intern(name);
        }

        interner
    }

    /// Intern a string, returning its `Symbol`.
    ///
    /// If the string has been interned before, the existing Symbol
    /// is returned. Otherwise a new Symbol is allocated.
    ///
    /// This is case-sensitive: `"Div"` and `"div"` get different symbols.
    /// Use `intern_lower()` when you need case-insensitive interning.
    pub fn intern(&mut self, name: &str) -> Symbol {
        if let Some(&sym) = self.map.get(name) {
            return sym;
        }

        let sym = Symbol(self.strings.len() as u32);
        let rc: Rc<str> = Rc::from(name);
        self.strings.push(Rc::clone(&rc));
        self.map.insert(rc, sym);
        sym
    }

    /// Intern a string after lowercasing it (for case-insensitive lookups).
    ///
    /// HTML tag names are case-insensitive: `<DIV>` and `<div>` should
    /// resolve to the same Symbol. This method lowercases the input
    /// before interning.
    ///
    /// Note: only ASCII lowercasing is performed (sufficient for HTML/CSS).
    pub fn intern_lower(&mut self, name: &str) -> Symbol {
        let lowered = name.to_ascii_lowercase();
        self.intern(&lowered)
    }

    /// Resolve a Symbol back to its string representation.
    ///
    /// # Panics
    ///
    /// Panics if the Symbol was not created by this interner (index out of bounds).
    pub fn resolve(&self, sym: Symbol) -> &str {
        &self.strings[sym.0 as usize]
    }

    /// Try to resolve a Symbol, returning `None` if the index is out of bounds.
    pub fn try_resolve(&self, sym: Symbol) -> Option<&str> {
        self.strings.get(sym.0 as usize).map(|s| &**s)
    }

    /// Look up a string without interning it. Returns `Some(Symbol)`
    /// if already interned, `None` otherwise.
    pub fn lookup(&self, name: &str) -> Option<Symbol> {
        self.map.get(name).copied()
    }

    /// How many unique strings are currently interned.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Is the interner empty? (Always false after `new()` because of pre-seeding.)
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Basic Interning ──────────────────────────────────────────

    #[test]
    fn test_intern_same_string_same_symbol() {
        let mut interner = Interner::new();
        let a = interner.intern("custom-tag");
        let b = interner.intern("custom-tag");
        assert_eq!(a, b);
    }

    #[test]
    fn test_intern_different_strings_different_symbols() {
        let mut interner = Interner::new();
        let a = interner.intern("alpha");
        let b = interner.intern("beta");
        assert_ne!(a, b);
    }

    #[test]
    fn test_resolve_round_trip() {
        let mut interner = Interner::new();
        let sym = interner.intern("round-trip");
        assert_eq!(interner.resolve(sym), "round-trip");
    }

    #[test]
    fn test_try_resolve_valid() {
        let mut interner = Interner::new();
        let sym = interner.intern("valid");
        assert_eq!(interner.try_resolve(sym), Some("valid"));
    }

    #[test]
    fn test_try_resolve_invalid() {
        let interner = Interner::new();
        assert_eq!(interner.try_resolve(Symbol(99999)), None);
    }

    // ── Case Sensitivity ─────────────────────────────────────────

    #[test]
    fn test_intern_is_case_sensitive() {
        let mut interner = Interner::new();
        let lower = interner.intern("div");  // pre-seeded
        let upper = interner.intern("DIV");
        assert_ne!(lower, upper, "intern() should be case-sensitive");
    }

    #[test]
    fn test_intern_lower_normalizes_case() {
        let mut interner = Interner::new();
        let a = interner.intern_lower("DIV");
        let b = interner.intern_lower("Div");
        let c = interner.intern("div");
        assert_eq!(a, b);
        assert_eq!(a, c, "intern_lower should match pre-seeded lowercase");
    }

    #[test]
    fn test_intern_lower_mixed_case() {
        let mut interner = Interner::new();
        let sym = interner.intern_lower("Background-Color");
        assert_eq!(interner.resolve(sym), "background-color");
    }

    // ── Pre-seeded Constants ─────────────────────────────────────

    #[test]
    fn test_pre_seeded_html_tags() {
        let interner = Interner::new();

        assert_eq!(interner.resolve(SYM_HTML), "html");
        assert_eq!(interner.resolve(SYM_HEAD), "head");
        assert_eq!(interner.resolve(SYM_BODY), "body");
        assert_eq!(interner.resolve(SYM_DIV), "div");
        assert_eq!(interner.resolve(SYM_P), "p");
        assert_eq!(interner.resolve(SYM_SPAN), "span");
        assert_eq!(interner.resolve(SYM_A), "a");
        assert_eq!(interner.resolve(SYM_H1), "h1");
        assert_eq!(interner.resolve(SYM_STYLE), "style");
        assert_eq!(interner.resolve(SYM_LINK), "link");
        assert_eq!(interner.resolve(SYM_STRONG), "strong");
        assert_eq!(interner.resolve(SYM_EM), "em");
    }

    #[test]
    fn test_pre_seeded_html_attributes() {
        let interner = Interner::new();

        assert_eq!(interner.resolve(SYM_CLASS), "class");
        assert_eq!(interner.resolve(SYM_ID), "id");
        assert_eq!(interner.resolve(SYM_HREF), "href");
        assert_eq!(interner.resolve(SYM_SRC), "src");
        assert_eq!(interner.resolve(SYM_REL), "rel");
        assert_eq!(interner.resolve(SYM_TYPE), "type");
        assert_eq!(interner.resolve(SYM_ALT), "alt");
    }

    #[test]
    fn test_pre_seeded_css_properties() {
        let interner = Interner::new();

        assert_eq!(interner.resolve(SYM_COLOR), "color");
        assert_eq!(interner.resolve(SYM_BACKGROUND_COLOR), "background-color");
        assert_eq!(interner.resolve(SYM_DISPLAY), "display");
        assert_eq!(interner.resolve(SYM_WIDTH), "width");
        assert_eq!(interner.resolve(SYM_HEIGHT), "height");
        assert_eq!(interner.resolve(SYM_MARGIN), "margin");
        assert_eq!(interner.resolve(SYM_MARGIN_TOP), "margin-top");
        assert_eq!(interner.resolve(SYM_PADDING), "padding");
        assert_eq!(interner.resolve(SYM_FONT_SIZE), "font-size");
        assert_eq!(interner.resolve(SYM_POSITION), "position");
    }

    #[test]
    fn test_pre_seeded_count() {
        let interner = Interner::new();
        assert_eq!(interner.len(), PRE_SEEDED.len());
    }

    #[test]
    fn test_pre_seeded_all_exhaustive() {
        let interner = Interner::new();
        for (index, &expected) in PRE_SEEDED.iter().enumerate() {
            let sym = Symbol(index as u32);
            assert_eq!(
                interner.resolve(sym),
                expected,
                "Pre-seeded entry at index {} did not match expected symbol constant",
                index
            );
        }
    }

    #[test]
    fn test_pre_seeded_constant_matches_intern() {
        let mut interner = Interner::new();
        // Interning a pre-seeded string should return the known constant
        assert_eq!(interner.intern("div"), SYM_DIV);
        assert_eq!(interner.intern("color"), SYM_COLOR);
        assert_eq!(interner.intern("margin-top"), SYM_MARGIN_TOP);
        assert_eq!(interner.intern("style"), SYM_STYLE);
        assert_eq!(interner.intern("class"), SYM_CLASS);
    }

    // ── Lookup ───────────────────────────────────────────────────

    #[test]
    fn test_lookup_existing() {
        let interner = Interner::new();
        assert_eq!(interner.lookup("div"), Some(SYM_DIV));
        assert_eq!(interner.lookup("color"), Some(SYM_COLOR));
    }

    #[test]
    fn test_lookup_nonexistent() {
        let interner = Interner::new();
        assert_eq!(interner.lookup("nonexistent-tag"), None);
    }

    // ── Symbol Properties ────────────────────────────────────────

    #[test]
    fn test_symbol_copy_and_clone() {
        let sym = SYM_DIV;
        let copy = sym;       // Copy
        let clone = sym.clone(); // Clone
        assert_eq!(sym, copy);
        assert_eq!(sym, clone);
    }

    #[test]
    fn test_symbol_ordering() {
        assert!(SYM_HTML < SYM_HEAD); // Symbol(0) < Symbol(1)
        assert!(SYM_DIV < SYM_COLOR); // Symbol(3) < Symbol(46)
    }

    #[test]
    fn test_symbol_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SYM_DIV);
        set.insert(SYM_P);
        set.insert(SYM_DIV); // duplicate
        assert_eq!(set.len(), 2);
    }

    // ── Growth ───────────────────────────────────────────────────

    #[test]
    fn test_intern_grows_interner() {
        let mut interner = Interner::new();
        let initial = interner.len();
        interner.intern("brand-new-string");
        assert_eq!(interner.len(), initial + 1);
    }

    #[test]
    fn test_intern_duplicate_does_not_grow() {
        let mut interner = Interner::new();
        let initial = interner.len();
        interner.intern("div"); // already pre-seeded
        assert_eq!(interner.len(), initial);
    }

    #[test]
    fn test_is_empty() {
        let interner = Interner::new();
        assert!(!interner.is_empty(), "should not be empty after pre-seeding");
    }
}
