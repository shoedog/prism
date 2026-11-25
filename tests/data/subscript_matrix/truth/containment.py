"""Token checks for trusted, small fixture programs."""

import ast
import io
import re
import tokenize
import unicodedata


class Refused(ValueError):
    pass


PYTHON_NAMES = {
    "open", "exec", "eval", "compile", "__import__", "__builtins__", "breakpoint", "input",
}
JS_NAMES = {"process", "globalThis", "fetch", "eval", "Function"}
GO_IMPORTS = {"fmt", "reflect", "strings"}


def python_names(source):
    has_interpolation = False
    try:
        for token in tokenize.generate_tokens(io.StringIO(source).readline):
            if token.type == tokenize.NAME:
                yield unicodedata.normalize("NFKC", token.string)
            if token.type == tokenize.STRING and re.match(r"(?i)[rf]*f[rf]*['\"]", token.string):
                has_interpolation = "{" in token.string or has_interpolation
    except (SyntaxError, tokenize.TokenError):
        # Let the interpreter classify malformed programs after checking their tokens.
        pass
    if has_interpolation:
        # Python 3.9 treats an entire f-string as one STRING token.
        tree = ast.parse(source)
        for node in ast.walk(tree):
            for field in ("id", "attr", "arg"):
                name = getattr(node, field, None)
                if isinstance(name, str):
                    yield name


BACKTICK = chr(96)
JS_TOKEN = re.compile(
    r"(?P<space>[\s\ufeff]+)|(?P<comment>//[^\r\n\u2028\u2029]*|/\*[\s\S]*?\*/)"
    r"|(?P<string>\"(?:\\[\s\S]|[^\"\\\r\n])*\"|'(?:\\[\s\S]|[^'\\\r\n])*')"
    r"|(?P<word>(?:[$\w\u200c\u200d]|\\u(?:[0-9a-fA-F]{4}|\{[0-9a-fA-F]+\}))+)"
    r"|(?P<punct>\?\.|=>|\+\+|--|[\s\S])"
)
JS_ESCAPE = re.compile(r"\\u(?:([0-9a-fA-F]{4})|\{([0-9a-fA-F]+)\})")
JS_REGEX = re.compile(r"/(?:\\[^\r\n]|\[(?:\\[^\r\n]|[^\]\\\r\n])*\]|[^/[\\\r\n])+/[a-z]*")
JS_REGEX_PREFIX = {
    "", "(", "{", "[", "=", "!", "~", ",", ";", ":", "?", "=>", "&", "|",
    "+", "-", "*", "%", "^", "<", ">", "return", "throw", "case", "yield", "void",
    "typeof", "delete", "in", "instanceof", "else", "do",
}


def js_tokens(source):
    """Skip literal text, but scan JavaScript expressions inside templates."""
    templates = []
    parens = []
    previous = ""
    index = 0
    while index < len(source):
        char = source[index]
        if templates and templates[-1] is None:
            if char == "\\":
                index += 2
                continue
            if source.startswith("$" + "{", index):
                templates[-1] = 0
                previous = ""
                index += 2
                continue
            if char == BACKTICK:
                templates.pop()
                previous = "literal"
            index += 1
            continue
        if char == BACKTICK:
            templates.append(None)
            index += 1
            continue
        token = JS_TOKEN.match(source, index)
        index = token.end()
        kind = token.lastgroup
        word = token.group()
        if kind in {"space", "comment"}:
            continue
        if kind == "string":
            previous = "literal"
            yield previous
            continue
        if word == "/" and previous in JS_REGEX_PREFIX:
            literal = JS_REGEX.match(source, token.start())
            if literal:
                index = literal.end()
                previous = "literal"
                yield previous
                continue
        if kind == "word":
            word = JS_ESCAPE.sub(lambda m: chr(int(m[1] or m[2], 16)), word)
        if templates and word == "{":
            templates[-1] += 1
        elif templates and word == "}":
            if templates[-1] == 0:
                templates[-1] = None
                continue
            templates[-1] -= 1
        if word == "(":
            parens.append(previous in {"if", "while", "for", "with", "switch", "catch"})
        follows_condition = word == ")" and parens and parens.pop()
        if follows_condition:
            previous = ""
        elif previous in {".", "?."} and kind == "word":
            previous = "property"
        else:
            previous = word
        yield word


GO_TOKEN = re.compile(
    r"(?P<skip>\s+|//[^\r\n]*|/\*[\s\S]*?\*/)"
    r"|(?P<string>\"(?:\\[\s\S]|[^\"\\])*\"|" + BACKTICK + "[^" + BACKTICK + "]*"
    + BACKTICK + r")|(?P<rune>'(?:\\[\s\S]|[^'\\])*')"
    r"|(?P<word>[^\W\d]\w*)|(?P<punct>[\s\S])"
)


def go_tokens(source):
    return [(token.lastgroup, token.group()) for token in GO_TOKEN.finditer(source)
            if token.lastgroup != "skip"]


def go_import_paths(tokens):
    for index, token in enumerate(tokens):
        if token != ("word", "import"):
            continue
        index += 1
        grouped = tokens[index] == ("punct", "(")
        index += grouped
        while not (grouped and tokens[index] == ("punct", ")")):
            kind, word = tokens[index]
            if kind == "string":
                yield word[1:-1] if word.startswith(BACKTICK) else ast.literal_eval(word)
                if not grouped:
                    break
            elif kind != "word" and word not in {".", ";"}:
                raise ValueError("malformed import")
            index += 1


def check_source(label, lang, source, prelude=False):
    try:
        if lang == "py":
            names = set(python_names(source))
            imports = {"import"} & names
            forbidden = PYTHON_NAMES & names
        elif lang in {"js", "ts"}:
            names = list(js_tokens(source))
            imports = {"import"} & set(names)
            for index, name in enumerate(names):
                if name == "require" and (
                    names[index + 1:index + 2] == ["("]
                    or names[index + 1:index + 3] == ["?.", "("]
                ):
                    imports.add("require")
            forbidden = JS_NAMES & set(names)
        elif lang == "go":
            tokens = go_tokens(source)
            imports = set(go_import_paths(tokens)) if prelude else (
                {"import"} if ("word", "import") in tokens else set()
            )
            forbidden = set()
        else:
            return
    except (IndexError, SyntaxError, ValueError) as error:
        raise Refused(f"{label}: cannot check containment tokens ({error})") from error
    if prelude:
        expected = GO_IMPORTS if lang == "go" else set()
        if imports != expected:
            raise Refused(f"{label}: prelude import set {sorted(imports)} must be {sorted(expected)}")
    elif imports:
        raise Refused(f"{label}: case imports are forbidden ({', '.join(sorted(imports))})")
    elif forbidden:
        raise Refused(f"{label}: forbidden identifier(s) {', '.join(sorted(forbidden))}")
