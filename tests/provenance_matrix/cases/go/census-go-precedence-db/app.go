package fixture

import "database/sql"

func read() *sql.Rows {
    db, _ := sql.Open("sqlite", ":memory:")
    rows, _ := db.Query("select 'request'")
    return rows
}
