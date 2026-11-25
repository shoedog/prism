package fixture

import "github.com/gin-gonic/gin"

type application struct{}

func (*application) list(c *gin.Context) {}

func (a *application) routes() *gin.Engine {
	r := gin.New()
	r.GET("/orders", a.list)
	return r
}

func NewApp() *gin.Engine { return (&application{}).routes() }
