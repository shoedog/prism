package fixture

import "github.com/gin-gonic/gin"

type Config struct {
	Base string
}

func list(c *gin.Context) {}

func NewApp(conf Config) *gin.Engine {
	r := gin.New()
	g := r.Group(conf.Base + "/api")
	g.GET("/x", list)
	return r
}
