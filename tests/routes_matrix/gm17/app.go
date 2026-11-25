package fixture

import "github.com/gin-gonic/gin"

type Ctrl struct{}

func (c *Ctrl) List(ctx *gin.Context) {}

type Base struct {
	ctrl *Ctrl
}

type apiRouter struct {
	*Base
}

func (a *apiRouter) routes() *gin.Engine {
	r := gin.New()
	r.GET("/x", a.Base.ctrl.List)
	return r
}

func NewApp() *gin.Engine {
	a := &apiRouter{Base: &Base{ctrl: &Ctrl{}}}
	return a.routes()
}
