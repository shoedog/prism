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
	if !strings.HasPrefix(cleaned, "/writes") {
		return
	}
	_ = os.WriteFile(cleaned, []byte("data"), 0644)
}
