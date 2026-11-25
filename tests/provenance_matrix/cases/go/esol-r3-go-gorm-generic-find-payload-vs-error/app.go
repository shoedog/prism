package fixture

import (
  "context"
  "gorm.io/driver/sqlite"
  "gorm.io/gorm"
)

type Book struct { Title string }

func catalog(ctx context.Context, path string) ([]Book, error) {
  db, err := gorm.Open(sqlite.Open(path), &gorm.Config{})
  if err != nil { return nil, err }
  books, findErr := gorm.G[Book](db).Find(ctx)
  return books, findErr
}
