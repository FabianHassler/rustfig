//! xfig's compose sequences: Meta (Alt) pressed and released alone while
//! typing text, then two characters, give one (`CompKeyDB`).
//
// Ported from xfig CompKeyDB (the compose key database) and src/w_canvas.c
// (`setup_canvas` reading it) (xfig licence, see LICENSE-xfig).

/// The two characters after Meta and the character they make.
#[rustfmt::skip]
const SEQUENCES: &[(char, char, char)] = &[
    ('o', 'e', 'œ'),
    ('O', 'E', 'Œ'),
    ('!', '!', '¡'),
    ('C', '/', '¢'),
    ('L', '-', '£'),
    ('O', 'X', '¤'),
    ('Y', '-', '¥'),
    ('|', '|', '¦'),
    ('S', 'O', '§'),
    ('"', '"', '¨'),
    ('C', 'O', '©'),
    ('_', 'a', 'ª'),
    ('<', '<', '«'),
    ('-', '|', '¬'),
    ('-', '-', '\u{AD}'), // soft hyphen
    ('R', 'O', '®'),
    ('^', '-', '¯'),
    ('^', '*', '°'),
    ('+', '-', '±'),
    ('^', '2', '²'),
    ('^', '3', '³'),
    ('\\', '\\', '´'),
    ('/', 'u', 'µ'),
    ('P', '!', '¶'),
    ('^', '.', '·'),
    (',', ',', '¸'),
    ('^', '1', '¹'),
    ('_', 'o', 'º'),
    ('>', '>', '»'),
    ('1', '4', '¼'),
    ('1', '2', '½'),
    ('3', '4', '¾'),
    ('?', '?', '¿'),
    ('A', '`', 'À'),
    ('A', '\'', 'Á'),
    ('A', '^', 'Â'),
    ('A', '~', 'Ã'),
    ('A', '"', 'Ä'),
    ('A', '*', 'Å'),
    ('A', 'E', 'Æ'),
    ('C', ',', 'Ç'),
    ('E', '`', 'È'),
    ('E', '\'', 'É'),
    ('E', '^', 'Ê'),
    ('E', '"', 'Ë'),
    ('I', '`', 'Ì'),
    ('I', '\'', 'Í'),
    ('I', '^', 'Î'),
    ('I', '"', 'Ï'),
    ('D', '-', 'Ð'),
    ('N', '~', 'Ñ'),
    ('O', '`', 'Ò'),
    ('O', '\'', 'Ó'),
    ('O', '^', 'Ô'),
    ('O', '~', 'Õ'),
    ('O', '"', 'Ö'),
    ('*', '*', '×'),
    ('O', '/', 'Ø'),
    ('U', '`', 'Ù'),
    ('U', '\'', 'Ú'),
    ('U', '^', 'Û'),
    ('U', '"', 'Ü'),
    ('Y', '\'', 'Ý'),
    ('P', '|', 'Þ'),
    ('s', 's', 'ß'),
    ('a', '`', 'à'),
    ('a', '\'', 'á'),
    ('a', '^', 'â'),
    ('a', '~', 'ã'),
    ('a', '"', 'ä'),
    ('a', '*', 'å'),
    ('a', 'e', 'æ'),
    ('c', ',', 'ç'),
    ('e', '`', 'è'),
    ('e', '\'', 'é'),
    ('e', '^', 'ê'),
    ('e', '"', 'ë'),
    ('i', '`', 'ì'),
    ('i', '\'', 'í'),
    ('i', '^', 'î'),
    ('i', '"', 'ï'),
    ('d', '-', 'ð'),
    ('n', '~', 'ñ'),
    ('o', '`', 'ò'),
    ('o', '\'', 'ó'),
    ('o', '^', 'ô'),
    ('o', '~', 'õ'),
    ('o', '"', 'ö'),
    ('-', ':', '÷'),
    ('o', '/', 'ø'),
    ('u', '`', 'ù'),
    ('u', '\'', 'ú'),
    ('u', '^', 'û'),
    ('u', '"', 'ü'),
    ('y', '\'', 'ý'),
    ('p', '|', 'þ'),
    ('y', '"', 'ÿ'),
];

/// The character xfig composes from `first` and `second`.
pub fn compose(first: char, second: char) -> Option<char> {
    SEQUENCES
        .iter()
        .find(|(a, b, _)| *a == first && *b == second)
        .map(|(_, _, c)| *c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequences() {
        assert_eq!(compose('a', '"'), Some('ä'));
        assert_eq!(compose('s', 's'), Some('ß'));
        assert_eq!(compose('o', 'e'), Some('œ'));
        assert_eq!(compose('1', '2'), Some('½'));
        assert_eq!(compose('C', 'O'), Some('©'));
        assert_eq!(compose('x', 'y'), None);
        assert_eq!(SEQUENCES.len(), 97);
    }
}
