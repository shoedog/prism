package middleware

import "github.com/gin-gonic/gin"

func Ban(c *gin.Context) {
	c.AbortWithStatus(403)
}
