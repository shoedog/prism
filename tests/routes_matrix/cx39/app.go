package fixture

import "github.com/gin-gonic/gin"

type controller struct{}

func (controller) list(c *gin.Context) {}

func NewApp() *gin.Engine {
	r := gin.New()
	var c controller
	r.GET("/orders", c.list)
	return r
}
