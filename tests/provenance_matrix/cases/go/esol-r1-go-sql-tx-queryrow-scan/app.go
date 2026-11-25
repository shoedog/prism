package fixture

import "database/sql"

func load(driver string) (string, error) {
    conn, err := sql.Open(driver, "catalog")
    if err != nil { return "", err }
    defer conn.Close()
    tx, err := conn.Begin()
    if err != nil { return "", err }
    defer tx.Rollback()
    var label string
    if scanErr := tx.QueryRow("select 'desk'").Scan(&label); scanErr != nil { return "", scanErr }
    return label, nil
}
