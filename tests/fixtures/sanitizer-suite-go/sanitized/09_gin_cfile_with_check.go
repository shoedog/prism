package main

import (
	"path/filepath"
	"strings"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	name := c.Param("file")
	cleaned := filepath.Clean(name)
	if !strings.HasPrefix(cleaned, "/static/") {
		c.AbortWithStatus(404)
		return
	}
	c.File(cleaned)
}
