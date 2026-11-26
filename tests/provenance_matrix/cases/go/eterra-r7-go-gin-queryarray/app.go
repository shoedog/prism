package main
import "github.com/gin-gonic/gin"
func main() {
 r := gin.Default()
 r.GET("/", func(c *gin.Context) {
  tags := c.QueryArray("tag")
  _ = tags
 })
}
