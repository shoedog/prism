package fixture
import (
    "context"
    "github.com/jackc/pgx/v5/pgxpool"
)
func load(ctx context.Context, dsn string) (string, error) {
    pool, err := pgxpool.New(ctx, dsn)
    if err != nil { return "", err }
    defer pool.Close()
    var title string
    scanErr := pool.QueryRow(ctx, "select title from editions limit 1").Scan(&title)
    if scanErr != nil { return "", scanErr }
    return title, nil
}
