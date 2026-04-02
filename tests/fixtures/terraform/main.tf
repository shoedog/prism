# Test fixture: Terraform taint flow through variable references
# var.allowed_cidrs -> local.merged -> security group rule

variable "allowed_cidrs" {
  description = "CIDRs allowed to access the service"
  type        = list(string)
}

variable "instance_type" {
  default = "t3.micro"
}

locals {
  merged_cidrs = concat(var.allowed_cidrs, ["10.0.0.0/8"])
  env_name     = "production"
}

resource "aws_security_group" "web" {
  name = "web-${local.env_name}"

  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = local.merged_cidrs  # Taint: user-supplied CIDRs reach SG rule
  }
}

resource "aws_instance" "web" {
  ami           = "ami-0123456789abcdef0"
  instance_type = var.instance_type
  user_data     = <<-EOF
    #!/bin/bash
    echo "Starting service for ${local.env_name}"
  EOF

  vpc_security_group_ids = [aws_security_group.web.id]
}

output "instance_ip" {
  value = aws_instance.web.public_ip
}
