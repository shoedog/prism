package catalog
import (
    "database/sql"
    "io"
    "net/http"
)
func edition(network bool, driver, dsn string) []byte {
    db, err := sql.Open(driver, dsn)
    if err != nil { panic(err) }
    defer db.Close()
    var title []byte
    if network {
        reply, err := http.Get("https://catalog.example/release")
        if err != nil { panic(err) }
        defer reply.Body.Close()
        bytes, err := io.ReadAll(reply.Body)
        if err != nil { panic(err) }
        title = bytes
    } else {
        if err := db.QueryRow("SELECT title FROM releases").Scan(&title); err != nil { panic(err) }
    }
    return title
}
