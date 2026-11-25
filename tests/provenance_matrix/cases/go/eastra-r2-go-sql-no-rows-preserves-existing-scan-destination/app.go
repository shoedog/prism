package fixture
import (
    "database/sql"
    "os"
)
func title(driver, dsn string, id int) (string, error) {
    db, err := sql.Open(driver, dsn)
    if err != nil { return "", err }
    defer db.Close()
    name := os.Getenv("DEFAULT_TITLE")
    err = db.QueryRow("select title from books where id = ?", id).Scan(&name)
    if err == sql.ErrNoRows {
        return name, nil
    }
    if err != nil { return "", err }
    return name, nil
}
