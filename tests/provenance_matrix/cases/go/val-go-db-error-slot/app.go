package fixture

import "database/sql"

func load() (*sql.Rows, error) {
    db, _ := sql.Open("sqlite", ":memory:")
    v, err := db.Query("select 1")
    return v, err
}
