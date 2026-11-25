package fixture

import "github.com/gin-gonic/gin"

type lister interface{ list(*gin.Context) }
type controller struct{}

func (controller) list(c *gin.Context) {}

func chooseController() lister { return controller{} }

func NewApp() *gin.Engine {
	r := gin.New()
	c := chooseController()
	r.GET("/orders", c.list)
	return r
}
