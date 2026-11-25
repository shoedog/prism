package fixture

import "github.com/gin-gonic/gin"

func health(c *gin.Context) {}

func NewApp() *gin.Engine {
	r := gin.New()
	g := r.Group("/api")
	register(g)
	return r
}
