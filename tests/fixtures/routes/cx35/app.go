package fixture

import "github.com/gin-gonic/gin"

type controller struct{}

func (controller) list(c *gin.Context) {}

type panel struct{ controller controller }
type application struct{ panel panel }

func (a *application) routes() *gin.Engine {
	r := gin.New()
	r.GET("/orders", a.panel.controller.list)
	return r
}

func NewApp() *gin.Engine { return (&application{}).routes() }
