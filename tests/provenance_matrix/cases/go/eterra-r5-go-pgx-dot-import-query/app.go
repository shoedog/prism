package fixture
import ("context"; . "github.com/jackc/pgx/v5")
func run(ctx context.Context) {
  conn, _ := Connect(ctx, "postgres://localhost/db")
  rows, err := conn.Query(ctx, "select 1")
  consume(rows, err)
}
func consume(...any) {}
