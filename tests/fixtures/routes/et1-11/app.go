package app

import "github.com/gin-gonic/gin"

func fallback(c *gin.Context) {}

func New() *gin.Engine {
	r := gin.New()
	r.NoRoute(fallback)
	return r
}
