package routes

import "github.com/gin-gonic/gin"

type Controller struct{}

func App() *gin.Engine {
	r := gin.New()
	c := &Controller{}
	r.GET("/status", c.Status)
	return r
}
