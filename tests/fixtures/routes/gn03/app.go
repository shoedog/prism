package fixture

import "github.com/gin-gonic/gin"

type controller struct{}

func (c *controller) Get(ctx *gin.Context) {}

type api struct {
	ctrl *controller
}

func (a *api) routes() *gin.Engine {
	r := gin.New()
	r.GET("/x", a.ctrl.Get)
	return r
}

func NewApp() *gin.Engine {
	a := &api{ctrl: &controller{}}
	return a.routes()
}
