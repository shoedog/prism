package catalog
import (
    "gorm.io/driver/sqlite"
    "gorm.io/gorm"
)
type Book struct { ID uint; Title string; Edition string }
func (book *Book) AfterFind(_ *gorm.DB) error {
    book.Edition = "preview"
    return nil
}
func labels(path string) (string, string) {
    db, err := gorm.Open(sqlite.Open(path))
    if err != nil { panic(err) }
    var book Book
    if err := db.First(&book).Error; err != nil { panic(err) }
    title, edition := book.Title, book.Edition
    return title, edition
}
