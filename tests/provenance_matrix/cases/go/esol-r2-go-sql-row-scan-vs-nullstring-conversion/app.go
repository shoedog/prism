package fixture
import (
    "database/sql"
    _ "github.com/lib/pq"
)
func load(dsn string) (string, string, error) {
    db, err := sql.Open("postgres", dsn)
    if err != nil { return "", "", err }
    defer db.Close()
    var queried, manual sql.NullString
    if err := db.QueryRow("select label from catalog").Scan(&queried); err != nil {
        return "", "", err
    }
    localLabel := "fixed"
    if err := manual.Scan(localLabel); err != nil { return "", "", err }
    return queried.String, manual.String, nil
}
