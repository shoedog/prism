package example
import "fmt"
import "github.com/gin-gonic/gin"
type Context = gin.Context
func keep(c *gin.Context) {}
func isolated() {
    t := acquire() // @SRC
    observe(t)
}
func read(first, other *gin.Context) {
    value := other.Param("key")
    _ = fmt.Sprintf(value) // @SINK
}
