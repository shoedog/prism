package fixture

import "github.com/gin-gonic/gin"

func routes() *gin.Engine {
    router := gin.New()
    router.GET("/catalog", func(incoming *gin.Context) {
        var options struct { Label string `form:"label"` }
        if err := incoming.ShouldBindQuery(&options); err != nil { return }
        incoming.String(200, "%s", options.Label)
    })
    return router
}
