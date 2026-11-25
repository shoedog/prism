package fixture

import "database/sql"

func load() string {
    db, _ := sql.Open("sqlite", ":memory:")
    row := db.QueryRow("select 'a'")
    var name string
    row.Scan(&name)
    return name
}

func open() *sql.DB {
    db, _ := sql.Open("sqlite", ":memory:")
    return db
}
