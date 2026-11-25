package fixture

import "github.com/gin-gonic/gin"

type connector struct{}

func (c *connector) Info(ctx *gin.Context) {}

type pluginRouter struct {
	connector *connector
}

func (pr *pluginRouter) routes() *gin.Engine {
	r := gin.New()
	c := pr.connector
	r.GET("/c", c.Info)
	return r
}

func NewApp() *gin.Engine {
	pr := &pluginRouter{connector: &connector{}}
	return pr.routes()
}
