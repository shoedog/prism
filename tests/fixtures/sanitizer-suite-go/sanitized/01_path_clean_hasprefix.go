package main

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	name := c.Param("file")
	cleaned := filepath.Clean(name)
	if !strings.HasPrefix(cleaned, "/uploads/") {
		c.AbortWithStatus(403)
		return
	}
	data, _ := os.ReadFile(cleaned)
	c.Data(200, "application/octet-stream", data)
}
