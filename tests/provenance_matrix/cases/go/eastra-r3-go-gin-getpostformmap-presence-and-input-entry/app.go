package catalog
import "github.com/gin-gonic/gin"
func routes() *gin.Engine {
    router := gin.New()
    router.POST("/filters", func(incoming *gin.Context) {
        fields, ok := incoming.GetPostFormMap("filter")
        if !ok { return }
        value := fields["edition"]
        incoming.JSON(200, []any{ok, value})
    })
    return router
}
