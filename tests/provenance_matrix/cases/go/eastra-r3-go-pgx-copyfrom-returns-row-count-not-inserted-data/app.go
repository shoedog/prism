package catalog
import (
    "context"
    "os"
    "github.com/jackc/pgx/v5"
    "github.com/jackc/pgx/v5/pgxpool"
)
func insert(ctx context.Context, dsn string) (string, int64, error) {
    pool, err := pgxpool.New(ctx, dsn)
    if err != nil { return "", 0, err }
    defer pool.Close()
    edition := os.Getenv("EDITION")
    count, err := pool.CopyFrom(ctx, pgx.Identifier{"editions"}, []string{"title"},
        pgx.CopyFromRows([][]any{{edition}}))
    if err != nil { return "", 0, err }
    return edition, count, nil
}
