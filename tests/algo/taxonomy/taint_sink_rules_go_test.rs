//! #622 S1 variant matrix, Go: review sinks match by call position and receiver.

use super::taint_sink_rules_support::*;
use crate::common::*;

#[rustfmt::skip]
const POSITIVES: &[Case] = &[
    Case { label: "db.Exec", source: "package p\nfunc h(db *sql.DB, in Input) {\n\tx := in.V // SEED\n\tdb.Exec(\"DELETE FROM t WHERE a=$1\", x) // SINK\n}\n" },
    Case { label: "field handle s.db.Exec", source: "package p\nfunc (s *S) h(in Input) {\n\tx := in.V // SEED\n\ts.db.Exec(\"DELETE FROM t WHERE a=$1\", x) // SINK\n}\n" },
    Case { label: "tx.QueryRow", source: "package p\nfunc h(tx *sql.Tx, in Input) {\n\tx := in.V // SEED\n\ttx.QueryRow(\"SELECT a FROM t WHERE b=$1\", x) // SINK\n}\n" },
    Case { label: "QueryContext on any receiver", source: "package p\nfunc h(ctx context.Context, h Handle, in Input) {\n\tx := in.V // SEED\n\th.QueryContext(ctx, x) // SINK\n}\n" },
    Case { label: "Fprintf to http.ResponseWriter with constant format", source: "package p\nfunc h(w http.ResponseWriter, in Input) {\n\tx := in.V // SEED\n\tfmt.Fprintf(w, \"data: %s\", x) // SINK\n}\n" },
    Case { label: "Fprintln to an io.Writer parameter", source: "package p\nfunc h(out io.Writer, in Input) {\n\tx := in.V // SEED\n\tfmt.Fprintln(out, x) // SINK\n}\n" },
    Case { label: "Sprintf with a non-constant format", source: "package p\nfunc h(format string, in Input) {\n\tx := in.V // SEED\n\ts := fmt.Sprintf(format, x) // SINK\n\t_ = s\n}\n" },
    Case { label: "Sprintf fed the tainted value as the format", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\ts := fmt.Sprintf(x) // SINK\n\t_ = s\n}\n" },
    Case { label: "Sprintf with a format read from a request field", source: "package p\nfunc h(req Request, in Input) {\n\tx := in.V // SEED\n\ts := fmt.Sprintf(req.Format, x) // SINK\n\t_ = s\n}\n" },
    Case { label: "template.HTML", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\ty := template.HTML(x) // SINK\n\t_ = y\n}\n" },
    Case { label: "exec.Command", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\texec.Command(x).Run() // SINK\n}\n" },
    Case { label: "os.Chmod", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\tos.Chmod(x, 0o777) // SINK\n}\n" },
    Case { label: "http.Redirect", source: "package p\nfunc h(w http.ResponseWriter, r *http.Request, in Input) {\n\tx := in.V // SEED\n\thttp.Redirect(w, r, x, 302) // SINK\n}\n" },
];

#[rustfmt::skip]
const NEGATIVES: &[Case] = &[
    Case { label: "constant-format Sprintf", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\ts := fmt.Sprintf(\"/v1/feeds/%d/refresh\", x) // SINK\n\t_ = s\n}\n" },
    Case { label: "package-const format Sprintf", source: "package p\nconst feedFmt = \"/v1/feeds/%d\"\nfunc h(in Input) {\n\tx := in.V // SEED\n\ts := fmt.Sprintf(feedFmt, x) // SINK\n\t_ = s\n}\n" },
    Case { label: "imported-package constant format Sprintf", source: "package p\nimport \"example.com/app/constant\"\nfunc h(in Input) {\n\tx := in.V // SEED\n\ts := fmt.Sprintf(constant.Prompt, x) // SINK\n\t_ = s\n}\n" },
    Case { label: "versioned import package constant format", source: "package p\nimport \"example.com/app/cfg/v2\"\nfunc h(in Input) {\n\tx := in.V // SEED\n\ts := fmt.Sprintf(cfg.Prompt, x) // SINK\n\t_ = s\n}\n" },
    Case { label: "constant-format Fprintf to a file", source: "package p\nfunc h(f *os.File, in Input) {\n\tx := in.V // SEED\n\tfmt.Fprintf(f, \"v=%s\", x) // SINK\n}\n" },
    Case { label: "sanitizedQuery name", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\tsanitizedQuery := trim(x) // SINK\n\t_ = sanitizedQuery\n}\n" },
    Case { label: "InvalidQueryError name", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\terr := InvalidQueryError(x) // SINK\n\t_ = err\n}\n" },
    Case { label: "URL.Query() call is a source, not a sink", source: "package p\nfunc h(r *http.Request, in Input) {\n\tx := in.V // SEED\n\tv := r.URL.Query().Get(x) // SINK\n\t_ = v\n}\n" },
    Case { label: "variable named source", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\tsource := x // SINK\n\t_ = source\n}\n" },
    Case { label: "variable named raw", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\traw := x // SINK\n\t_ = raw\n}\n" },
    Case { label: "variable named run", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\trun := x // SINK\n\t_ = run\n}\n" },
    Case { label: "variable named redirect", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\tredirect := x // SINK\n\t_ = redirect\n}\n" },
    Case { label: "removeUser helper", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\tremoveUser(x) // SINK\n}\n" },
    Case { label: "Find on a list", source: "package p\nfunc h(l List, in Input) {\n\tx := in.V // SEED\n\tl.Find(x) // SINK\n}\n" },
    Case { label: "sendEvent helper", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\tsendEvent(x) // SINK\n}\n" },
    Case { label: "field HTML read", source: "package p\nfunc h(f Field, in Input) {\n\tx := in.V // SEED\n\ty := f.HTML + x // SINK\n\t_ = y\n}\n" },
    Case { label: "func signature naming a sink word", source: "package p\nfunc sendQuery(x string) string { // SEED SINK\n\treturn x\n}\n" },
    Case { label: "freeze (free substring)", source: "package p\nfunc h(in Input) {\n\tx := in.V // SEED\n\tfreeze(x) // SINK\n}\n" },
    Case { label: "Writer-less Fprintln to a buffer", source: "package p\nfunc h(buf *bytes.Buffer, in Input) {\n\tx := in.V // SEED\n\tfmt.Fprintln(buf, x) // SINK\n}\n" },
];

#[test]
fn go_genuine_call_sinks_are_reported() {
    assert_all_sinks("a.go", Language::Go, POSITIVES);
}

#[test]
fn go_name_fragments_are_not_sinks() {
    assert_no_sinks("a.go", Language::Go, NEGATIVES);
}

#[test]
fn go_generic_structured_sink_keeps_main_path() {
    assert_all_sinks("a.go", Language::Go, &[Case {
        label: "generic structured path",
        source: "package p\nfunc h(in Input) {\n x := in.V // SEED\n os.ReadFile[T](x,0) // SINK\n}\n",
    }]);
}

#[test]
fn go_generic_flat_sink_keeps_callee_position() {
    assert_all_sinks("a.go", Language::Go, &[Case {
        label: "generic flat callee",
        source: "package p\nfunc h(db Handle, in Input) {\n x := in.V // SEED\n db.Queryx[T](x,0) // SINK\n}\n",
    }]);
}

#[test]
fn go_generic_file_constructor_is_not_a_collection() {
    assert_all_sinks("a.go", Language::Go, &[Case {
        label: "generic file constructor",
        source: "package p\nimport \"github.com/spf13/afero\"\nfunc h(in Input) {\n cache := afero.NewFs[T](0,0)\n x := in.V // SEED\n cache.Remove(x) // SINK\n}\n",
    }]);
}

#[test]
fn go_real_sink_named_container_indices_stay_indices() {
    assert_no_sinks("a.go", Language::Go, &[Case {
        label: "slice and map of sink-named functions",
        source: "package p\nfunc h(Query []func(string), ReadFile map[int]func(string), i int, in Input) {\n x := in.V // SEED\n Query[i](x); ReadFile[i](x) // SINK\n}\n",
    }]);
}

#[test]
fn go_generic_enclosing_sink_restores_continuation_severity() {
    use super::taint_sink_strength_test::{assert_severities, go, S, W};
    assert_severities(&[
        go("generic enclosing Exec is strong", "package p\nimport dbx \"example/dbx\"\nfunc h(db *sql.DB, q, name string, in Input) {\n x := in.V // SEED\n res := dbx.Exec[Res](q + name,\n  db.QueryRow(\"SELECT 1\", x)) // SINK\n _ = res\n}\n", W),
        go("non-generic enclosing Exec is strong", "package p\nimport dbx \"example/dbx\"\nfunc h(db *sql.DB, q, name string, in Input) {\n x := in.V // SEED\n res := dbx.Exec(q + name,\n  db.QueryRow(\"SELECT 1\", x)) // SINK\n _ = res\n}\n", W),
        go("generic constant query stays weak", "package p\nimport dbx \"example/dbx\"\nfunc h(db *sql.DB, in Input) {\n x := in.V // SEED\n res := dbx.Exec[Res](\"SELECT 1\",\n  db.QueryRow(\"SELECT 1\", x)) // SINK\n _ = res\n}\n", S),
    ]);
}
