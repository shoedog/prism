package sample
import ("database/sql"; "github.com/gin-gonic/gin")
func f(c *gin.Context, db *sql.DB) {
  t := c.Query("f"); db.Query(t) // @SRC @SINK
  observe(t)
}
