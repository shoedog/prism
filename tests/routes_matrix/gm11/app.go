package fixture

import "github.com/gin-gonic/gin"

type Ctrl struct{}

func (c *Ctrl) List(ctx *gin.Context) {}

type otherCtrl struct{}

func (c *otherCtrl) List(ctx *gin.Context) {}

type apiRouter struct {
	ctrl *Ctrl
}

type Other struct {
	ctrl *otherCtrl
}

func (a *apiRouter) routes(other *Other) *gin.Engine {
	r := gin.New()
	{
		a := other
		r.GET("/x", a.ctrl.List)
	}
	return r
}

func NewApp() *gin.Engine {
	a := &apiRouter{ctrl: &Ctrl{}}
	other := &Other{ctrl: &otherCtrl{}}
	return a.routes(other)
}
