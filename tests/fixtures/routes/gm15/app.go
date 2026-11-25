package fixture

import (
	"example.com/routes/gm15/middleware"
	"github.com/gin-gonic/gin"
)

type Ctrl = gin.Context

func NewApp() *gin.Engine {
	r := gin.New()
	r.GET("/restricted", middleware.Ban)
	r.GET("/abort", (*Ctrl).Abort)
	return r
}
