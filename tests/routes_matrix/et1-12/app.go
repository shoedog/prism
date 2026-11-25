package app

import (
	"net/http"

	"github.com/gin-gonic/gin"
)

func New() *gin.Engine {
	r := gin.New()
	r.Static("/assets", "public")
	r.StaticFS("/files", http.Dir("public"))
	return r
}
