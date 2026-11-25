package main
import "github.com/gin-gonic/gin"
func main() {
  router := gin.Default()
  router.GET("/users/:id", func(c *gin.Context) {
    id := c.Param("id")
    _ = id
  })
}
