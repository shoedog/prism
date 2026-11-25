package fixture

import ("context"; "github.com/jackc/pgx/v5")

func load(ctx context.Context, dsn string) (pgx.Rows, error) {
    connection, err := pgx.Connect(ctx, dsn)
    if err != nil { return nil, err }
    rows, queryErr := connection.Query(ctx, "select 1")
    return rows, queryErr
}
