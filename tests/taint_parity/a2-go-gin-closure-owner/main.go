package example
import "fmt"
import "github.com/gin-gonic/gin"
type Context = gin.Context
func keep(c *gin.Context) {}
func isolated() {
    t := acquire() // @SRC
    observe(t)
}
type fixed struct{}
func (*fixed) Param(key string) string { return "fixed" }
func read(c *gin.Context) {
    func(c *fixed) {
        value := c.Param("key")
        _ = fmt.Sprintf(value) // @SINK
    }(&fixed{})
}
