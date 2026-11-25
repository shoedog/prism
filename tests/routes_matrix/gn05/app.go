package fixture

import "github.com/gin-gonic/gin"

func list(c *gin.Context) {}

func registerItems(g *gin.RouterGroup) {
	g.GET("/x", list)
}

func register(g *gin.RouterGroup) {
	registerItems(g)
}

func NewApp() *gin.Engine {
	r := gin.New()
	g := r.Group("/api")
	register(g)
	return r
}
