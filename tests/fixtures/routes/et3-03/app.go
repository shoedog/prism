package routes

import "github.com/gin-gonic/gin"

type Controller[T any] struct{}

func (c *Controller[T]) List(ctx *gin.Context) {}

func register(r *gin.Engine, c *Controller[string]) {
	r.GET("/items", c.List)
}

func NewApp() *gin.Engine {
	r := gin.New()
	register(r, &Controller[string]{})
	return r
}
