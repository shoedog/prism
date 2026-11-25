package fixture
import "github.com/gin-gonic/gin"
func main() {
  r := gin.Default()
  r.POST("/", func(c *gin.Context) {
    name := c.DefaultPostForm("name", "guest")
    println(name)
  })
  r.Run()
}
