#!/usr/bin/env python3
"""Static storage-format diff between two git refs of this repo.

Used by atomica/docs/decisions/db-migration-assessment.md. It needs only git and
python3, so it runs where aptos-node cannot build. It compares:
  1. Column family name constants (storage/aptosdb/src/schema/mod.rs).
  2. Column family lists per DB (storage/aptosdb/src/db_options.rs).
  3. DbMetadataKey variant order (BCS tag stability).
  4. Per-schema codec files with license headers and comments stripped.
  5. Variant order of persisted enums in aptos-types and sibling crates.

Usage: storage_schema_diff.py [--strict] <old_ref> <new_ref>
It always prints the findings. With --strict the exit code is 1 when a breaking
difference (reorder, removed CF, renamed CF) is found.
"""
import re
import subprocess
import sys

ENUMS = {
    "types/src/transaction/mod.rs": [
        "Transaction", "TransactionPayload", "TransactionStatus", "ExecutionStatus",
        "TransactionAuxiliaryData", "TransactionInfo", "ReplayProtector",
    ],
    "types/src/state_store/state_key/inner.rs": ["StateKeyInner"],
    "types/src/write_set.rs": ["BaseStateOp"],
    "types/src/state_store/state_value.rs": ["PersistedStateValueMetadata"],
    "types/src/contract_event.rs": ["ContractEvent"],
    "types/src/ledger_info.rs": ["LedgerInfoWithSignatures"],
    "types/src/validator_txn.rs": ["ValidatorTransaction"],
    "types/src/transaction/authenticator.rs": ["TransactionAuthenticator", "AccountAuthenticator"],
    "types/src/on_chain_config/aptos_features.rs": ["FeatureFlag"],
}


def show(ref, path):
    r = subprocess.run(["git", "show", f"{ref}:{path}"], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def strip(src):
    src = re.sub(r"//[^\n]*", "", src)
    src = re.sub(r"/\*.*?\*/", "", src, flags=re.S)
    return re.sub(r"\s+", " ", src).strip()


def ls_tree(ref, path):
    r = subprocess.run(["git", "ls-tree", "-r", "--name-only", ref, path], capture_output=True, text=True)
    return [l for l in r.stdout.splitlines() if l]


def enum_variants(src, name):
    m = re.search(r"pub(?:\(crate\))? enum " + name + r"\b[^{]*\{", src)
    if not m:
        return None
    i, depth, body = m.end(), 1, ""
    while depth and i < len(src):
        c = src[i]
        depth += (c == "{") - (c == "}")
        if depth:
            body += c
        i += 1
    body = strip(body)
    out, depth, cur = [], 0, ""
    for c in body:
        if c in "({[":
            depth += 1
        elif c in ")}]":
            depth -= 1
        if c == "," and depth == 0:
            out.append(cur)
            cur = ""
        else:
            cur += c
    if cur.strip():
        out.append(cur)
    names = []
    for v in out:
        v = re.sub(r"#\[[^\]]*\]", "", v).strip()
        mm = re.match(r"([A-Za-z0-9_]+)", v)
        if mm:
            # Deprecation renames (leading underscore or _DEPRECATED_) keep the tag.
            names.append(re.sub(r"^_(DEPRECATED_)?", "", mm.group(1)))
    return names


def cf_lists(src):
    res = {}
    for m in re.finditer(r"fn (\w+column_families)\(\)[^{]*\{\s*vec!\[(.*?)\]\s*\}", src, re.S):
        res[m.group(1)] = re.findall(r"([A-Z_]+_CF_NAME|DEFAULT_COLUMN_FAMILY_NAME)", m.group(2))
    return res


def main():
    args = [a for a in sys.argv[1:] if a != "--strict"]
    strict = "--strict" in sys.argv[1:]
    old, new = args[0], args[1]
    breaking = 0

    print("== 1. CF name constants")
    pat = r'pub const (\w+_CF_NAME): ColumnFamilyName =\s*"([^"]+)"'
    a = dict(re.findall(pat, show(old, "storage/aptosdb/src/schema/mod.rs")))
    b = dict(re.findall(pat, show(new, "storage/aptosdb/src/schema/mod.rs")))
    for k in sorted(set(a) | set(b)):
        if k not in b:
            print(f"  removed in new : {k} = {a[k]}")
        elif k not in a:
            print(f"  added in new   : {k} = {b[k]}")
        elif a[k] != b[k]:
            print(f"  RENAMED        : {k} {a[k]} -> {b[k]}")
            breaking += 1

    print("== 2. CF lists per DB (db_options.rs)")
    la = cf_lists(show(old, "storage/aptosdb/src/db_options.rs"))
    lb = cf_lists(show(new, "storage/aptosdb/src/db_options.rs"))
    for fn in sorted(set(la) | set(lb)):
        if fn not in lb:
            print(f"  fn removed in new: {fn} ({len(la[fn])} CFs)")
        elif fn not in la:
            print(f"  fn added in new  : {fn} -> {lb[fn]}")
        elif la[fn] == lb[fn]:
            print(f"  same             : {fn}")
        else:
            miss = [c for c in la[fn] if c not in lb[fn]]
            add = [c for c in lb[fn] if c not in la[fn]]
            print(f"  CHANGED          : {fn} removed={miss} added={add}")
            if miss:
                print("    note: on-disk CFs absent from the new open list make RocksDB open fail")
                breaking += 1

    print("== 3. DbMetadataKey variant order")
    ka = enum_variants(show(old, "storage/aptosdb/src/schema/db_metadata/mod.rs"), "DbMetadataKey")
    kb = enum_variants(show(new, "storage/aptosdb/src/schema/db_metadata/mod.rs"), "DbMetadataKey")
    ok = kb[: len(ka)] == ka
    print(f"  old={len(ka)} variants, new={len(kb)}; old is prefix of new: {ok}")
    print(f"  appended: {kb[len(ka):]}")
    if not ok:
        breaking += 1

    print("== 4. Schema codec files (comments stripped)")
    base = "storage/aptosdb/src/schema/"
    files_a = {f for f in ls_tree(old, base) if f.endswith("/mod.rs")}
    files_b = {f for f in ls_tree(new, base) if f.endswith("/mod.rs")}
    for f in sorted(files_a | files_b):
        n = f[len(base):-len("/mod.rs")] if f != base + "mod.rs" else "(root)"
        if f not in files_b:
            print(f"  removed schema   : {n}")
        elif f not in files_a:
            print(f"  added schema     : {n}")
        elif strip(show(old, f)) != strip(show(new, f)):
            print(f"  CODE CHANGED     : {n}")
    unchanged = sum(1 for f in files_a & files_b if strip(show(old, f)) == strip(show(new, f)))
    print(f"  {unchanged} schema files have identical code in both refs")

    print("== 5. Persisted enum variant order (BCS tags)")
    for path, names in ENUMS.items():
        sa, sb = show(old, path), show(new, path)
        if sa is None or sb is None:
            print(f"  {path}: missing in one ref")
            continue
        for name in names:
            va, vb = enum_variants(sa, name), enum_variants(sb, name)
            if va is None or vb is None:
                print(f"  {name}: not found ({'old' if va is None else 'new'})")
            elif vb[: len(va)] == va:
                tail = f" appended {vb[len(va):]}" if len(vb) > len(va) else ""
                print(f"  ok  {name}: {len(va)} -> {len(vb)}{tail}")
            else:
                print(f"  BREAKING {name}: old={va} new={vb}")
                breaking += 1

    print(f"== breaking findings: {breaking}")
    sys.exit(1 if (breaking and strict) else 0)


main()
