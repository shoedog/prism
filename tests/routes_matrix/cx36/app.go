package fixture

import "github.com/gin-gonic/gin"

type controller struct{}

func (*controller) list(c *gin.Context) {}

func NewController() *controller { return &controller{} }

func NewApp() *gin.Engine {
	r := gin.New()
	c := NewController()
	r.GET("/orders", c.list)
	return r
}
