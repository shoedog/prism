package fixture

import "github.com/gin-gonic/gin"

type Ctrl struct{}

func (c Ctrl) List(ctx *gin.Context) {}

func (c *Ctrl) Show(ctx *gin.Context) {}

type apiRouter struct {
	ctrl *Ctrl
}

func (a *apiRouter) routes() *gin.Engine {
	r := gin.New()
	r.GET("/items", a.ctrl.List)
	r.GET("/items/:id", a.ctrl.Show)
	return r
}

func NewApp() *gin.Engine {
	a := &apiRouter{ctrl: &Ctrl{}}
	return a.routes()
}
