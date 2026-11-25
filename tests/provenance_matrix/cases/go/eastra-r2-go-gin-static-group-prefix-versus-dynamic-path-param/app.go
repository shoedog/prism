package fixture
import "github.com/gin-gonic/gin"
func routes() *gin.Engine {
    app := gin.New()
    group := app.Group("/catalog")
    group.GET("/:edition", func(c *gin.Context) {
        route := c.FullPath()
        edition := c.Param("edition")
        println(route, edition)
    })
    return app
}
