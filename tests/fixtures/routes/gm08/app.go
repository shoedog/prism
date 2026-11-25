package fixture

import "github.com/gin-gonic/gin"

type Base struct{}

func (b Base) Health(ctx *gin.Context) {}

type Ctrl struct {
	Base
}

type apiRouter struct {
	ctrl *Ctrl
}

func (a *apiRouter) routes() *gin.Engine {
	r := gin.New()
	r.GET("/health", a.ctrl.Health)
	return r
}

func NewApp() *gin.Engine {
	a := &apiRouter{ctrl: &Ctrl{}}
	return a.routes()
}
