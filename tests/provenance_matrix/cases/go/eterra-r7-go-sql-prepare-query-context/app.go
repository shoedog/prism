package main
import ("context"; "database/sql")
func load(ctx context.Context) {
 db, _ := sql.Open("driver", "dsn")
 stmt, _ := db.PrepareContext(ctx, "select 1")
 rows, err := stmt.QueryContext(ctx)
 _, _ = rows, err
}
