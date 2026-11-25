package fixture
import ("context"; "database/sql")
func run(ctx context.Context, db *sql.DB) {
  rows, err := db.QueryContext(ctx, "select 1")
  consume(rows, err)
}
func consume(...any) {}
