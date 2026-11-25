package fixture

import "github.com/gin-gonic/gin"

func replaceSettings(c *gin.Context) {}

func NewApp() *gin.Engine {
	r := gin.New()
	r.PUT("/settings", replaceSettings)
	return r
}
