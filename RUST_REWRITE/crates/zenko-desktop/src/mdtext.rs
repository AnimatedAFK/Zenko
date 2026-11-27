//! zenko.py's rich text for chat replies and notes: md_segments (Markdown blocks), md_inline (_INLINE), tex_segments
//! (simple LaTeX -> readable text), chem_segments (\ce{} and H2SO4), md_plain. Each gives [(text, tags)] for a Tk text
//! widget. One difference on purpose: a lone backslash at the very end of a formula is kept as a backslash (zenko.py
//! raises there and the reply never shows).

use crate::mdtex_data::{GREEK, TEX};
use fancy_regex::Regex as FRegex;
use regex::Regex;
use std::sync::OnceLock;

pub type Seg = (String, Vec<String>);

const SUP_FROM: &str = "0123456789+-=()n";
const SUP_TO: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁼⁽⁾ⁿ";
const SUB_FROM: &str = "0123456789+-=()";
const SUB_TO: &str = "₀₁₂₃₄₅₆₇₈₉₊₋₌₍₎";

fn translate(s: &str, from: &str, to: &str) -> String {
    s.chars().map(|c| from.chars().position(|f| f == c).map(|k| to.chars().nth(k).unwrap()).unwrap_or(c)).collect()
}

/// str.translate(_SUP)
pub fn sup(s: &str) -> String {
    translate(s, SUP_FROM, SUP_TO)
}

/// str.translate(_SUB)
pub fn sub(s: &str) -> String {
    translate(s, SUB_FROM, SUB_TO)
}

/// Python's str.isspace
fn py_space(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// Python's str.strip()
pub fn py_strip(s: &str) -> &str {
    s.trim_matches(py_space)
}

fn tex(name: &str) -> Option<&'static str> {
    TEX.binary_search_by(|(k, _)| (*k).cmp(name)).ok().map(|i| TEX[i].1)
}

fn tags_plus(tags: &[String], extra: &[&str]) -> Vec<String> {
    let mut t = tags.to_vec();
    t.extend(extra.iter().map(|s| s.to_string()));
    t
}

macro_rules! re {
    ($name:ident, $kind:ty, $pat:expr) => {
        fn $name() -> &'static $kind {
            static R: OnceLock<$kind> = OnceLock::new();
            R.get_or_init(|| <$kind>::new($pat).unwrap())
        }
    };
}

re!(re_matrix, FRegex, r"(?s)\\begin\{([pbBvV]?matrix|smallmatrix|cases)\}(.*?)\\end\{\1\}");
re!(re_row, Regex, r"\\\\(?:\[[^\]]*\])?");
re!(re_chem, FRegex, r"(\^\{?[0-9]*[+-]\}?|\^[0-9]+)|((?<=[A-Za-z)\]])[0-9]+)|([^\^0-9]+|[0-9]+)");
re!(re_inline, FRegex, concat!(
    r"(?s)(?P<code>`+)(?P<codet>.+?)(?P=code)",
    r"|\$\$(?P<dm>.+?)\$\$|\\\[(?P<bm>.+?)\\\]|\\\((?P<pm>.+?)\\\)|(?<![\w$\\])\$(?=\S)(?P<m>[^$\n]*?\S)\$(?!\d)",
    r"|\*\*\*(?P<bi>.+?)\*\*\*|___(?P<bi2>.+?)___",
    r"|\*\*(?P<b>.+?)\*\*|(?<!\w)__(?P<b2>.+?)__(?!\w)",
    r"|(?<![\w*])\*(?P<i>[^\s*](?:.*?[^\s*])?)\*(?![\w*])|(?<![\w_])_(?P<i2>[^\s_](?:.*?[^\s_])?)_(?![\w_])",
    r"|~~(?P<s>.+?)~~",
    r"|\\ce\{(?P<ce>[^}]*(?:\{[^}]*\}[^}]*)*)\}",
    r"|(?<=[\w)\]])\^(?:\{(?P<sp>[^}]*)\}|(?P<sp2>[0-9]+[+-]?|[+-]|[a-z]\b))",
    r"|(?<=[A-Za-z)\]])_\{(?P<sb>[^}]*)\}",
    r"|\[(?P<lt>[^\]]+)\]\((?P<lu>https?://[^)\s]+)\)",
    r"|<sup>(?P<sup>.*?)</sup>|<sub>(?P<sub>.*?)</sub>|<b>(?P<hb>.*?)</b>|<i>(?P<hi>.*?)</i>|<br\s*/?>(?P<br>)"));
re!(re_stars, Regex, r"\*{2,}");
re!(re_dollar, FRegex, r"\$\$|(?<!\\)\$(?!\d)");
re!(re_bare, Regex, r"\\[A-Za-z]{2,}|\\[{}()\[\],;!]|[\w)\]]\^\{");
re!(re_paren, Regex, r"\\[()\[\]]");
re!(re_tab, FRegex, r"\t(?=(?:imes|heta|ext|an\b|au\b|riangle|o\b|herefore|ilde|frac|extbf|extit))");
re!(re_cr, FRegex, r"\r(?=(?:ightarrow|ho\b|ight\b|angle|ceil|floor|vert|m\b))");
re!(re_four, Regex, r"\*{4,}");
re!(re_sep, Regex, r"^[\s:|-]+$");
re!(re_head, Regex, r"^(#{1,6})\s+(.*?)\s*#*\s*$");
re!(re_hr, Regex, r"^(?:[-*_]\s*){3,}$");
re!(re_quote, Regex, r"^\s*>\s?(.*)$");
re!(re_bullet, Regex, r"^(\s*)([-*+•])\s+(.*)$");
re!(re_num, Regex, r"^(\s*)(\d+|[a-z])([.)])\s+(.*)$");
re!(re_simple, Regex, r"^[\w.]+$");

/// _brace: the {...} group at s[i] (or one character, or one \command)
fn brace(s: &[char], mut i: usize) -> (String, usize) {
    while i < s.len() && s[i] == ' ' {
        i += 1;
    }
    if i >= s.len() {
        return (String::new(), i);
    }
    if s[i] != '{' {
        if s[i] == '\\' {
            let n = s[i + 1..].iter().take_while(|c| c.is_ascii_alphabetic()).count();
            if n > 0 {
                return (s[i..i + 1 + n].iter().collect(), i + 1 + n);
            }
        }
        return (s[i].to_string(), i + 1);
    }
    let mut depth = 0;
    for j in i..s.len() {
        if s[j] == '{' {
            depth += 1;
        } else if s[j] == '}' {
            depth -= 1;
            if depth == 0 {
                return (s[i + 1..j].iter().collect(), j + 1);
            }
        }
    }
    (s[i + 1..].iter().collect(), s.len())
}

fn matrix(env: &str, body: &str) -> String {
    let rows: Vec<String> = re_row().split(body).map(py_strip).filter(|r| !r.is_empty())
        .map(|r| r.split('&').map(py_strip).collect::<Vec<_>>().join(", ")).collect();
    if env == "cases" {
        return format!("{{ {} }}", rows.join(";  "));
    }
    let (l, r) = match env {
        "pmatrix" => ("(", ")"),
        "bmatrix" => ("[", "]"),
        "Bmatrix" => ("{", "}"),
        "vmatrix" => ("|", "|"),
        "Vmatrix" => ("‖", "‖"),
        _ => ("(", ")"),
    };
    format!("{l}{}{r}", rows.join("; "))
}

/// tex_segments
pub fn tex_segments(s: &str, tags: &[String]) -> Vec<Seg> {
    let s = re_matrix().replace_all(s, |c: &fancy_regex::Captures| matrix(&c[1], &c[2])).into_owned();
    let s: Vec<char> = s.chars().collect();
    let mut out: Vec<Seg> = vec![];
    fn add(out: &mut Vec<Seg>, tags: &[String], t: &str, extra: &[String]) {
        if !t.is_empty() {
            let mut tg = tags.to_vec();
            tg.extend(extra.iter().cloned());
            out.push((t.to_string(), tg));
        }
    }
    let mut i = 0;
    let mut buf = String::new();
    while i < s.len() {
        let ch = s[i];
        if ch == '\\' {
            let rest = &s[i + 1..];
            let name: String = if rest.len() >= 9 && rest[..7].iter().collect::<String>() == "mathbb{" && rest[7].is_ascii_uppercase()
                && rest[8] == '}' {
                rest[..9].iter().collect()
            } else if rest.first().is_some_and(|c| c.is_ascii_alphabetic()) {
                rest.iter().take_while(|c| c.is_ascii_alphabetic()).collect()
            } else if rest.first().is_some_and(|&c| c != '\n') {
                rest[0].to_string()
            } else {
                buf.push('\\');
                i += 1;
                continue;
            };
            i += 1 + name.chars().count();
            let n = name.as_str();
            match n {
                "begin" | "end" => {
                    let (env, j) = brace(&s, i);
                    i = j;
                    if n == "begin" && (env == "array" || env == "tabular") && i < s.len() && s[i] == '{' {
                        i = brace(&s, i).1;
                    }
                    if n == "end" {
                        buf.push('\n');
                    }
                }
                "binom" | "dbinom" | "tbinom" | "choose" => {
                    let (a, j) = brace(&s, i);
                    let (b, j) = brace(&s, j);
                    i = j;
                    add(&mut out, tags, &buf, &[]);
                    buf.clear();
                    add(&mut out, tags, "(", &[]);
                    out.extend(tex_segments(&a, tags));
                    add(&mut out, tags, " C ", &[]);
                    out.extend(tex_segments(&b, tags));
                    add(&mut out, tags, ")", &[]);
                }
                "color" | "textcolor" | "phantom" | "hphantom" | "vphantom" | "label" | "tag" => {
                    i = brace(&s, i).1;
                    if n == "color" || n == "textcolor" {
                        let (a, j) = brace(&s, i);
                        i = j;
                        let o = tex_segments(&a, tags);
                        add(&mut out, tags, &buf, &[]);
                        buf.clear();
                        out.extend(o);
                    }
                }
                "boxed" | "fbox" | "underline" | "mathcal" | "mathscr" | "mathfrak" | "mathsf" | "mathtt" | "textit" | "emph"
                | "textsf" | "texttt" | "text" | "mathrm" | "textrm" | "mathbf" | "textbf" | "mathit" | "operatorname"
                | "mbox" | "boldsymbol" | "bm" | "hbox" | "textnormal" | "mathop" | "ensuremath" => {
                    let (a, j) = brace(&s, i);
                    i = j;
                    add(&mut out, tags, &buf, &[]);
                    buf.clear();
                    if matches!(n, "text" | "textrm" | "mbox" | "hbox" | "textnormal" | "textit" | "emph" | "textsf" | "texttt") {
                        add(&mut out, tags, &a, &[]);
                    } else {
                        out.extend(tex_segments(&a, tags));
                    }
                }
                "dot" | "ddot" | "tilde" | "widetilde" | "widehat" | "overrightarrow" | "overleftarrow" | "underbrace"
                | "overbrace" | "check" | "breve" | "acute" | "grave" => {
                    let (a, j) = brace(&s, i);
                    i = j;
                    let mark = match n {
                        "dot" => "\u{307}",
                        "ddot" => "\u{308}",
                        "tilde" | "widetilde" => "\u{303}",
                        "widehat" => "\u{302}",
                        "overrightarrow" => "\u{20d7}",
                        "overleftarrow" => "\u{20d6}",
                        "check" => "\u{30c}",
                        "breve" => "\u{306}",
                        "acute" => "\u{301}",
                        "grave" => "\u{300}",
                        _ => "",
                    };
                    buf.push_str(&a);
                    buf.push_str(mark);
                }
                "hline" | "cline" | "nonumber" | "notag" | "noindent" | "centering" | "small" | "large" | "Large"
                | "normalsize" | "scriptstyle" | "textstyle" | "nolimits" | "middle" | "bigl" | "bigr" | "Bigl" | "Bigr"
                | "biggl" | "biggr" | "Biggl" | "Biggr" | "Big" | "bigg" | "Bigg" => {}
                "frac" | "dfrac" | "tfrac" | "cfrac" => {
                    let (a, j) = brace(&s, i);
                    let (b, j) = brace(&s, j);
                    i = j;
                    add(&mut out, tags, &buf, &[]);
                    buf.clear();
                    let simple = |x: &str| re_simple().is_match(x);
                    out.extend(tex_segments(&if simple(&a) { a.clone() } else { format!("({a})") }, tags));
                    add(&mut out, tags, "/", &[]);
                    out.extend(tex_segments(&if simple(&b) { b.clone() } else { format!("({b})") }, tags));
                }
                "sqrt" => {
                    let mut nn = String::new();
                    if i < s.len() && s[i] == '[' {
                        // s.find("]", i): -1 when missing gives s[i+1:-1] and i = 0 in zenko.py
                        match s[i..].iter().position(|&c| c == ']') {
                            Some(k) => {
                                nn = s[i + 1..i + k].iter().collect();
                                i += k + 1;
                            }
                            None => {
                                nn = if i + 1 < s.len() { s[i + 1..s.len() - 1].iter().collect() } else { String::new() };
                                i = 0;
                            }
                        }
                    }
                    let (a, j) = brace(&s, i);
                    i = j;
                    add(&mut out, tags, &buf, &[]);
                    buf.clear();
                    if !nn.is_empty() {
                        add(&mut out, tags, &sup(&nn), &[]);
                    }
                    add(&mut out, tags, "√(", &[]);
                    out.extend(tex_segments(&a, tags));
                    add(&mut out, tags, ")", &[]);
                }
                "ce" | "pu" => {
                    let (a, j) = brace(&s, i);
                    i = j;
                    add(&mut out, tags, &buf, &[]);
                    buf.clear();
                    out.extend(chem_segments(&a, tags));
                }
                "left" | "right" | "big" | "displaystyle" | "limits" | "," => {
                    if n == "," {
                        buf.push(' ');
                    }
                    if (n == "left" || n == "right") && i < s.len() && s[i] == '.' {
                        i += 1;
                    }
                }
                "overline" | "bar" | "vec" | "hat" => {
                    let (a, j) = brace(&s, i);
                    i = j;
                    buf.push_str(&a);
                    buf.push_str(match n {
                        "overline" => "\u{305}",
                        "bar" => "\u{304}",
                        "vec" => "\u{20d7}",
                        _ => "\u{302}",
                    });
                }
                ";" | ":" | "!" | " " => buf.push(' '),
                "{" | "}" | "$" | "_" | "#" | "&" => buf.push_str(n),
                "\\" => buf.push('\n'),
                _ => {
                    buf.push_str(tex(n).unwrap_or(n));
                    if GREEK.contains(&n) {
                        // \Delta H -> ΔH: " +[A-Za-z0-9](?![A-Za-z])"
                        let sp = s[i..].iter().take_while(|&&c| c == ' ').count();
                        let k = i + sp;
                        if sp > 0 && k < s.len() && s[k].is_ascii_alphanumeric()
                            && !s.get(k + 1).is_some_and(|c| c.is_ascii_alphabetic()) {
                            i = k;
                        }
                    }
                }
            }
        } else if ch == '^' || ch == '_' {
            let (a, j) = brace(&s, i + 1);
            i = j;
            add(&mut out, tags, &buf, &[]);
            buf.clear();
            let is_sub = ch == '_';
            if (a == "\\circ" || a == "\\degree" || a == "o") && !is_sub {
                add(&mut out, tags, "°", &[]);
                continue;
            }
            for (t, tg) in tex_segments(&a, &[]) {
                let mut extra = tg;
                extra.push(if is_sub { "md_sub" } else { "md_sup" }.into());
                add(&mut out, tags, &t, &extra);
            }
        } else if ch == '{' || ch == '}' {
            i += 1;
        } else if ch == '~' || ch == '&' {
            buf.push(' ');
            i += 1;
        } else {
            buf.push(ch);
            i += 1;
        }
    }
    add(&mut out, tags, &buf, &[]);
    out
}

/// chem_segments: H2SO4 / Fe^{3+} / CO3^2- as lowered and raised text, arrows for -> and <=>
pub fn chem_segments(s: &str, tags: &[String]) -> Vec<Seg> {
    let s = s.replace("<=>", "⇌").replace("<->", "⇌").replace("->", "→").replace("<-", "←");
    let mut out = vec![];
    for m in re_chem().captures_iter(&s) {
        let m = m.unwrap();
        if let Some(g) = m.get(1).filter(|g| !g.as_str().is_empty()) {
            out.push((g.as_str().trim_matches(|c| c == '^' || c == '{' || c == '}').to_string(), tags_plus(tags, &["md_sup"])));
        } else if let Some(g) = m.get(2).filter(|g| !g.as_str().is_empty()) {
            out.push((g.as_str().to_string(), tags_plus(tags, &["md_sub"])));
        } else if let Some(g) = m.get(3) {
            out.push((g.as_str().to_string(), tags.to_vec()));
        }
    }
    out
}

/// md_inline
pub fn md_inline(text: &str, tags: &[String]) -> Vec<Seg> {
    let mut out: Vec<Seg> = vec![];
    let mut pos = 0;
    for m in re_inline().captures_iter(text) {
        let m = m.unwrap();
        let whole = m.get(0).unwrap();
        if whole.start() > pos {
            out.push((text[pos..whole.start()].to_string(), tags.to_vec()));
        }
        pos = whole.end();
        let g = |n: &str| m.name(n).map(|x| x.as_str());
        let t = |n: &str| g(n).filter(|x| !x.is_empty());
        if let Some(c) = g("codet") {
            out.push((c.into(), tags_plus(tags, &["md_code"])));
        } else if let Some(x) = t("dm").or(t("bm")).or(t("pm")).or(t("m")) {
            out.extend(tex_segments(py_strip(x), &tags_plus(tags, &["md_math"])));
        } else if let Some(x) = t("bi").or(t("bi2")) {
            out.extend(md_inline(x, &tags_plus(tags, &["md_bi"])));
        } else if let Some(x) = t("b").or(t("b2")).or(t("hb")) {
            out.extend(md_inline(x, &tags_plus(tags, &["md_b"])));
        } else if let Some(x) = t("i").or(t("i2")).or(t("hi")) {
            out.extend(md_inline(x, &tags_plus(tags, &["md_i"])));
        } else if let Some(x) = t("s") {
            out.extend(md_inline(x, &tags_plus(tags, &["md_s"])));
        } else if let Some(x) = g("ce") {
            out.extend(chem_segments(x, tags));
        } else if let Some(x) = g("sp").or(g("sp2")) {
            out.extend(tex_segments(x, &tags_plus(tags, &["md_sup"])));
        } else if let Some(x) = g("sb") {
            out.extend(tex_segments(x, &tags_plus(tags, &["md_sub"])));
        } else if let Some(x) = t("lt") {
            out.push((x.into(), tags_plus(tags, &["md_link", &format!("url:{}", g("lu").unwrap())])));
        } else if let Some(x) = g("sup") {
            out.push((x.into(), tags_plus(tags, &["md_sup"])));
        } else if let Some(x) = g("sub") {
            out.push((x.into(), tags_plus(tags, &["md_sub"])));
        } else if g("br").is_some() {
            out.push(("\n".into(), tags.to_vec()));
        }
    }
    if pos < text.len() {
        out.push((text[pos..].to_string(), tags.to_vec()));
    }
    let mut fixed = vec![];
    for (t, tg) in out {
        if tg.iter().any(|x| x == "md_code" || x == "md_math" || x == "md_pre" || x.starts_with("url:")) {
            fixed.push((t, tg));
            continue;
        }
        let t = re_stars().replace_all(&t, "").into_owned();
        let t = re_dollar().replace_all(&t, "").into_owned();
        if re_bare().is_match(&t) {
            let t = re_paren().replace_all(&t, "").into_owned();
            for (t2, tg2) in tex_segments(&t.replace('~', "\x01"), &tg) {
                fixed.push((t2.replace('\x01', "~"), tg2));
            }
            continue;
        }
        fixed.push((t, tg));
    }
    fixed
}

fn plain_inline(t: &str) -> String {
    md_inline(t, &[]).into_iter().map(|(x, _)| x).collect()
}

/// clip(text, n)
pub fn clip(text: &str, n: usize) -> String {
    if text.chars().count() <= n {
        text.to_string()
    } else {
        text.chars().take(n - 1).collect::<String>() + "…"
    }
}

fn ljust(s: &str, w: usize) -> String {
    let n = s.chars().count();
    if n >= w {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(w - n))
    }
}

/// md_segments
pub fn md_segments(text: &str, base: &[&str]) -> Vec<Seg> {
    let text = text.replace("\r\n", "\n");
    let text = text.replace('\x0c', "\\f").replace('\x08', "\\b").replace('\x0b', "\\v");
    let text = re_tab().replace_all(&text, "\\t").into_owned();
    let text = re_cr().replace_all(&text, "\\r").into_owned();
    let text = re_four().replace_all(&text, "**").into_owned();
    let lines: Vec<&str> = text.split('\n').collect();
    let base: Vec<String> = base.iter().map(|s| s.to_string()).collect();
    let b = |extra: &[&str]| tags_plus(&base, extra);
    let lvl_of = |ind: &str| (ind.replace('\t', "    ").chars().count() / 2).min(3);
    let mut out: Vec<Seg> = vec![];
    let mut i = 0;
    while i < lines.len() {
        let ln = lines[i];
        let st = py_strip(ln);
        if st.starts_with("```") {
            let mut j = i + 1;
            let mut code = vec![];
            while j < lines.len() && !py_strip(lines[j]).starts_with("```") {
                code.push(lines[j]);
                j += 1;
            }
            out.push((code.join("\n") + "\n", b(&["md_pre"])));
            i = j + 1;
            continue;
        }
        if st.starts_with("$$") && st.matches("$$").count() == 1 {
            let mut j = i + 1;
            let mut eq = vec![st[2..].to_string()];
            while j < lines.len() && !lines[j].contains("$$") {
                eq.push(lines[j].to_string());
                j += 1;
            }
            if j < lines.len() {
                eq.push(lines[j].replace("$$", ""));
            }
            out.push(("    ".into(), base.clone()));
            let joined: Vec<&str> = eq.iter().map(|x| py_strip(x)).filter(|x| !x.is_empty()).collect();
            out.extend(tex_segments(&joined.join(" "), &b(&["md_math"])));
            out.push(("\n".into(), base.clone()));
            i = j + 1;
            continue;
        }
        if st.starts_with('|') && st.matches('|').count() >= 2 {
            let mut rows: Vec<Vec<String>> = vec![];
            while i < lines.len() && py_strip(lines[i]).starts_with('|') {
                let r = py_strip(lines[i]).trim_matches('|');
                if !re_sep().is_match(r) {
                    rows.push(r.split('|').map(|c| plain_inline(py_strip(c))).collect());
                }
                i += 1;
            }
            let ncol = rows.iter().map(|r| r.len()).max().unwrap_or(0);
            for r in rows.iter_mut() {
                r.resize(ncol, String::new());
            }
            let wid: Vec<usize> = (0..ncol).map(|k| rows.iter().map(|r| r[k].chars().count()).max().unwrap().min(40)).collect();
            for (n, r) in rows.iter().enumerate() {
                let cells: Vec<String> = r.iter().enumerate().map(|(k, c)| ljust(&clip(c, 40), wid[k])).collect();
                out.push((format!("  {}\n", cells.join("  │  ")), b(&[if n == 0 { "md_th" } else { "md_table" }])));
                if n == 0 && rows.len() > 1 {
                    let bars: Vec<String> = wid.iter().map(|&w| "─".repeat(w)).collect();
                    out.push((format!("  {}\n", bars.join("──┼──")), b(&["md_table"])));
                }
            }
            continue;
        }
        if let Some(m) = re_head().captures(st) {
            let lvl = m[1].len().min(3);
            out.extend(md_inline(&m[2], &b(&[&format!("md_h{lvl}")])));
            out.push(("\n".into(), base.clone()));
            i += 1;
            continue;
        }
        if re_hr().is_match(st) {
            out.push(("─".repeat(30) + "\n", b(&["md_hr"])));
            i += 1;
            continue;
        }
        if let Some(m) = re_quote().captures(ln) {
            out.push(("▎ ".into(), b(&["md_quote"])));
            out.extend(md_inline(&m[1], &b(&["md_quote"])));
            out.push(("\n".into(), base.clone()));
            i += 1;
            continue;
        }
        if let Some(m) = re_bullet().captures(ln) {
            let lvl = lvl_of(&m[1]);
            let li = format!("md_li{lvl}");
            out.push((format!("{} ", "•◦▪▫".chars().nth(lvl).unwrap()), b(&[&li, "md_bullet"])));
            out.extend(md_inline(&m[3], &b(&[&li])));
            out.push(("\n".into(), base.clone()));
            i += 1;
            continue;
        }
        if let Some(m) = re_num().captures(ln) {
            let lvl = lvl_of(&m[1]);
            let li = format!("md_li{lvl}");
            out.push((format!("{}{} ", &m[2], &m[3]), b(&[&li, "md_num"])));
            out.extend(md_inline(&m[4], &b(&[&li])));
            out.push(("\n".into(), base.clone()));
            i += 1;
            continue;
        }
        out.extend(md_inline(ln, &base));
        out.push(("\n".into(), base.clone()));
        i += 1;
    }
    while out.last().is_some_and(|s| s.0 == "\n") {
        out.pop();
    }
    out.into_iter().map(|(t, tg)| {
        if tg.iter().any(|x| x.starts_with("md_h")) {
            (t, tg.into_iter().filter(|x| !matches!(x.as_str(), "md_b" | "md_i" | "md_bi")).collect())
        } else if tg.iter().any(|x| x == "md_b") && tg.iter().any(|x| x == "md_i") {
            let mut v: Vec<String> = tg.into_iter().filter(|x| x != "md_b" && x != "md_i").collect();
            v.push("md_bi".into());
            (t, v)
        } else {
            (t, tg)
        }
    }).collect()
}

/// md_plain: the text without Markdown marks (bubbles, speech, saved history)
pub fn md_plain(text: &str) -> String {
    md_segments(text, &[]).into_iter().map(|(t, tg)| {
        if tg.iter().any(|x| x == "md_sup") {
            sup(&t)
        } else if tg.iter().any(|x| x == "md_sub") {
            sub(&t)
        } else {
            t
        }
    }).collect()
}
