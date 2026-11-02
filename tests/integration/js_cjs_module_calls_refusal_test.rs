//! #598: the refusals of the CommonJS module-call route, each pinned to the outcome the call had
//! before the route existed, and the shapes reduced from real call sites. A regression that adds
//! an edge where main had none fails here.
use super::js_cjs_module_calls_test::{app, case, run_cases, Case, FG, NONE};

#[test]
fn producer_refusals_keep_main() {
    let o = |extra: &str| format!("{FG}module.exports = {{ f, g }};\n{extra}");
    let c = app("const m = require('./m');", "m.f(1)");
    run_cases(vec![
        case(
            "module-exports-reassigned",
            &format!("{FG}module.exports = f;\nmodule.exports = g;\n"),
            &app("const m = require('./m');", "m(1)"),
            "m",
            NONE,
        ),
        case(
            "module-exports-replaced-after-object",
            &o("module.exports = {};\n"),
            &c,
            "f",
            NONE,
        ),
        case(
            "conditional-module-exports",
            &format!("{FG}if (process.env.X) {{ module.exports = f; }}\n"),
            &app("const m = require('./m');", "m(1)"),
            "m",
            NONE,
        ),
        case(
            "conditional-member-export",
            &format!("{FG}if (process.env.X) {{ exports.f = f; }}\n"),
            &c,
            "f",
            NONE,
        ),
        case(
            "same-key-assigned-twice",
            &format!("{FG}exports.f = f;\nexports.f = g;\n"),
            &c,
            "f",
            NONE,
        ),
        case(
            "object-key-mutated-later",
            &o("module.exports.f = g;\n"),
            &c,
            "f",
            NONE,
        ),
        case(
            "exports-alias-written",
            &format!("{FG}exports.f = f;\nconst e = exports;\ne.f = g;\n"),
            &c,
            "f",
            NONE,
        ),
        case(
            "exports-after-module-exports-replaced",
            &format!("{FG}module.exports = {{ g }};\nexports.f = f;\n"),
            &c,
            "f",
            NONE,
        ),
        case(
            "spread-in-export-object",
            &format!("{FG}module.exports = {{ ...other, f }};\n"),
            &c,
            "f",
            NONE,
        ),
        case(
            "computed-key-in-export-object",
            &format!("{FG}module.exports = {{ ['f']: f }};\n"),
            &c,
            "f",
            NONE,
        ),
        case(
            "mixed-esm-and-cjs",
            &format!("{FG}module.exports = f;\nexport function h() {{}}\n"),
            &app("const m = require('./m');", "m(1)"),
            "m",
            NONE,
        ),
        case(
            "default-member-is-not-the-module-value",
            &format!("{FG}exports.default = f;\n"),
            &app("const m = require('./m');", "m(1)"),
            "m",
            NONE,
        ),
        case(
            "esm-default-export",
            "export default function f(x) { return x; }\n",
            &app("const m = require('./m');", "m(1)"),
            "m",
            NONE,
        ),
        case(
            "esm-named-export",
            "export function f(x) { return x; }\n",
            &c,
            "f",
            NONE,
        ),
        case(
            "anonymous-function-module",
            "module.exports = function (x) { return x; };\n",
            &app("const m = require('./m');", "m(1)"),
            "m",
            NONE,
        ),
        case(
            "anonymous-arrow-member",
            "exports.f = (x) => x;\n",
            &c,
            "f",
            NONE,
        ),
        case(
            "anonymous-arrow-member-in-object",
            "module.exports = { f: (x) => x };\n",
            &c,
            "f",
            NONE,
        ),
        case(
            "data-module-value",
            "const f = 5;\nmodule.exports = f;\n",
            &app("const m = require('./m');", "m(1)"),
            "m",
            NONE,
        ),
        case(
            "data-member",
            "const f = 5;\nmodule.exports = { f };\n",
            &c,
            "f",
            NONE,
        ),
        case(
            "class-module-value",
            "class f { }\nmodule.exports = f;\n",
            &app("const m = require('./m');", "m(1)"),
            "m",
            NONE,
        ),
        case(
            "object-module-value-called",
            &o(""),
            &app("const m = require('./m');", "m(1)"),
            "m",
            NONE,
        ),
        case(
            "member-missing",
            &o(""),
            &app("const m = require('./m');", "m.zzz(1)"),
            "zzz",
            NONE,
        ),
        case(
            "function-module-value-static-member",
            &format!("{FG}module.exports = f;\nf.g = g;\n"),
            &app("const m = require('./m');", "m.g(1)"),
            "g",
            NONE,
        ),
        case(
            "duplicate-function-name",
            "function f(x) { return x; }\nfunction f(x) { return 0; }\nmodule.exports = f;\n",
            &app("const m = require('./m');", "m(1)"),
            "m",
            NONE,
        ),
        case(
            "terminal-rebound-before-export",
            &format!("{FG}f = g;\nmodule.exports = f;\n"),
            &app("const m = require('./m');", "m(1)"),
            "m",
            NONE,
        ),
        case(
            "export-of-imported-binding",
            "const f = require('./other').f;\nmodule.exports = { f };\n",
            &c,
            "f",
            NONE,
        ),
        case(
            "nested-declaration-only",
            "function holder() { function f() {} }\nmodule.exports = { f };\n",
            &c,
            "f",
            NONE,
        ),
    ]);
}

#[test]
fn consumer_refusals_keep_main() {
    let module_fn = format!("{FG}module.exports = f;\n");
    let object = format!("{FG}module.exports = {{ f, g }};\n");
    let files = |producer: &str, consumer: &str| {
        vec![
            ("m.js", producer.to_owned()),
            ("app.js", consumer.to_owned()),
        ]
    };
    let mk =
        |id: &'static str, consumer: &str, callee: &'static str, expect: &[&'static str]| Case {
            id,
            files: files(&object, consumer)
                .into_iter()
                .map(|(p, s)| (p, s))
                .collect(),
            caller: "run",
            callee,
            expect: expect.to_vec(),
        };
    let mk_fn = |id: &'static str, consumer: &str, expect: &[&'static str]| Case {
        id,
        files: files(&module_fn, consumer),
        caller: "run",
        callee: "m",
        expect: expect.to_vec(),
    };
    run_cases(vec![
        // #436: a require inside a function is not a top-level binding.
        mk_fn(
            "require-inside-function",
            "function run() { const m = require('./m'); m(1); }\n",
            NONE,
        ),
        mk(
            "member-require-inside-function",
            "function run() { const m = require('./m'); m.f(1); }\n",
            "f",
            NONE,
        ),
        mk_fn(
            "require-inside-block",
            "if (process.env.X) { var m = require('./m'); }\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "require-inside-try",
            "try { var m = require('./m'); } catch (e) {}\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "exported-declaration",
            "export const m = require('./m');\nfunction run() { m(1); }\n",
            NONE,
        ),
        // The binding is not the require.
        mk_fn(
            "shadowed-by-parameter",
            "const m = require('./m');\nfunction run(m) { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "shadowed-by-inner-declaration",
            "const m = require('./m');\nfunction run() { const m = other; m(1); }\n",
            NONE,
        ),
        mk(
            "member-shadowed-by-parameter",
            "const m = require('./m');\nfunction run(m) { m.f(1); }\n",
            "f",
            NONE,
        ),
        // The name is also bound by a top-level require: only the nearest binding counts.
        mk_fn(
            "shadowed-by-inner-require",
            "const m = require('./other');\nfunction run() { const m = require('./m'); m(1); }\n",
            NONE,
        ),
        mk(
            "member-shadowed-by-inner-require",
            "const m = require('./other');\nfunction run() { const m = require('./m'); m.f(1); }\n",
            "f",
            NONE,
        ),
        mk(
            "member-reassigned-var",
            "var m = require('./m');\nm = other;\nfunction run() { m.f(1); }\n",
            "f",
            NONE,
        ),
        mk(
            "member-reassigned-inside-function",
            "var m = require('./m');\nfunction swap() { m = other; }\nfunction run() { m.f(1); }\n",
            "f",
            NONE,
        ),
        mk_fn(
            "reassigned-var",
            "var m = require('./m');\nm = other;\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "reassigned-inside-function",
            "var m = require('./m');\nfunction swap() { m = other; }\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "reassigned-by-array-pattern",
            "var m = require('./m');\n[m] = [other];\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "reassigned-by-for-of",
            "var m = require('./m');\nfor (m of others) {}\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "redeclared-var",
            "var m = require('./m');\nvar m = other;\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "function-and-var-same-name",
            "var m = require('./m');\nfunction m() {}\nfunction run() { m(1); }\n",
            &["drop LocalBindingUnproven"],
        ),
        // The specifier is not one plain relative literal.
        mk_fn(
            "bare-package",
            "const m = require('m');\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "absolute-specifier",
            "const m = require('/abs/m');\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "missing-file",
            "const m = require('./nope');\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "computed-specifier",
            "const p = './m';\nconst m = require(p);\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "concatenated-specifier",
            "const m = require('./' + 'm');\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "template-specifier",
            "const m = require(`./m`);\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "second-argument",
            "const m = require('./m', 1);\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "optional-call-require",
            "const m = require?.('./m');\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "require-shadowed",
            "function require(x) { return x; }\nconst m = require('./m');\nfunction run() { m(1); }\n",
            NONE,
        ),
        mk_fn(
            "require-written",
            "require = other;\nconst m = require('./m');\nfunction run() { m(1); }\n",
            NONE,
        ),
        // The value is wrapped, or the callee is not a plain member.
        mk_fn(
            "require-result-called-through-wrapper",
            "const m = wrap(require('./m'));\nfunction run() { m(1); }\n",
            NONE,
        ),
        // Main binds `m` to the callback literal the declarator's name is inferred onto.
        mk_fn(
            "require-with-function-argument",
            "const m = require('./m')(function () {});\nfunction run() { m(1); }\n",
            &["app.js:m@1-1 Exact/local_def"],
        ),
        mk(
            "member-optional-chain",
            "const m = require('./m');\nfunction run() { m?.f(1); }\n",
            "f",
            NONE,
        ),
        mk(
            "member-computed",
            "const m = require('./m');\nfunction run() { m['f'](1); }\n",
            "f",
            &[],
        ),
        mk(
            "member-of-member-alias",
            "const q = require('./m').f;\nfunction run() { q.g(1); }\n",
            "g",
            NONE,
        ),
        // A caller that is an anonymous or module-level owner keeps its outcome.
        Case {
            caller: "*",
            ..mk(
                "anonymous-callback-caller",
                "const m = require('./m');\n[1].forEach(() => { m.f(1); });\n",
                "f",
                NONE,
            )
        },
        Case {
            caller: "*",
            ..mk(
                "module-level-caller",
                "const m = require('./m');\nm.f(1);\n",
                "f",
                NONE,
            )
        },
        Case {
            caller: "*",
            ..mk_fn(
                "anonymous-callback-caller-module-value",
                "const m = require('./m');\n[1].forEach(() => { m(1); });\n",
                &["drop LocalBindingUnproven"],
            )
        },
    ]);
}

#[test]
fn cross_family_targets_prove_nothing() {
    run_cases(vec![
        Case {
            id: "only-a-c-file",
            files: vec![
                ("native.c", "int f(int x) { return x; }\n".to_owned()),
                ("app.js", app("const m = require('./native');", "m(1)")),
            ],
            caller: "run",
            callee: "m",
            expect: NONE.to_vec(),
        },
        Case {
            id: "only-a-python-file",
            files: vec![
                ("helper.py", "def f(x):\n    return x\n".to_owned()),
                ("app.js", app("const m = require('./helper');", "m.f(1)")),
            ],
            caller: "run",
            callee: "f",
            expect: NONE.to_vec(),
        },
    ]);
}

#[test]
fn typescript_optional_chains_prove_nothing() {
    let producer = format!("{FG}module.exports = {{ f }};\n");
    run_cases(vec![
        Case {
            id: "ts-optional-member-call",
            files: vec![
                ("m.ts", producer.clone()),
                ("app.ts", app("const m = require('./m');", "m?.f(1)")),
            ],
            caller: "run",
            callee: "f",
            expect: NONE.to_vec(),
        },
        Case {
            id: "ts-optional-require-call",
            files: vec![
                ("m.ts", format!("{FG}module.exports = f;\n")),
                ("app.ts", app("const m = require?.('./m');", "m(1)")),
            ],
            caller: "run",
            callee: "m",
            expect: NONE.to_vec(),
        },
    ]);
}

/// Shapes reduced from real call sites of published packages (names stay in the evidence, not
/// here): the declaration keyword, specifier spelling, export form and neighbours as found.
#[test]
fn census_reduced_shapes() {
    let case = |id: &'static str,
                producer_path: &'static str,
                producer: &str,
                consumer_path: &'static str,
                consumer: &str,
                callee: &'static str,
                expect: &[&'static str]| Case {
        id,
        files: vec![
            (producer_path, producer.to_owned()),
            (consumer_path, consumer.to_owned()),
        ],
        caller: "run",
        callee,
        expect: expect.to_vec(),
    };
    run_cases(vec![
        // `var x = require('../dir/x')` with `module.exports = x` of a hoisted declaration.
        case(
            "census-var-module-fn-parent-dir",
            "object/mixIn.js",
            "function mixIn(target, objects) { return target; }\nmodule.exports = mixIn;\n",
            "date/i18n.js",
            "var mixIn = require('../object/mixIn');\nfunction run(a, b) { return mixIn(a, b); }\n",
            "mixIn",
            &["object/mixIn.js:mixIn@1-1 Exact/import_member"],
        ),
        // The export is the last line of a longer module.
        case(
            "census-module-fn-after-body",
            "lib/is-mergeable.js",
            "var A = 1;\nfunction isMergeable(selector, mergeable) {\n  return selector + mergeable;\n}\n\nmodule.exports = isMergeable;\n",
            "lib/merge.js",
            "var isMergeable = require('./is-mergeable');\nfunction run(token) { return isMergeable(token, 1); }\n",
            "isMergeable",
            &["lib/is-mergeable.js:isMergeable@2-4 Exact/import_member"],
        ),
        // A multi-declarator `var` mixing local values and several requires.
        case(
            "census-multi-declarator-var-members",
            "lib/compile/util.js",
            "function copy(o, to) { return to; }\nfunction toHash(arr) { return arr; }\nmodule.exports = {\n  copy: copy,\n  toHash: toHash\n};\n",
            "lib/ajv.js",
            "var other = require('./other')\n  , resolve = require('./compile/resolve')\n  , util = require('./compile/util');\nfunction run(self) { return util.copy(self._opts); }\n",
            "copy",
            &["lib/compile/util.js:copy@1-1 Exact/import_member"],
        ),
        // `./../x` spelling.
        case(
            "census-dot-dot-slash-spelling",
            "lib/utils.js",
            "function isString(v) { return typeof v === 'string'; }\nmodule.exports = {\n  isString: isString\n};\n",
            "lib/helpers/cookies.js",
            "var utils = require('./../utils');\nfunction run(path) { return utils.isString(path); }\n",
            "isString",
            &["lib/utils.js:isString@1-1 Exact/import_member"],
        ),
        // A long run of `module.exports.x = x` members, hoisted declarations after them.
        case(
            "census-module-exports-dot-members",
            "lib/common.js",
            "module.exports.isNothing = isNothing;\nmodule.exports.repeat = repeat;\nfunction isNothing(s) { return !s; }\nfunction repeat(string, count) { return string; }\n",
            "lib/mark.js",
            "var common = require('./common');\nfunction run(indent) { return common.repeat(' ', indent); }\n",
            "repeat",
            &["lib/common.js:repeat@4-4 Exact/import_member"],
        ),
        // Members registered with `exports.x = x` of hoisted declarations after the export.
        case(
            "census-exports-dot-hoisted",
            "lib/normalizer.js",
            "exports.removeDiacritics = removeDiacritics\nexports.converters = converters\nfunction removeDiacritics(text) { return text; }\nvar converters = {};\n",
            "lib/tokenizer.js",
            "const normalizer = require('../lib/normalizer')\nfunction run(text) { return normalizer.removeDiacritics(text) }\n",
            "removeDiacritics",
            &["lib/normalizer.js:removeDiacritics@3-3 Exact/import_member"],
        ),
        // `var x = require('./y.js').x`, producer member is a named function expression.
        case(
            "census-member-alias-named-function-expression",
            "lib/static.js",
            "exports.writeStatic = function writeStatic(request, response, callback) {\n  return callback();\n};\n",
            "lib/index.js",
            "var writeStatic = require('./static.js').writeStatic\nfunction run(request, response) { return writeStatic(request, response, function () { return 1; }); }\n",
            "writeStatic",
            &["lib/static.js:writeStatic@1-3 Exact/import_member"],
        ),
        // `var x = require('./y').x` where the export is a hoisted declaration.
        case(
            "census-member-alias-declared-function",
            "lib/utils.js",
            "function shell(cmd, args) { return cmd; }\nexports.shell = shell;\n",
            "lib/linux.js",
            "var shell = require('./utils').shell;\nfunction run(pid) { return shell('kill', ['-9', pid]); }\n",
            "shell",
            &["lib/utils.js:shell@1-1 Exact/import_member"],
        ),
        // Two function expressions share one name: the terminal is not unique, nothing binds.
        case(
            "census-duplicate-expression-names",
            "lib/utils.js",
            "exports.escape = function escape(arg) { return arg; };\nexports.unescape = function escape(arg) { return arg; };\n",
            "lib/compare.js",
            "var utils = require('./utils');\nfunction run(orig) { return utils.escape(orig); }\n",
            "escape",
            NONE,
        ),
        // A function used as the module value and also given static members.
        case(
            "census-function-module-with-static-member",
            "lib/resolve.js",
            "function resolve(compile, root, ref) { return ref; }\nfunction normalizeId(id) { return id; }\nresolve.normalizeId = normalizeId;\nmodule.exports = resolve;\n",
            "lib/ajv.js",
            "var resolve = require('./resolve');\nfunction run(key) { return resolve.normalizeId(key); }\n",
            "normalizeId",
            NONE,
        ),
        // The same module called as a function binds; the static member does not.
        case(
            "census-function-module-called",
            "lib/resolve.js",
            "function resolve(compile, root, ref) { return ref; }\nresolve.normalizeId = function normalizeId(id) { return id; };\nmodule.exports = resolve;\n",
            "lib/ajv.js",
            "var resolve = require('./resolve');\nfunction run(root) { return resolve(null, root, 'a'); }\n",
            "resolve",
            &["lib/resolve.js:resolve@1-1 Exact/import_member"],
        ),
        // A constructor function used through its static member.
        case(
            "census-constructor-function-static-member",
            "lib/classifier.js",
            "function Classifier(options) { this.options = options; }\nClassifier.load = function load(name, cb) { return cb(); };\nmodule.exports = Classifier\n",
            "lib/bayes.js",
            "const Classifier = require('./classifier')\nfunction run(filename) { return Classifier.load(filename, function () {}); }\n",
            "load",
            NONE,
        ),
        // The result of `require(..)` called inline stays out of this route.
        case(
            "census-inline-require-call",
            "info/ini_get.js",
            "function ini_get(varname) { return varname; }\nmodule.exports = ini_get;\n",
            "php/arsort.js",
            "function run() { return require('../info/ini_get')('x'); }\n",
            "require",
            &["drop UnknownName"],
        ),
        // A named function expression assigned to `module.exports` is not an identifier export.
        case(
            "census-module-exports-named-function-expression",
            "helper/makeString.js",
            "module.exports = function makeString(object) { return object; };\n",
            "levenshtein.js",
            "var makeString = require('./helper/makeString');\nfunction run(str1) { return makeString(str1); }\n",
            "makeString",
            NONE,
        ),
    ]);
}
