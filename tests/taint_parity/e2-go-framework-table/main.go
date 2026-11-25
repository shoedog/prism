package sample
import "github.com/gin-gonic/gin"
func f(c *gin.Context, inp string) {
  t := inp // @SRC
  c.File(t); observe(t) // @SINK
}
