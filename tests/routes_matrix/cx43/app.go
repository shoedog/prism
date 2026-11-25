package fixture

import "github.com/gin-gonic/gin"

func list(c *gin.Context) {}

func NewApp() *gin.Engine {
	r := gin.New()
	r.GET("/orders", func(c *gin.Context) { list(c) })
	return r
}
