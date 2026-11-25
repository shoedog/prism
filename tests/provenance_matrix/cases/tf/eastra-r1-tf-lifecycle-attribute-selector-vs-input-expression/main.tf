variable "edition" { type = string }
resource "terraform_data" "edition" {
  input = var.edition
  lifecycle {
    ignore_changes = [input]
  }
}
