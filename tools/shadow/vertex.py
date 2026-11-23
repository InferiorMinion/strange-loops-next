"""Read the single-store subset of a legacy KDL vertex used by shadow export."""

from dataclasses import dataclass
import re


class InvalidVertex(ValueError):
    def __init__(self, line, message):
        super().__init__(f"shadow-map-invalid at line {line}: {message}")


@dataclass(frozen=True)
class Token:
    kind: str
    value: str
    line: int


_TOKEN = re.compile(
    r'(?P<space>[ \t]+)|(?P<end>\r\n|\r|\n|;)|(?P<comment>//[^\r\n]*)'
    r'|(?P<block>/\*)|(?P<disabled>/-)|(?P<open>\{)|(?P<close>\})|(?P<equals>=)'
    r'|(?P<string>"(?:[^"\\\r\n]|\\[^\r\n])*")'
    r'|(?P<word>(?:(?!//|/\*|/-)[^\s{};="])+)'
)


_ESCAPE = re.compile(r'\\(?:u\{([0-9a-fA-F]{1,6})\}|(.))')
_ESCAPES = {'"': '"', '\\': '\\', '/': '/', 'b': '\b', 'f': '\f', 'n': '\n', 'r': '\r', 't': '\t'}


def quoted(text, line):
    def decode(match):
        digits, short = match.groups()
        if digits is None and short in _ESCAPES:
            return _ESCAPES[short]
        if digits is not None:
            scalar = int(digits, 16)
            if scalar <= 0x10FFFF and not 0xD800 <= scalar <= 0xDFFF:
                return chr(scalar)
        raise InvalidVertex(line, "unsupported string escape")
    return _ESCAPE.sub(decode, text[1:-1])


def tokens(text):
    pos, line = 0, 1
    previous = None
    while pos < len(text):
        match = _TOKEN.match(text, pos)
        if not match:
            raise InvalidVertex(line, "unrecognized token or unterminated string")
        pos = match.end()
        kind, value = match.lastgroup, match.group()
        if kind == "block":
            start, depth = line, 1
            for delimiter in re.finditer(r"/\*|\*/|\r\n|\r|\n", text[pos:]):
                part = delimiter.group()
                if part == "/*":
                    depth += 1
                elif part == "*/":
                    depth -= 1
                    if depth == 0:
                        pos += delimiter.end()
                        break
                else:
                    line += 1
            else:
                raise InvalidVertex(start, "unterminated block comment")
        elif kind not in ("space", "comment"):
            if kind in ("word", "string") and previous in ("word", "string"):
                raise InvalidVertex(line, "expected space between tokens")
            if kind == "string":
                value = quoted(value, line)
            yield Token(kind, value, line)
            if kind == "end" and value != ";":
                line += 1
        previous = kind
    yield Token("eof", "", line)


@dataclass
class Node:
    name: str
    line: int
    args: list
    children: list | None


class Parser:
    def __init__(self, text):
        self.tokens = iter(tokens(text))
        self.token = next(self.tokens)

    def take(self):
        old = self.token
        self.token = next(self.tokens)
        return old

    def nodes(self, parent=None):
        nodes = []
        while self.token.kind not in ("close", "eof"):
            if self.token.kind == "end":
                self.take()
                continue
            disabled = self.token.kind == "disabled"
            if disabled:
                self.take()
                while self.token.kind == "end":
                    self.take()
            name = self.token
            if name.kind not in ("word", "string"):
                raise InvalidVertex(name.line, "expected a node name")
            self.take()
            args = []
            while self.token.kind in ("word", "string", "disabled"):
                omit = self.token.kind == "disabled"
                if omit:
                    self.take()
                    if self.token.kind == "open":
                        self.take()
                        self.nodes(name)
                        continue
                if self.token.kind not in ("word", "string"):
                    raise InvalidVertex(self.token.line, "expected an argument or property")
                arg = self.take()
                if self.token.kind == "equals":
                    self.take()
                    if self.token.kind not in ("word", "string"):
                        raise InvalidVertex(arg.line, "expected a property value")
                    self.take()
                    arg = Token("property", arg.value, arg.line)
                if not omit:
                    args.append(arg)
            children = None
            if self.token.kind == "open":
                self.take()
                children = self.nodes(name)
            if self.token.kind not in ("close", "end", "eof"):
                raise InvalidVertex(self.token.line, "expected a node terminator")
            if not disabled:
                nodes.append(Node(name.value, name.line, args, children))
        if parent is None and self.token.kind == "close":
            raise InvalidVertex(self.token.line, "unexpected closing brace")
        if parent is not None:
            if self.token.kind == "eof":
                raise InvalidVertex(parent.line, f"unclosed {parent.value} block")
            self.take()
        return nodes


def block(node):
    if node.args or node.children is None:
        raise InvalidVertex(node.line, f"{node.name} requires a block without arguments")
    return node.children


def unique(nodes, allowed, ignored=()):
    result = {}
    for node in nodes:
        if node.name in ignored:
            continue
        if node.name not in allowed:
            raise InvalidVertex(node.line, f"unsupported {node.name!r}")
        if node.name in result:
            raise InvalidVertex(node.line, f"duplicate {node.name!r}")
        result[node.name] = node
    return result


def required(nodes, name, line):
    if name not in nodes:
        raise InvalidVertex(line, f"missing {name}")
    return nodes[name]


def string_arg(node):
    if (node.children is not None or len(node.args) != 1
            or node.args[0].kind != "string" or not node.args[0].value):
        raise InvalidVertex(node.line, f"{node.name} requires one nonempty quoted string")
    return node.args[0].value


def parse_vertex(text):
    top = unique(Parser(text).nodes(), {"name", "store", "loops"})
    name = string_arg(required(top, "name", 1))
    if name in (".", "..") or any(c in name for c in "/\\\x00"):
        raise InvalidVertex(top["name"].line, "name must be one filename component")
    store = string_arg(required(top, "store", 1))
    if "\x00" in store:
        raise InvalidVertex(top["store"].line, "invalid store path")
    kinds = {}
    for kind in block(required(top, "loops", 1)):
        if kind.name == "boundary":
            continue
        if not kind.name or kind.name in kinds:
            raise InvalidVertex(kind.line, "empty or duplicate kind")
        fields = unique(block(kind), {"fold"}, {"edge", "lifecycle", "search", "preview", "boundary"})
        fold = required(fields, "fold", kind.line)
        items = required(unique(block(fold), {"items"}), "items", fold.line)
        if (items.children is not None or len(items.args) != 2
                or items.args[0].kind != "string"):
            raise InvalidVertex(items.line, "expected items \"by\" field or items \"collect\" limit")
        mode, arg = items.args
        if mode.value == "by" and arg.kind == "string" and arg.value:
            kinds[kind.name] = {"by": arg.value}
        elif (mode.value == "collect" and arg.kind == "word"
              and re.fullmatch(r"0|[1-9][0-9]{0,19}", arg.value)
              and int(arg.value) < 2**64):
            kinds[kind.name] = {"collect": arg.value}
        else:
            raise InvalidVertex(items.line, f"unsupported items fold {mode.value!r} or argument")
    return {"format": "shadow-map-v1", "source": name, "store": store, "kinds": kinds}
