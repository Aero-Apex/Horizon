"""Extract EMF's OptiFine part-name tables (EMFModelMappings.java) for Minecraft 26.3.

EMF is built with a Java preprocessor: `//#if MC >= 26.1` ... `//$$ code` ... `//#endif`.
This keeps the branches that are active for 26.3 and un-comments their `//$$` lines, then reads
every `OptifineMapper.models("a", "b").parts(EXPR)` into {model: {optifine part: vanilla part}}.

usage: python extract_cem_parts.py EMFModelMappings.java out.json
"""
import json
import re
import sys

MC = (26, 3, 0)


def version(text):
    text = text.strip()
    if "." in text:
        parts = [int(p) for p in text.split(".")]
        while len(parts) < 3:
            parts.append(0)
        return tuple(parts)
    n = int(text)
    # 12102 -> 1.21.2, 12111 -> 1.21.11, 12006 -> 1.20.6
    return (n // 10000, (n // 100) % 100, n % 100)


def holds(cond):
    m = re.fullmatch(r"MC\s*(>=|<=|>|<|==)\s*([0-9.]+)", cond.strip())
    if not m:
        raise ValueError(f"cannot read condition {cond!r}")
    op, v = m.group(1), version(m.group(2))
    return {
        ">=": MC >= v,
        "<=": MC <= v,
        ">": MC > v,
        "<": MC < v,
        "==": MC == v,
    }[op]


def preprocess(src):
    out = []
    # stack of (this branch active, some earlier branch taken, parent active)
    stack = []
    active = True
    for line in src.splitlines():
        s = line.strip()
        if s.startswith("//#if "):
            parent = active
            cond = holds(s[6:])
            stack.append((cond, cond, parent))
            active = parent and cond
            continue
        if s.startswith("//#elseif "):
            _, taken, parent = stack.pop()
            cond = (not taken) and holds(s[10:])
            stack.append((cond, taken or cond, parent))
            active = parent and cond
            continue
        if s == "//#else":
            _, taken, parent = stack.pop()
            cond = not taken
            stack.append((cond, True, parent))
            active = parent and cond
            continue
        if s == "//#endif":
            _, _, parent = stack.pop()
            active = parent
            continue
        if not active:
            continue
        if s.startswith("//$$"):
            line = line.replace("//$$", "", 1)
        out.append(line)
    assert not stack, "unbalanced preprocessor"
    return "\n".join(out)


def strip_comments(src):
    return re.sub(r"//[^\n]*", "", src)


def balanced(text, start):
    """Index just past the parenthesis group that opens at text[start] == '('."""
    depth = 0
    in_str = False
    i = start
    while i < len(text):
        c = text[i]
        if in_str:
            if c == "\\":
                i += 1
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    raise ValueError("unbalanced")


def strings(text):
    return re.findall(r'"([^"]*)"', text)


def entries(text):
    """partMapping("a") / partMapping("a", "b") calls inside text."""
    out = {}
    for m in re.finditer(r"partMapping\s*\(", text):
        end = balanced(text, m.end() - 1)
        args = strings(text[m.end() : end - 1])
        if len(args) == 1:
            out[args[0]] = args[0]
        elif len(args) == 2:
            out[args[0]] = args[1]
    return out


def evaluate(expr, variables):
    expr = expr.strip()
    if expr.startswith("Map.ofEntries"):
        return entries(expr)
    m = re.match(r"new\s+HashMap\s*<>\s*\(", expr)
    if m:
        end = balanced(expr, m.end() - 1)
        base = evaluate(expr[m.end() : end - 1], variables)
        result = dict(base)
        rest = expr[end:]
        for p in re.finditer(r"putAll\s*\(", rest):
            e = balanced(rest, p.end() - 1)
            result.update(evaluate(rest[p.end() : e - 1], variables))
        return result
    if re.fullmatch(r"[A-Za-z_]\w*", expr):
        return dict(variables[expr])
    raise ValueError(f"cannot evaluate {expr[:60]!r}")


def main(path, out_path):
    src = strip_comments(preprocess(open(path, encoding="utf-8").read()))
    variables = {}
    for m in re.finditer(r"\b(?:var|Map<String,\s*String>)\s+(\w+)\s*=\s*", src):
        # the statement runs to the ';' that closes it at depth 0
        i, depth = m.end(), 0
        while True:
            c = src[i]
            if c in "({":
                depth += 1
            elif c in ")}":
                depth -= 1
            elif c == ";" and depth == 0:
                break
            i += 1
        try:
            variables[m.group(1)] = evaluate(src[m.end() : i], variables)
        except ValueError:
            pass
    models = {}
    for m in re.finditer(r"OptifineMapper\s*\.models\s*\(", src):
        end = balanced(src, m.end() - 1)
        names = strings(src[m.end() : end - 1])
        rest = src[end:]
        p = re.match(r"\s*\.parts\s*\(", rest)
        if not p:
            continue
        pend = balanced(rest, p.end() - 1)
        mapping = evaluate(rest[p.end() : pend - 1], variables)
        for name in names:
            models[name] = mapping
    json.dump(models, open(out_path, "w"), indent=1, sort_keys=True)
    print(f"{len(models)} models, {len(variables)} variables")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
