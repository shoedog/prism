package sample
import "github.com/gin-gonic/gin"
func f(c *gin.Context) {
  other := c.Query("other"); c.File(c.Param("f")); println(other) // @SRC @SINK
  println(other)
}
