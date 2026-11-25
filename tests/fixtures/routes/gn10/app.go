package fixture

import "github.com/gin-gonic/gin"

func mw(c *gin.Context) {
	c.Next()
}

func list(c *gin.Context) {}

func NewApp() *gin.Engine {
	r := gin.New()
	g := r.Group("", mw)
	g.GET("/x", list)
	return r
}
