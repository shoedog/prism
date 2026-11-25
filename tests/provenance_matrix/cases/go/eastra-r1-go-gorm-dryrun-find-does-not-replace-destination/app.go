package fixture
import (
    "gorm.io/driver/sqlite"
    "gorm.io/gorm"
)
type Edition struct { ID int; Title string }
func titles(path string) (string, string, error) {
    db, err := gorm.Open(sqlite.Open(path), &gorm.Config{})
    if err != nil { return "", "", err }
    preview := Edition{Title: "sample"}
    db.Session(&gorm.Session{DryRun: true}).First(&preview)
    var saved Edition
    if err := db.First(&saved).Error; err != nil { return "", "", err }
    draft, live := preview.Title, saved.Title
    return draft, live, nil
}
