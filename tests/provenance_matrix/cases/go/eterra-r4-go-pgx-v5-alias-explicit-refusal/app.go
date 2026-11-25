package fixture
import ("context"; dbx "github.com/jackc/pgx/v5")
func run(ctx context.Context) {
  conn, _ := dbx.Connect(ctx, "postgres://localhost/app")
  rows, err := conn.Query(ctx, "select 1")
  consume(rows, err)
}
func consume(...any) {}
