package main
import "github.com/gin-gonic/gin"
func main() {
 r := gin.Default()
 r.GET("/users/:id", func(c *gin.Context) {
  id := c.Param("id")
  println(id)
 })
}
