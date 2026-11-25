package fixture
import "github.com/gin-gonic/gin"
func run() {
  r := gin.New()
  r.GET("/", func(c *gin.Context) {
    token := c.GetHeader("Authorization")
    consume(token)
  })
}
func consume(...any) {}
