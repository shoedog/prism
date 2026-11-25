package routes_test

import "github.com/gin-gonic/gin"

func status(ctx *gin.Context) {}

func fixtureRouter() *gin.Engine {
	r := gin.New()
	r.GET("/test-only", status)
	return r
}
