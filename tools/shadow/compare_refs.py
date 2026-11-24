"""Project AD4 from original legacy facts. Only salted record names come from the id map."""

from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
import re


class Invalid(ValueError):
    """Messages are public. Keep source text and child diagnostics out of them."""


@dataclass(eq=False)
class Fact:
    destination: Path
    old_id: str
    kind: str
    key: str | None
    payload: dict | None  # None for omitted occurrences, still members of the group.
    observer: str
    time: float
    coordinate: tuple
    record: str | None = None

    @property
    def thing(self):
        return self.record if self.key is None else f"{self.kind}:{self.key}"


# Unicode White_Space, as used by the writer. Python str.strip() additionally
# strips four ASCII information separators that can be part of an address.
WHITESPACE = (" \t\n\v\f\r\x85\xa0\u1680" + "".join(map(chr, range(0x2000, 0x200b)))
              + "\u2028\u2029\u202f\u205f\u3000")


def parts(raw, reviewed=False):
    if isinstance(raw, str):
        raw = [raw]
    elif not reviewed or not isinstance(raw, list) or not all(isinstance(s, str) for s in raw):
        return None
    return [part for s in raw for item in s.split(",") if (part := item.strip(WHITESPACE))]


def reference_fields(payload, reviewed):
    for name, raw in payload.items():
        if name == "ref" or (len(name) > 4 and name.endswith("_ref")) or name in reviewed:
            yield name, parts(raw, name in reviewed)


def components(edges):
    """Iterative Kosaraju traversal, including chains longer than the Python stack."""
    reverse = {node: [] for node in edges}
    for node, successors in edges.items():
        for successor in successors:
            reverse[successor].append(node)
    seen = set()
    finished = []
    for start in edges:
        stack = [(start, False)]
        while stack:
            node, leaving = stack.pop()
            if leaving:
                finished.append(node)
            elif node not in seen:
                seen.add(node)
                stack.append((node, True))
                stack.extend((child, False) for child in edges[node])
    result = {}
    for start in reversed(finished):
        stack = [start]
        while stack:
            node = stack.pop()
            if node not in result:
                result[node] = start
                stack.extend(reverse[node])
    return result


class References:
    def __init__(self, cuts):
        self.copies = defaultdict(dict)
        for cut in cuts:
            for row in cut.rows.values():
                self.copies[row.old_id][row.destination] = row
        edges = {}
        for cut in cuts:
            for row in cut.rows.values():
                if row.key is None and row.payload is not None:
                    edges[row] = {target for _, items in reference_fields(row.payload, cut.reviewed)
                                  for part in items or () for target in self.candidates(row, part)
                                  if target.key is None and target is not row}
        self.component = components(edges)

    def candidates(self, row, part):
        copies = self.copies.get(part, {})
        local = copies.get(row.destination)
        if local is not None and local.payload is not None:
            return [local]
        return [r for r in copies.values() if r.payload is not None]

    def resolve(self, row, part, address):
        """None is unresolved. A direct root self-hit also works without an id map."""
        if part not in self.copies:
            match = re.fullmatch(r"([a-z][a-z0-9-]*)[:/](.+)", part, re.DOTALL) if address else None
            if match and match[1] != "r":
                return f"{match[1]}:{match[2]}"
            return None
        targets = set()
        for target in self.candidates(row, part):
            if row is target:
                return row.thing or row
            if target in self.component and self.component[target] is self.component.get(row):
                return None
            if target.thing is None:
                raise Invalid("a referenced root needs the group id map (use --group)")
            targets.add(target.thing)
        return targets.pop() if len(targets) == 1 else None

    def changes(self, row, payload, reviewed, companions):
        """Use the supplying fact, even when the old reader has overlaid its fields."""
        additions, replacements, removed = set(), {}, set()
        for name, items in reference_fields(payload, reviewed):
            targets = {self.resolve(row, part, name == "ref") for part in items or ()}
            complete = items is not None and None not in targets
            targets.difference_update((None, row, row.thing))
            if name == "ref":
                additions.update(targets)
                if complete:
                    removed.add(name)
            else:
                replacements[name] = targets
                if (name not in reviewed and name in companions and complete
                        and (items or payload[name] == "")):
                    removed.add(name)
        return additions, replacements, removed


def paired(payload):
    return {k for k in payload if k.endswith("_ref") and k[:-4] in payload and k[:-4]}


@dataclass
class Mentions:
    refs: set = field(default_factory=set)
    current: dict = field(default_factory=dict)

    def apply(self, additions, replacements, observer, own):
        self.refs.update(additions)
        self.current.update(replacements)
        about = self.refs.union(*self.current.values())
        if observer:
            about.add("observer:" + observer)
        about.discard(own)
        return about
