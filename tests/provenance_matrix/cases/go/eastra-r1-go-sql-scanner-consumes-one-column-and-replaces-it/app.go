package fixture
import (
    "database/sql"
    "database/sql/driver"
)
type stamp string
func (s *stamp) Scan(_ any) error {
    *s = "local"
    return nil
}
func names(connector driver.Connector) (string, stamp, error) {
    db := sql.OpenDB(connector)
    defer db.Close()
    var title string
    var shown stamp
    err := db.QueryRow("select title, edition from catalog limit 1").Scan(&title, &shown)
    if err != nil { return "", "", err }
    return title, shown, nil
}
