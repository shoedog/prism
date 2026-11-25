package fixture

import "github.com/gin-gonic/gin"

func read(ctx *gin.Context) string {
    term := ctx.Query("term")
    return term
}
