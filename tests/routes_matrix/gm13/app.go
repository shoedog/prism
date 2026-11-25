package fixture

import "github.com/gin-gonic/gin"

type Ctrl struct{}

func (c *Ctrl) List(ctx *gin.Context) {}

func newCtrl() *Ctrl {
	return &Ctrl{}
}

type inner struct {
	z *Ctrl
}

type outer struct {
	y inner
}

type apiRouter struct {
	x outer
}

func (a *apiRouter) routes() *gin.Engine {
	r := gin.New()
	m := map[string]*Ctrl{"k": newCtrl()}
	r.GET("/fresh", newCtrl().List)
	r.GET("/deep", a.x.y.z.List)
	r.GET("/keyed", m["k"].List)
	return r
}

func NewApp() *gin.Engine {
	a := &apiRouter{x: outer{y: inner{z: newCtrl()}}}
	return a.routes()
}
