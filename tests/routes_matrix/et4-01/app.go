package routes

import "github.com/gin-gonic/gin"

func App() *gin.Engine {
	r := gin.New()
	c := &Controller[string]{}
	r.GET("/items", c.List)
	return r
}
