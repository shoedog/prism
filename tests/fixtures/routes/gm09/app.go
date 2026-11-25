package fixture

import "github.com/gin-gonic/gin"

type Lister interface {
	List(*gin.Context)
}

type service struct{}

func (s *service) List(ctx *gin.Context) {}

type apiRouter struct {
	svc Lister
}

func (a *apiRouter) routes() *gin.Engine {
	r := gin.New()
	r.GET("/x", a.svc.List)
	return r
}

func NewApp() *gin.Engine {
	a := &apiRouter{svc: &service{}}
	return a.routes()
}
