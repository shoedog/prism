package fixture

import "github.com/gin-gonic/gin"

type apiRouter struct {
	engine *gin.Engine
}

func (a *apiRouter) routes() *gin.Engine {
	r := gin.New()
	r.GET("/internal", a.engine.HandleContext)
	return r
}

func NewApp() *gin.Engine {
	a := &apiRouter{engine: gin.New()}
	return a.routes()
}
