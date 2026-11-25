package fixture
import "gorm.io/gorm"
type User struct { Name string }
func run(db *gorm.DB) {
  var users []User
  db.Find(&users)
  consume(users)
}
func consume(...any) {}
