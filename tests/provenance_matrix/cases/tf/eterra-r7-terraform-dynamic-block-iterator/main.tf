variable "ports" { type = list(number) }
resource "example_service" "test" {
  dynamic "port" {
    for_each = var.ports
    content { number = port.value }
  }
}
