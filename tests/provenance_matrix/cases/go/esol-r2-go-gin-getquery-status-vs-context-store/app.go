package fixture
import "github.com/gin-gonic/gin"
func routes() *gin.Engine {
    router := gin.New()
    router.GET("/catalog", func(incoming *gin.Context) {
        incoming.Set("cached", "fixed")
        phrase, supplied := incoming.GetQuery("q")
        cached, _ := incoming.Get("cached")
        _, _, _ = phrase, supplied, cached
    })
    return router
}
