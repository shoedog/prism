package fixture

import "github.com/gin-gonic/gin"

type Ctrl struct{}

func (c *Ctrl) List(ctx *gin.Context) {}

var defaultCtrl *Ctrl = &Ctrl{}

func NewApp() *gin.Engine {
	r := gin.New()
	r.GET("/x", defaultCtrl.List)
	return r
}
