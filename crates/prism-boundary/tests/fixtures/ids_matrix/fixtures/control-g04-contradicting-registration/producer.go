package lantern

import "github.com/gin-gonic/gin"

func register(r *gin.Engine) { r.GET("/cups", Glaze) }

// @Router /cups [get]
func Glaze(c *gin.Context) {}
