package fixture
import (
    "gorm.io/driver/sqlite"
    "gorm.io/gorm"
)
func load(path string) ([]string, int64) {
    db, err := gorm.Open(sqlite.Open(path), &gorm.Config{})
    if err != nil { return nil, 0 }
    var labels []string
    result := db.Table("catalog").Pluck("label", &labels)
    count := result.RowsAffected
    return labels, count
}
