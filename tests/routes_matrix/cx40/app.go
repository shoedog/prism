package fixture

import "github.com/gin-gonic/gin"

type controller struct{}

func (controller) list(c *gin.Context) {}

type panel struct{ controller controller }
type portal struct{ inner panel }

func NewApp(p *portal) *gin.Engine {
	r := gin.New()
	r.GET("/orders", p.inner.controller.list)
	return r
}
