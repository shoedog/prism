terraform {
  required_providers {
    http = {
      source = "hashicorp/http"
    }
  }
}
data "http" "catalog" {
  url = "https://example.test/catalog"
}
output "catalog" {
  value = data.http.catalog.response_body
}
