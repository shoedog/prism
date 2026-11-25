variable "items" { type = list(string) }
output "values" {
  value = [for item in var.items : item]
}
