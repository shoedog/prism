variable "name" { type = string }
output "labels" {
  value = { name = var.name }
}
