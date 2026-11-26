package fixture
import "github.com/gin-gonic/gin"
func server() *gin.Engine {
    engine := gin.New()
    engine.POST("/preview", func(ctx *gin.Context) {
        raw, err := ctx.GetRawData()
        if err != nil { return }
        header := ctx.GetHeader("Upgrade")
        upgrading := ctx.IsWebsocket()
        _, _, _ = raw, header, upgrading
    })
    return engine
}
