package fixture
import (
    "gorm.io/driver/sqlite"
    "gorm.io/gorm"
)
type Book struct { ID uint; Edition string }
func edition(id uint) (string, error) {
    db, err := gorm.Open(sqlite.Open("books.db"), &gorm.Config{})
    if err != nil { return "", err }
    var book Book
    found := db.Where("id = ?", id).Attrs(Book{Edition:"preview"}).FirstOrInit(&book)
    if found.Error != nil { return "", found.Error }
    if found.RowsAffected == 0 {
        fallback := book.Edition
        return fallback, nil
    }
    stored := book.Edition
    return stored, nil
}
