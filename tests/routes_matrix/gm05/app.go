package fixture

import "github.com/gin-gonic/gin"

type Ctrl struct{}

func (c *Ctrl) List(ctx *gin.Context) {}

func reg(r *gin.RouterGroup, c *Ctrl) {
	r.GET("/x", c.List)
}

func NewApp() *gin.Engine {
	app := gin.New()
	group := app.Group("/api")
	reg(group, &Ctrl{})
	return app
}
