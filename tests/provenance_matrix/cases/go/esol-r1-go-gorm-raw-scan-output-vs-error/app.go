package fixture

import (
    "gorm.io/driver/sqlite"
    "gorm.io/gorm"
)

func load() (string, error) {
    store, err := gorm.Open(sqlite.Open(":memory:"), &gorm.Config{})
    if err != nil { return "", err }
    var item struct{ Label string }
    scanErr := store.Raw("select 'desk' as label").Scan(&item).Error
    return item.Label, scanErr
}
