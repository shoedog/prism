package fixture

import "github.com/gin-gonic/gin"

func NewApp() *gin.Engine {
	r := gin.New()
	g := r.Group("/api")
	g.POST("/y", func(c *gin.Context) {})
	return r
}
