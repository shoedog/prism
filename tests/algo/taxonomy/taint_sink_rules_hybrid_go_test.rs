//! #622 S1 hybrid-policy matrix, Go: a sink needs call position; only proven non-sinks are
//! suppressed, an unknown receiver keeps firing. Rows marked `probe:` replay the review r1 harness
//! (expectation = the pre-change result unless a drop is deliberate).

use super::taint_sink_rules_support::*;
use crate::common::*;

#[rustfmt::skip]
const GO_ROWS: &[Row] = &[
    Row { sink: true, label: "sqlx Queryx", path: "a.go", lang: Language::Go, source: "package p\nfunc h(db *sqlx.DB, in Input) {\n	x := in.V // SEED\n	db.Queryx(x)  // SINK\n}\n" },
    Row { sink: true, label: "sqlx MustExec", path: "a.go", lang: Language::Go, source: "package p\nfunc h(db *sqlx.DB, in Input) {\n	x := in.V // SEED\n	db.MustExec(x)  // SINK\n}\n" },
    Row { sink: true, label: "sqlx NamedExec", path: "a.go", lang: Language::Go, source: "package p\nfunc h(db *sqlx.DB, in Input) {\n	x := in.V // SEED\n	db.NamedExec(x, args)  // SINK\n}\n" },
    Row { sink: true, label: "sqlx QueryRowx", path: "a.go", lang: Language::Go, source: "package p\nfunc h(db *sqlx.DB, in Input) {\n	x := in.V // SEED\n	db.QueryRowx(x)  // SINK\n}\n" },
    Row { sink: true, label: "dbConn.Exec", path: "a.go", lang: Language::Go, source: "package p\nfunc h(dbConn *sql.Conn, in Input) {\n	x := in.V // SEED\n	dbConn.Exec(x)  // SINK\n}\n" },
    Row { sink: true, label: "handle *sql.DB Query", path: "a.go", lang: Language::Go, source: "package p\nfunc h(handle *sql.DB, in Input) {\n	x := in.V // SEED\n	handle.Query(x)  // SINK\n}\n" },
    Row { sink: true, label: "pgx r.pg.Exec", path: "a.go", lang: Language::Go, source: "package p\nfunc (r *Repo) h(ctx context.Context, in Input) {\n	x := in.V // SEED\n	r.pg.Exec(ctx, x) // SINK\n}\n" },
    Row { sink: true, label: "gorm chain Exec", path: "a.go", lang: Language::Go, source: "package p\nfunc (r *Repo) h(ctx context.Context, in Input) {\n	x := in.V // SEED\n	r.db.WithContext(ctx).Exec(x) // SINK\n}\n" },
    Row { sink: true, label: "method value query := db.Query", path: "a.go", lang: Language::Go, source: "package p\nfunc h(db *sql.DB, in Input) {\n	x := in.V // SEED\n	query := db.Query\n	query(x) // SINK\n}\n" },
    Row { sink: true, label: "Query on a non-db-named receiver", path: "a.go", lang: Language::Go, source: "package p\nfunc h(cache Cache, in Input) {\n	x := in.V // SEED\n	cache.Query(x)  // SINK\n}\n" },
    Row { sink: true, label: "aliased os/exec", path: "a.go", lang: Language::Go, source: "package p\nimport osexec \"os/exec\"\nfunc h(in Input) {\n	x := in.V // SEED\n	osexec.Command(x).Run()  // SINK\n}\n" },
    Row { sink: true, label: "exec.CommandContext", path: "a.go", lang: Language::Go, source: "package p\nfunc h(ctx context.Context, in Input) {\n	x := in.V // SEED\n	exec.CommandContext(ctx, x).Run()  // SINK\n}\n" },
    Row { sink: true, label: "syscall.Exec", path: "a.go", lang: Language::Go, source: "package p\nimport \"syscall\"\nfunc h(in Input) {\n	x := in.V // SEED\n	syscall.Exec(x, nil, nil)  // SINK\n}\n" },
    Row { sink: true, label: "aliased html/template", path: "a.go", lang: Language::Go, source: "package p\nimport htmpl \"html/template\"\nfunc h(in Input) {\n	x := in.V // SEED\n	_ = htmpl.HTML(x)  // SINK\n}\n" },
    Row { sink: true, label: "template.HTML", path: "a.go", lang: Language::Go, source: "package p\nimport \"html/template\"\nfunc h(in Input) {\n	x := in.V // SEED\n	_ = template.HTML(x)  // SINK\n}\n" },
    Row { sink: true, label: "response.HTML helper keeps firing", path: "a.go", lang: Language::Go, source: "package p\nfunc h(w http.ResponseWriter, r *http.Request, in Input) {\n	x := in.V // SEED\n	response.HTML(w, r, x)  // SINK\n}\n" },
    Row { sink: true, label: "os.Remove", path: "a.go", lang: Language::Go, source: "package p\nimport \"os\"\nfunc h(in Input) {\n	x := in.V // SEED\n	os.Remove(x)  // SINK\n}\n" },
    Row { sink: true, label: "os.RemoveAll", path: "a.go", lang: Language::Go, source: "package p\nimport \"os\"\nfunc h(in Input) {\n	x := in.V // SEED\n	os.RemoveAll(x)  // SINK\n}\n" },
    Row { sink: true, label: "os.Rename", path: "a.go", lang: Language::Go, source: "package p\nimport \"os\"\nfunc h(in Input) {\n	x := in.V // SEED\n	os.Rename(x, \"b\")  // SINK\n}\n" },
    Row { sink: true, label: "aliased os Chmod", path: "a.go", lang: Language::Go, source: "package p\nimport o \"os\"\nfunc h(in Input) {\n	x := in.V // SEED\n	o.Chmod(x, 0o777)  // SINK\n}\n" },
    Row { sink: true, label: "afero WriteFile", path: "a.go", lang: Language::Go, source: "package p\nimport \"github.com/spf13/afero\"\nfunc h(in Input) {\n	x := in.V // SEED\n	afero.WriteFile(fs, x, nil, 0o644)  // SINK\n}\n" },
    Row { sink: true, label: "store.Remove unknown receiver", path: "a.go", lang: Language::Go, source: "package p\nfunc h(store Store, in Input) {\n	x := in.V // SEED\n	store.Remove(x)  // SINK\n}\n" },
    Row { sink: true, label: "Sprintf where a local shadows a const", path: "a.go", lang: Language::Go, source: "package p\nconst format = \"%s\"\nfunc h(in Input) string {\n	x := in.V // SEED\n	format := x\n	return fmt.Sprintf(format) // SINK\n}\n" },
    Row { sink: true, label: "Sprintf where a param shadows a const", path: "a.go", lang: Language::Go, source: "package p\nconst format = \"%s\"\nfunc h(format string, in Input) string {\n	x := in.V // SEED\n	return fmt.Sprintf(format, x) // SINK\n}\n" },
    Row { sink: false, label: "Sprintf with a const not shadowed", path: "a.go", lang: Language::Go, source: "package p\nconst format = \"%s\"\nfunc h(in Input) string {\n	x := in.V // SEED\n	return fmt.Sprintf(format, x) // SINK\n}\n" },
    Row { sink: false, label: "Sprintf with a function-local const", path: "a.go", lang: Language::Go, source: "package p\nfunc h(in Input) string {\n	const format = \"%s\"\n	x := in.V // SEED\n	return fmt.Sprintf(format, x) // SINK\n}\n" },
    Row { sink: false, label: "Sprintf through aliased fmt with a literal", path: "a.go", lang: Language::Go, source: "package p\nimport f \"fmt\"\nfunc h(in Input) {\n	x := in.V // SEED\n	_ = f.Sprintf(\"%d\", x)  // SINK\n}\n" },
    Row { sink: false, label: "u.Query() zero args", path: "a.go", lang: Language::Go, source: "package p\nfunc h(u *url.URL, in Input) {\n	x := in.V // SEED\n	v := u.Query()  // SINK\n}\n" },
    Row { sink: false, label: "r.URL.Query() zero args", path: "a.go", lang: Language::Go, source: "package p\nfunc h(r *http.Request, in Input) {\n	x := in.V // SEED\n	v := r.URL.Query().Get(x)  // SINK\n}\n" },
    Row { sink: false, label: "gin ctx.Query", path: "a.go", lang: Language::Go, source: "package p\nfunc h(ctx *gin.Context, in Input) {\n	x := in.V // SEED\n	s := ctx.Query(\"state\")  // SINK\n}\n" },
    Row { sink: false, label: "echo c.QueryParam is another name", path: "a.go", lang: Language::Go, source: "package p\nfunc h(c echo.Context, in Input) {\n	x := in.V // SEED\n	s := c.QueryParam(x)  // SINK\n}\n" },
    Row { sink: false, label: "cobra.Command (imported package other than os/exec)", path: "a.go", lang: Language::Go, source: "package p\nimport \"github.com/spf13/cobra\"\nfunc h(in Input) {\n	x := in.V // SEED\n	cobra.Command(x)  // SINK\n}\n" },
    Row { sink: false, label: "container/list Remove", path: "a.go", lang: Language::Go, source: "package p\nfunc h(l *list.List, in Input) {\n	x := in.V // SEED\n	l.Remove(e)  // SINK\n}\n" },
    Row { sink: false, label: "lru cache Remove", path: "a.go", lang: Language::Go, source: "package p\nfunc h(cache *lru.Cache, in Input) {\n	x := in.V // SEED\n	cache.Remove(x)  // SINK\n}\n" },
    Row { sink: true, label: "imported helper package HTML keeps firing", path: "a.go", lang: Language::Go, source: "package p\nimport g \"example.com/app/response\"\nfunc h(in Input) {\n	x := in.V // SEED\n	h := g.HTML(x)  // SINK\n}\n" },
    Row { sink: true, label: "gin ctx.HTML template render fires (weak-pending)", path: "a.go", lang: Language::Go, source: "package p\nfunc h(ctx *gin.Context, in Input) {\n	x := in.V // SEED\n	ctx.HTML(200, tpl, x)  // SINK\n}\n" },
    Row { sink: true, label: "echo c.HTML raw string fires", path: "a.go", lang: Language::Go, source: "package p\nfunc h(c echo.Context, in Input) {\n\tx := in.V // SEED\n\tc.HTML(200, x)  // SINK\n}\n" },
    Row { sink: true, label: "iris ctx.HTML raw markup fires", path: "a.go", lang: Language::Go, source: "package p\nfunc h(ctx iris.Context, in Input) {\n\tx := in.V // SEED\n\tctx.HTML(x)  // SINK\n}\n" },
    Row { sink: false, label: "gin ctx.Query(x) stays suppressed", path: "a.go", lang: Language::Go, source: "package p\nfunc h(ctx *gin.Context, in Input) {\n\tx := in.V // SEED\n\tctx.Query(x)  // SINK\n}\n" },
    Row { sink: false, label: "field .HTML read", path: "a.go", lang: Language::Go, source: "package p\nfunc h(f Field, in Input) {\n	x := in.V // SEED\n	y := f.HTML + x  // SINK\n}\n" },
    Row { sink: false, label: "Writer-less Fprintln to a buffer", path: "a.go", lang: Language::Go, source: "package p\nfunc h(buf *bytes.Buffer, in Input) {\n	x := in.V // SEED\n	fmt.Fprintln(buf, x)  // SINK\n}\n" },
    Row { sink: false, label: "gorm Raw (not a base sink)", path: "a.go", lang: Language::Go, source: "package p\nfunc (r *Repo) h(in Input) {\n	x := in.V // SEED\n	r.db.Raw(x).Scan(&out) // SINK\n}\n" },
    Row { sink: false, label: "sqlx db.Select (not a base sink)", path: "a.go", lang: Language::Go, source: "package p\nfunc h(db *sqlx.DB, in Input) {\n	x := in.V // SEED\n	db.Select(&out, x)  // SINK\n}\n" },
    Row { sink: true, label: "probe: go control: db.Query", path: "a.go", lang: Language::Go, source: "package a\nfunc h(db *sql.DB, in Input) {\n	x := in.V // SEED\n	db.Query(x) // SINK\n}\n" },
    Row { sink: true, label: "probe: go sqlx db.Queryx", path: "a.go", lang: Language::Go, source: "package a\nfunc h(db *sqlx.DB, in Input) {\n	x := in.V // SEED\n	db.Queryx(x) // SINK\n}\n" },
    Row { sink: true, label: "probe: go sqlx db.MustExec", path: "a.go", lang: Language::Go, source: "package a\nfunc h(db *sqlx.DB, in Input) {\n	x := in.V // SEED\n	db.MustExec(x) // SINK\n}\n" },
    Row { sink: false, label: "probe: go sqlx db.Select", path: "a.go", lang: Language::Go, source: "package a\nfunc h(db *sqlx.DB, in Input) {\n	x := in.V // SEED\n	db.Select(&out, x) // SINK\n}\n" },
    Row { sink: true, label: "probe: go dbConn.Exec", path: "a.go", lang: Language::Go, source: "package a\nfunc h(dbConn *sql.Conn, in Input) {\n	x := in.V // SEED\n	dbConn.Exec(x) // SINK\n}\n" },
    Row { sink: true, label: "probe: go gorm db.WithContext(ctx).Exec", path: "a.go", lang: Language::Go, source: "package a\nfunc (r *Repo) h(ctx context.Context, in Input) {\n	x := in.V // SEED\n	r.db.WithContext(ctx).Exec(x) // SINK\n}\n" },
    Row { sink: false, label: "probe: go gorm db.Raw", path: "a.go", lang: Language::Go, source: "package a\nfunc (r *Repo) h(in Input) {\n	x := in.V // SEED\n	r.db.Raw(x).Scan(&out) // SINK\n}\n" },
    Row { sink: true, label: "probe: go pgx r.pg.Exec", path: "a.go", lang: Language::Go, source: "package a\nfunc (r *Repo) h(ctx context.Context, in Input) {\n	x := in.V // SEED\n	r.pg.Exec(ctx, x) // SINK\n}\n" },
    Row { sink: true, label: "probe: go syscall.Exec (structured)", path: "a.go", lang: Language::Go, source: "package a\nimport \"syscall\"\nfunc h(in Input) {\n	x := in.V // SEED\n	syscall.Exec(x, nil, nil) // SINK\n}\n" },
    Row { sink: true, label: "probe: go aliased osexec.Command", path: "a.go", lang: Language::Go, source: "package a\nimport osexec \"os/exec\"\nfunc h(in Input) {\n	x := in.V // SEED\n	osexec.Command(x).Run() // SINK\n}\n" },
    Row { sink: true, label: "probe: go os.RemoveAll (structured)", path: "a.go", lang: Language::Go, source: "package a\nimport \"os\"\nfunc h(in Input) {\n	x := in.V // SEED\n	os.RemoveAll(x) // SINK\n}\n" },
    Row { sink: true, label: "probe: go fmt.Fprintf(w) writer", path: "a.go", lang: Language::Go, source: "package a\nimport (\"fmt\"; \"net/http\")\nfunc h(w http.ResponseWriter, r *http.Request) {\n	x := r.FormValue(\"q\") // SEED\n	fmt.Fprintf(w, \"<p>%s</p>\", x) // SINK\n}\n" },
    Row { sink: false, label: "probe: go w.Write([]byte(x))", path: "a.go", lang: Language::Go, source: "package a\nimport \"net/http\"\nfunc h(w http.ResponseWriter, r *http.Request) {\n	x := r.FormValue(\"q\") // SEED\n	w.Write([]byte(x)) // SINK\n}\n" },
    Row { sink: false, label: "probe: go io.WriteString(w,x)", path: "a.go", lang: Language::Go, source: "package a\nimport (\"io\"; \"net/http\")\nfunc h(w http.ResponseWriter, r *http.Request) {\n	x := r.FormValue(\"q\") // SEED\n	io.WriteString(w, x) // SINK\n}\n" },
    Row { sink: false, label: "probe: go c.String gin", path: "a.go", lang: Language::Go, source: "package a\nfunc h(c *gin.Context) {\n	x := c.Query(\"q\") // SEED\n	c.Data(200, \"text/html\", []byte(x)) // SINK\n}\n" },
    Row { sink: true, label: "probe: go template.HTML via alias htmpl", path: "a.go", lang: Language::Go, source: "package a\nimport htmpl \"html/template\"\nfunc h(in Input) {\n	x := in.V // SEED\n	_ = htmpl.HTML(x) // SINK\n}\n" },
];

#[test]
fn hybrid_go_matrix() {
    assert_rows(GO_ROWS);
}
