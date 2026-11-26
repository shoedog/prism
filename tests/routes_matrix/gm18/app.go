package fixture

import "github.com/gin-gonic/gin"

type Ctrl struct{}

func (c *Ctrl) List(ctx *gin.Context) {}

type fieldCtrl struct{}

func (c *fieldCtrl) List(ctx *gin.Context) {}

type panel struct {
	f *fieldCtrl
}

func register(c *Ctrl, p *panel, ctx *gin.Context) *gin.Engine {
	r := gin.New()
	r.GET("/x", c.List)
	c.List(ctx)
	r.GET("/panel", p.f.List)
	p.f.List(ctx)
	return r
}

func NewApp() *gin.Engine {
	return register(&Ctrl{}, &panel{f: &fieldCtrl{}}, &gin.Context{})
}
