package routes

import "github.com/gin-gonic/gin"

func status(ctx *gin.Context) {}

func App() *gin.Engine {
	r := gin.New()
	r.GET("/status", status)
	return r
}
