package fixture

import "database/sql"

func load() (string, error) {
    db, _ := sql.Open("sqlite", ":memory:")
    var name string
    err := db.QueryRow("select 'a'").Scan(&name)
    return name, err
}
