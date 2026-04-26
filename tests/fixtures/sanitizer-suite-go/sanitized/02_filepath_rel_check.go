package main

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	name := c.Param("file")
	rel, err := filepath.Rel("/safe", name)
	if err != nil || strings.HasPrefix(rel, "..") {
		c.AbortWithStatus(403)
		return
	}
	data, _ := os.ReadFile(filepath.Join("/safe", rel))
	c.Data(200, "application/octet-stream", data)
}
