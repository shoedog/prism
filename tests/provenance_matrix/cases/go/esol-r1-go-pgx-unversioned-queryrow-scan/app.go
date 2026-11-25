package fixture

import "github.com/jackc/pgx"

func load(dsn string) (string, error) {
    config, err := pgx.ParseConnectionString(dsn)
    if err != nil { return "", err }
    conn, err := pgx.Connect(config)
    if err != nil { return "", err }
    defer conn.Close()
    var label string
    if err := conn.QueryRow("select 'desk'").Scan(&label); err != nil { return "", err }
    return label, nil
}
