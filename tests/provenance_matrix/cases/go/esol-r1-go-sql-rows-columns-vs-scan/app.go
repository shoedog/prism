package fixture

import "database/sql"

func load(driver string) ([]string, string) {
    conn, err := sql.Open(driver, "catalog")
    if err != nil { return nil, "" }
    defer conn.Close()
    rows, err := conn.Query("select 'desk' as label")
    if err != nil { return nil, "" }
    defer rows.Close()
    names, _ := rows.Columns()
    if !rows.Next() { return nil, "" }
    var label string
    if err := rows.Scan(&label); err != nil { return nil, "" }
    return names, label
}
