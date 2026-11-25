package catalog
import "database/sql"
func title(driver, dsn string) (string, int) {
    db, err := sql.Open(driver, dsn)
    if err != nil { panic(err) }
    defer db.Close()
    rows, err := db.Query("SELECT title, NULL FROM editions WHERE title IS NOT NULL")
    if err != nil { panic(err) }
    defer rows.Close()
    if !rows.Next() { panic("no edition") }
    var label string
    count := 17
    if err := rows.Scan(&label, &count); err == nil { panic("wanted NULL") }
    return label, count
}
