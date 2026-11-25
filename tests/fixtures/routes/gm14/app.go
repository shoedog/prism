package fixture

import "github.com/gin-gonic/gin"

type apiRouter struct {
	onList func(*gin.Context)
}

func list(ctx *gin.Context) {}

func (a *apiRouter) routes() *gin.Engine {
	r := gin.New()
	r.GET("/x", a.onList)
	return r
}

func NewApp() *gin.Engine {
	a := &apiRouter{onList: list}
	return a.routes()
}
