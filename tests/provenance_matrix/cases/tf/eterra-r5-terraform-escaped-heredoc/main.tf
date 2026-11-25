output "message" {
  value = <<EOT
$${var.secret}
EOT
}
