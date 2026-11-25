package fixture

import "github.com/gin-gonic/gin"

func list(c *gin.Context) {}

func register(g *gin.RouterGroup) {
	g.GET("/x", list)
}

func NewApp() *gin.Engine {
	r := gin.New()
	g := r.Group("/api")
	register(g)
	return r
}
