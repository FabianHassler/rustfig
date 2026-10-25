//! A quick Unicode approximation of simple LaTeX math, for places that show
//! special text without typesetting it (library icons and preview). No TeX.

/// Turn simple LaTeX (`$\lambda/2$`, `R_x(\theta)`, `$|0\rangle$`, `e^-`) into
/// Unicode text. Never fails: what it doesn't know is kept readable.
pub fn latex_approx(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::new();
    run(&chars, &mut out, 0);
    out
}

const MAX_DEPTH: u32 = 16;

fn run(c: &[char], out: &mut String, depth: u32) {
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '$' | '{' | '}' => i += 1,
            '~' => {
                out.push(' ');
                i += 1;
            }
            '^' | '_' => {
                let sup = c[i] == '^';
                let (arg, next) = argument(c, i + 1);
                let inner = approx_chars(&arg, depth + 1);
                match inner.chars().map(|ch| script(ch, sup)).collect::<Option<String>>() {
                    Some(s) if !inner.is_empty() => out.push_str(&s),
                    _ => {
                        out.push(c[i]);
                        out.push_str(&inner);
                    }
                }
                i = next;
            }
            '\\' => {
                let mut j = i + 1;
                while j < c.len() && c[j].is_ascii_alphabetic() {
                    j += 1;
                }
                if j == i + 1 {
                    // A control symbol: `\,` `\;` `\ ` are spaces, `\{` `\%` literal.
                    if let Some(&s) = c.get(j) {
                        match s {
                            ',' | ';' | ':' | ' ' => out.push(' '),
                            '!' => {}
                            '|' => out.push('‖'),
                            _ => out.push(s),
                        }
                        j += 1;
                    }
                    i = j;
                    continue;
                }
                let name: String = c[i + 1..j].iter().collect();
                i = j;
                match name.as_str() {
                    "frac" | "dfrac" | "tfrac" => {
                        let (a, n1) = argument(c, i);
                        let (b, n2) = argument(c, n1);
                        let (a, b) = (approx_chars(&a, depth + 1), approx_chars(&b, depth + 1));
                        out.push_str(&wrap(&a));
                        out.push('/');
                        out.push_str(&wrap(&b));
                        i = n2;
                    }
                    "sqrt" => {
                        let (a, n) = argument(c, i);
                        let a = approx_chars(&a, depth + 1);
                        out.push('√');
                        out.push_str(&wrap(&a));
                        i = n;
                    }
                    "mathrm" | "text" | "mathbf" | "textbf" | "textit" | "mathit" | "mathcal" | "mathsf" | "mathtt"
                    | "textrm" | "operatorname" | "boldsymbol" | "bm" | "left" | "right" | "mbox" => {
                        // The content follows as ordinary text (braces are dropped).
                    }
                    _ => match symbol(&name) {
                        Some(s) => out.push_str(s),
                        None => out.push_str(&name),
                    },
                }
            }
            ch => {
                out.push(ch);
                i += 1;
            }
        }
    }
}

fn approx_chars(c: &[char], depth: u32) -> String {
    if depth > MAX_DEPTH {
        return c.iter().collect();
    }
    let mut s = String::new();
    run(c, &mut s, depth);
    s
}

/// Parenthesise multi-character operands of a fraction or root.
fn wrap(s: &str) -> String {
    if s.chars().count() > 1 && !s.chars().all(|c| c.is_alphanumeric() || c == '√') {
        format!("({s})")
    } else {
        s.to_string()
    }
}

/// The argument starting at `i`: a `{…}` group (to its matching brace, or the
/// end when unbalanced), a `\command`, or one character. Returns it and the
/// index after it.
fn argument(c: &[char], mut i: usize) -> (Vec<char>, usize) {
    while c.get(i) == Some(&' ') {
        i += 1;
    }
    match c.get(i) {
        None => (Vec::new(), i),
        Some('{') => {
            let mut level = 0usize;
            for (j, &ch) in c.iter().enumerate().skip(i) {
                match ch {
                    '{' => level += 1,
                    '}' => {
                        level = level.saturating_sub(1);
                        if level == 0 {
                            return (c[i + 1..j].to_vec(), j + 1);
                        }
                    }
                    _ => {}
                }
            }
            (c[i + 1..].to_vec(), c.len())
        }
        Some('\\') => {
            let mut j = i + 1;
            while j < c.len() && c[j].is_ascii_alphabetic() {
                j += 1;
            }
            if j == i + 1 {
                j = (j + 1).min(c.len());
            }
            (c[i..j].to_vec(), j)
        }
        Some(&ch) => (vec![ch], i + 1),
    }
}

fn script(ch: char, sup: bool) -> Option<char> {
    const SUP: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁼⁽⁾ⁱⁿᵃᵇᶜᵈᵉᶠᵍʰʲᵏˡᵐᵒᵖʳˢᵗᵘᵛʷˣʸᶻ";
    const SUP_SRC: &str = "0123456789+-=()inabcdefghjklmoprstuvwxyz";
    const SUB: &str = "₀₁₂₃₄₅₆₇₈₉₊₋₌₍₎ₐₑₕᵢⱼₖₗₘₙₒₚᵣₛₜᵤᵥₓ";
    const SUB_SRC: &str = "0123456789+-=()aehijklmnoprstuvx";
    let ch = if ch == '−' { '-' } else { ch };
    let (dst, src) = if sup { (SUP, SUP_SRC) } else { (SUB, SUB_SRC) };
    let p = src.chars().position(|s| s == ch)?;
    dst.chars().nth(p)
}

fn symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" | "varepsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" => "θ",
        "vartheta" => "ϑ",
        "iota" => "ι",
        "kappa" => "κ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "omicron" => "ο",
        "pi" => "π",
        "rho" => "ρ",
        "sigma" => "σ",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" | "varphi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Upsilon" => "Υ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        "pm" => "±",
        "mp" => "∓",
        "times" => "×",
        "cdot" => "·",
        "to" | "rightarrow" => "→",
        "leftarrow" => "←",
        "leftrightarrow" => "↔",
        "Rightarrow" => "⇒",
        "langle" => "⟨",
        "rangle" => "⟩",
        "infty" => "∞",
        "partial" => "∂",
        "hbar" => "ℏ",
        "dagger" => "†",
        "nabla" => "∇",
        "sum" => "∑",
        "int" => "∫",
        "approx" => "≈",
        "neq" | "ne" => "≠",
        "leq" | "le" => "≤",
        "geq" | "ge" => "≥",
        "circ" => "∘",
        "prime" => "′",
        "ldots" | "dots" | "cdots" => "…",
        "otimes" => "⊗",
        "oplus" => "⊕",
        "sim" => "∼",
        "propto" => "∝",
        "quad" | "qquad" => " ",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::latex_approx as a;

    #[test]
    fn examples() {
        assert_eq!(a(r"$\lambda/2$"), "λ/2");
        assert_eq!(a(r"$R_x(\theta)$"), "Rₓ(θ)");
        assert_eq!(a(r"$|0\rangle$"), "|0⟩");
        assert_eq!(a(r"$e^-$"), "e⁻");
        assert_eq!(a(r"$x^{2}+y_{12}$"), "x²+y₁₂");
        assert_eq!(a(r"\frac{a}{b}"), "a/b");
        assert_eq!(a(r"\frac{a+b}{2}"), "(a+b)/2");
        assert_eq!(a(r"\sqrt{x}"), "√x");
        assert_eq!(a(r"\mathrm{CNOT}"), "CNOT");
        assert_eq!(a(r"\text{ab}\,c\;d~e"), "ab c d e");
        assert_eq!(a(r"\foo"), "foo");
        assert_eq!(a(r"\pm \times \to \infty \hbar \Omega"), "± × → ∞ ℏ Ω");
    }

    #[test]
    fn scripts_without_unicode_form_keep_marker() {
        assert_eq!(a("x^{q}"), "x^q");
        assert_eq!(a("x_{b}"), "x_b");
        assert_eq!(a("x^{-1}"), "x⁻¹");
    }

    #[test]
    fn nested_and_edge_cases() {
        assert_eq!(a(r"\frac{\alpha}{\sqrt{2}}"), "α/√2");
        assert_eq!(a(r"e^{i\theta}"), "e^iθ");
        assert_eq!(a(""), "");
        assert_eq!(a("plain text 1/2"), "plain text 1/2");
        assert_eq!(a(r"\frac{a"), "a/");
        assert_eq!(a("x^{2"), "x²");
        assert_eq!(a("}{ x"), " x");
        assert_eq!(a("x^"), "x^");
        assert_eq!(a(r"\"), "");
        assert_eq!(a(&"{".repeat(5000)), "");
        assert!(!a(&"^{".repeat(5000)).is_empty());
    }
}
