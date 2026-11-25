package sample
import "github.com/gin-gonic/gin"
func f(c *gin.Context, inp string) {
  t := inp; c.File(t) // @SRC @SINK
  observe(t)
}
