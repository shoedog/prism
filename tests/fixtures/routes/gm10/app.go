package fixture

import "github.com/gin-gonic/gin"

type Lister interface {
	List(*gin.Context)
}

type service struct{}

func (s *service) List(ctx *gin.Context) {}

func register(l Lister) *gin.Engine {
	r := gin.New()
	r.GET("/x", l.List)
	return r
}

func NewApp() *gin.Engine {
	return register(&service{})
}
