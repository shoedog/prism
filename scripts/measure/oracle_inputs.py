"""Pre-launch pins and the exact bytes authorized for the guarded oracle."""

from __future__ import annotations

import hashlib
import os
from pathlib import Path
import stat

from .edges import canonical
from .inputs import bind_tree, json_object, record_header, sha256_file
from .receipt import is_digest

KIT = Path(__file__).resolve().parent / "oracle"
QUALIFICATIONS = KIT / "qualifications"
CHECKER = "3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675"
TREE = "7e02162c902e5c29ec19fefc573ad55b8ed15fcbda940e737f297c53e9c41f54"
OPTIONS = dict(target="ES2022", module="CommonJS", moduleResolution="Node10", jsx="Preserve",
               allowJs=True, checkJs=True, noEmit=True, skipLibCheck=True, strict=True,
               noLib=True, types=[])
RESULTS = ("CORRECT", "WRONG", "UNDECIDED", "INADMISSIBLE")
FORMS = ("named-function", "named-method", "decorated-method")


class OracleError(ValueError):
    def __init__(self, code, reason):
        super().__init__(reason)
        self.code = code


def digest(data):
    return hashlib.sha256(data).hexdigest()


def safe_path(name):
    return (isinstance(name, str) and bool(name) and "\\" not in name
            and "\0" not in name and all(part not in ("", ".", "..", ".git") for part in name.split("/")))


def name(value):
    return isinstance(value, str) and bool(value.strip()) and "\0" not in value


def shape(value, required, optional=()):
    return isinstance(value, dict) and set(required) <= value.keys() <= set(required) | set(optional)


def regular(path):
    if not stat.S_ISREG(path.lstat().st_mode):
        raise ValueError("expected a regular file")
    return path.read_bytes()


def file_tree(root):
    result = {}
    if not stat.S_ISDIR(root.lstat().st_mode):
        raise ValueError("closure root must be a regular directory")
    for base, directories, files in os.walk(root, followlinks=False):
        directory = Path(base)
        for entry in directories:
            if not stat.S_ISDIR((directory / entry).lstat().st_mode):
                raise ValueError("linked or special directory in closure")
        for entry in files:
            path = directory / entry
            relative = path.relative_to(root).as_posix()
            if not safe_path(relative):
                raise ValueError("unsupported closure path")
            result[relative] = regular(path)
    return result


def framed(files):
    result = hashlib.sha256()
    for key in sorted(files, key=lambda item: item.encode("utf-8")):
        data = files[key]
        record_header(result, b"F", key.encode("utf-8"), len(data))
        result.update(data)
    return result.hexdigest()


def checker_path(explicit=None):
    if explicit is not None:
        return explicit.absolute()
    if os.environ.get("PRISM_TYPESCRIPT"):
        return Path(os.environ["PRISM_TYPESCRIPT"]).absolute()
    data = Path(os.environ.get("XDG_DATA_HOME") or Path.home() / ".local/share")
    root = Path(os.environ.get("PRISM_GATE_INPUTS_ROOT") or data / "prism/gate-inputs")
    return (root / "typescript" / TREE / "lib/typescript.js").absolute()


def checker_files(path):
    try:
        if path.name != "typescript.js" or path.parent.name != "lib":
            raise ValueError("expected lib/typescript.js")
        if not stat.S_ISREG(path.lstat().st_mode) or sha256_file(path) != CHECKER:
            raise ValueError("typescript.js digest differs")
        root = path.parent.parent.resolve(strict=True)
        if path != root / "lib/typescript.js":
            raise ValueError("checker path traverses an alias")
        files = file_tree(root)
        if any(not key.isascii() or any(not 32 <= ord(ch) <= 126 for ch in key) for key in files):
            raise ValueError("unsupported checker path")
        hashes = {key: digest(value) for key, value in files.items()}
        if digest(canonical(sorted([k, v] for k, v in hashes.items()))) != TREE:
            raise ValueError("TypeScript tree digest differs")
        return dict(root=str(root), files=hashes, entry="lib/typescript.js")
    except (ValueError, OSError) as error:
        raise OracleError(3, f"checker-pin: {error}") from error


def cache_path(key):
    base = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache")
    directory = Path(os.environ.get("PRISM_MEASURE_SELFCHECK_DIR") or
                     base / "prism/measure/selfcheck").resolve()
    return directory / f"selfcheck-{key}.json"


def fixture_set(root, raw, prefix):
    from .oracle_selection import edge, representable, site, validate_change

    cases = {}
    for relative, data in raw.items():
        parts = relative.split("/")
        if len(parts) < 2:
            raise ValueError("fixture file must be inside a case directory")
        case, member = relative.split("/", 1)
        entry = cases.setdefault(case, dict(id=f"{prefix}/{case}", root=str(root / case),
                                            files={}, prefix=prefix))
        entry["files"][member] = digest(data)
    for entry in cases.values():
        if not {"input.jsonl", "expected.jsonl"} <= entry["files"].keys():
            raise ValueError("self-check case requires input and expected JSONL")
        for filename in ("input.jsonl", "expected.jsonl"):
            for line in raw[f"{entry['id'].split('/', 1)[1]}/{filename}"].split(b"\n"):
                if not line.strip(b" \t\r"):
                    continue
                record = representable(json_object(line))
                if filename != "input.jsonl":
                    continue
                row = record
                if "from" not in record and "record_kind" not in record:
                    if not shape(record, ("relation", "arm", "row"), ("target_index", "change")):
                        raise ValueError("selfcheck-schema: invalid input wrapper")
                    row = record["row"]
                    if record.get("change") is not None:
                        validate_change(record["change"])
                if not isinstance(row, dict):
                    raise ValueError("selfcheck-schema: expected wire object")
                if row.get("record_kind") == "call_site":
                    site(row)
                elif "record_kind" not in row:
                    edge(row)
                else:
                    raise ValueError("selfcheck-schema: expected call_site or edge")
        if not (Path(entry["root"]) / "repo").is_dir():
            raise ValueError("self-check case requires repo directory")
    if not cases:
        raise ValueError("self-check has no cases")
    return [cases[key] for key in sorted(cases)]


EVIDENCE = ("binary_sha256", "form", "controls", "label", "raw", "source_tree_sha256",
            "source_state_sha256", "source_file_sha256", "receipt_sha256", "sites_sha256")


def labelled(value, run_count=None):
    """The closed label of a qualification control; `run_count` bounds `run_index` when known."""
    from .oracle_selection import number, require

    require(shape(value, ("run_index", "record_ordinal", "target_index", "form",
                          "declaration", "contradiction_line"))
            and all(number(value[key]) for key in ("run_index", "record_ordinal", "target_index"))
            and (run_count is None or value["run_index"] < run_count) and value["form"] in FORMS,
            "qualification-label: invalid index or form")
    point = value["declaration"]
    require(shape(point, ("file", "name", "start_byte", "end_byte", "line"))
            and safe_path(point["file"]) and name(point["name"])
            and number(point["start_byte"]) and number(point["end_byte"], point["start_byte"] + 1)
            and number(point["line"], 1) and number(value["contradiction_line"], 1)
            and point["line"] != value["contradiction_line"], "qualification-label: invalid source label")
    return value


def provenance(record, form):
    """Evidence must be what the builder writes: labelled source, indexed capture and control results."""
    from .oracle_selection import require, site

    try:
        require(shape(record, EVIDENCE)
                and all(is_digest(record[key]) for key in EVIDENCE if key.endswith("_sha256")),
                "closed evidence fields")
        label = labelled(record["label"])
        point = label["declaration"]
        require(label["form"] == form, "label form")
        raw = site(record["raw"])
        require(raw["record_kind"] == "call_site" and label["target_index"] < len(raw["resolved_targets"]),
                "indexed capture")
        target = raw["resolved_targets"][label["target_index"]]
        require(target["confidence"] == "exact" and target["function_id"]["file"] == point["file"]
                and target["function_id"]["name"] == point["name"]
                and target["function_id"]["start_line"] == point["line"], "capture disagrees with label")
    except (ValueError, KeyError, TypeError) as error:
        raise ValueError(f"qualification evidence lacks builder provenance: {error}") from error


def qualifications(directory):
    if not directory.exists() and not directory.is_symlink():
        return {}, None, {}
    files = file_tree(directory)
    bundle = json_object(files["bundle.json"])
    if (not shape(bundle, ("schema", "builds")) or bundle["schema"] != "prism.measure.qualification/1"
            or not isinstance(bundle["builds"], dict)):
        raise ValueError("invalid qualification bundle")
    builds, used = {}, {"bundle.json"}
    for binary, forms in bundle["builds"].items():
        if not is_digest(binary) or not isinstance(forms, dict) or not forms.keys() <= set(FORMS):
            raise ValueError("invalid qualification build or form")
        builds[binary] = []
        for form, evidence in forms.items():
            if (not shape(evidence, ("path", "sha256")) or not safe_path(evidence["path"])
                    or evidence["path"] not in files or digest(files[evidence["path"]]) != evidence["sha256"]):
                raise ValueError("unbound qualification evidence")
            record = json_object(files[evidence["path"]])
            if (record.get("binary_sha256") != binary or record.get("form") != form
                    or record.get("controls") != {"match": "CORRECT", "contradiction": "WRONG"}):
                raise ValueError("qualification evidence does not establish the form")
            provenance(record, form)
            used.add(evidence["path"])
            builds[binary].append(form)
    if files.keys() != used:
        raise ValueError("unlisted qualification evidence")
    return builds, digest(files["bundle.json"]), files


def prepare_inputs(typescript=None, question=None, *, qualification_dir=None):
    checker = checker_files(checker_path(typescript))
    try:
        question_files = {path.name: regular(path) for path in KIT.glob("*.cjs")}
        # The worker executes exactly these verified bytes from a staged copy.
        kit = dict(question_files)
        question_files.update({path.name: regular(path) for path in KIT.parent.glob("oracle*.py")})
        built_ins = {"questions/" + key: value for key, value in file_tree(KIT / "questions").items()}
        question_files.update(built_ins)
        kit.update(built_ins)
        qdir = QUALIFICATIONS if qualification_dir is None else qualification_dir
        builds, qualification_sha256, evidence = qualifications(qdir)
        question_files.update({"qualifications/" + k: v for k, v in evidence.items()})
        fixtures_root = KIT / "selfcheck"
        fixtures = file_tree(fixtures_root)
        cases = fixture_set(fixtures_root, fixtures, "kit")
        question_files = {"kit/" + k: v for k, v in question_files.items()}
        fixtures = {"kit/" + k: v for k, v in fixtures.items()}
        module = None
        if question is not None:
            absolute = question.absolute()
            if absolute != absolute.resolve(strict=True) or absolute.suffix not in (".cjs", ".js"):
                raise ValueError("question must be a regular CommonJS entry")
            root = absolute.parent
            raw = file_tree(root)
            data = json_object(raw["question-data.json"]) if "question-data.json" in raw else {}
            for key, value in data.items():
                if (not safe_path(key) or key not in raw or key == absolute.name
                        or not is_digest(value) or digest(raw[key]) != value):
                    raise ValueError("unbound question data")
            module = dict(root=str(root), entry=absolute.name,
                          files={k: digest(v) for k, v in raw.items()}, data=data)
            lane_fixtures = {k[len("selfcheck/"):]: v for k, v in raw.items()
                             if k.startswith("selfcheck/")}
            cases += fixture_set(root / "selfcheck", lane_fixtures, "lane")
            question_files.update({"lane/" + k: v for k, v in raw.items()})
            fixtures.update({"lane/" + k: v for k, v in lane_fixtures.items()})
            # The closure is the whole directory, so the key also names which entry graded.
            question_files["lane-entry"] = absolute.name.encode("utf-8")
        components = dict(checker_tree_sha256=TREE, compiler_options=OPTIONS,
                          default_lib_options=dict(OPTIONS, noLib=False),
                          question_sha256=framed(question_files), fixtures_sha256=framed(fixtures))
        key = digest(canonical(components))
        if question is not None and cache_path(key).is_relative_to(question.parent.resolve()):
            raise ValueError("self-check cache must be outside question directory")
        return dict(checker=checker, components=components, key=key, cases=cases, module=module,
                    builds=builds, qualification_sha256=qualification_sha256, kit=kit)
    except (ValueError, OSError, KeyError) as error:
        raise OracleError(3, f"selfcheck-binding: {error}") from error


def cached(request):
    try:
        value = json_object(regular(cache_path(request["key"])))
        if (not shape(value, ("schema", "key", "status", "components", "cases", "expected", "agreed",
                              "failures", "runtime"))
                or value["schema"] != "prism.measure.selfcheck/1" or value["status"] != "ok"
                or value["key"] != request["key"] or value["components"] != request["components"]
                or value["failures"] != [] or type(value["cases"]) is not int
                or value["cases"] != len(request["cases"]) or type(value["expected"]) is not int
                or value["expected"] < 1 or type(value["agreed"]) is not int
                or value["agreed"] != value["expected"]
                or not shape(value["runtime"], ("node", "python"))
                or not all(name(part) for part in value["runtime"].values())):
            raise ValueError("invalid receipt")
        return value
    except (ValueError, OSError) as error:
        raise OracleError(3, "selfcheck-receipt: missing or stale for current inputs") from error


def source(run, *, recheck=False):
    root = Path(run.receipt["corpus"]["root"])
    if not root.is_absolute() or not root.is_dir() or root.resolve() != root:
        raise OracleError(1 if recheck else 2, "source-root: recorded snapshot unavailable")
    files = {} if not recheck else None
    try:
        binding = bind_tree(root, file_digests=files)
    except (OSError, ValueError) as error:
        raise OracleError(1 if recheck else 2, f"source-root: {error}") from error
    state = binding.sha256, binding.state_sha256
    matches = state == (run.receipt["corpus"]["input_sha256"],
                        run.receipt["corpus"]["input_state_sha256"])
    entry = dict(root=str(root), files=files, matches=matches,
                 binary=run.receipt["binary"]["sha256"])
    return entry, state
