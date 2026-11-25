variable "name" {
  type = string
}
output "greeting" {
  value = <<-EOT
    hello ${var.name}
  EOT
}
