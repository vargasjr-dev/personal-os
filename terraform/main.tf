terraform {
  required_version = ">= 1.0"
  
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

# Elastic IP for stable addressing
resource "aws_eip" "personalos" {
  domain = "vpc"
  
  tags = {
    Name = "PersonalOS"
    Project = "PersonalOS"
  }
}

# Security Group
resource "aws_security_group" "personalos" {
  name        = "personalos-sg"
  description = "Security group for PersonalOS EC2"
  
  # SSH
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "SSH access"
  }
  
  # HTTP
  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "HTTP access"
  }
  
  # HTTPS
  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "HTTPS access"
  }
  
  # noVNC port (direct access fallback)
  ingress {
    from_port   = 6080
    to_port     = 6080
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "noVNC WebSocket"
  }
  
  # Outbound internet access
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  tags = {
    Name = "PersonalOS Security Group"
    Project = "PersonalOS"
  }
}

# EC2 Key Pair (will be created/imported)
resource "aws_key_pair" "personalos" {
  key_name   = "personalos-key"
  public_key = file(var.ssh_public_key_path)
  
  tags = {
    Name = "PersonalOS SSH Key"
    Project = "PersonalOS"
  }
}

# EC2 Instance
resource "aws_instance" "personalos" {
  ami           = data.aws_ami.ubuntu.id
  instance_type = var.instance_type
  
  key_name               = aws_key_pair.personalos.key_name
  vpc_security_group_ids = [aws_security_group.personalos.id]
  
  user_data = templatefile("${path.module}/user-data.sh", {
    setup_script_url = "https://raw.githubusercontent.com/vargasjr-dev/personal-os/main/deployment/ec2/setup-ec2.sh"
  })
  
  root_block_device {
    volume_size = 20
    volume_type = "gp3"
  }
  
  tags = {
    Name = "PersonalOS"
    Project = "PersonalOS"
  }
}

# Associate Elastic IP with EC2
resource "aws_eip_association" "personalos" {
  instance_id   = aws_instance.personalos.id
  allocation_id = aws_eip.personalos.id
}

# Get latest Ubuntu 22.04 LTS AMI
data "aws_ami" "ubuntu" {
  most_recent = true
  owners      = ["099720109477"] # Canonical
  
  filter {
    name   = "name"
    values = ["ubuntu/images/hvm-ssd/ubuntu-jammy-22.04-amd64-server-*"]
  }
  
  filter {
    name   = "virtualization-type"
    values = ["hvm"]
  }
}

# Route 53 Hosted Zone for subdomain
resource "aws_route53_zone" "personalos" {
  name = var.subdomain
  
  tags = {
    Name = "PersonalOS Subdomain Zone"
    Project = "PersonalOS"
  }
}

# A record pointing subdomain to Elastic IP
resource "aws_route53_record" "personalos" {
  zone_id = aws_route53_zone.personalos.zone_id
  name    = var.subdomain
  type    = "A"
  ttl     = 300
  records = [aws_eip.personalos.public_ip]
}

# Outputs
output "elastic_ip" {
  description = "Elastic IP address for PersonalOS"
  value       = aws_eip.personalos.public_ip
}

output "instance_id" {
  description = "EC2 Instance ID"
  value       = aws_instance.personalos.id
}

output "instance_public_dns" {
  description = "EC2 Public DNS"
  value       = aws_instance.personalos.public_dns
}

output "subdomain" {
  description = "Subdomain for PersonalOS"
  value       = var.subdomain
}

output "route53_nameservers" {
  description = "Route 53 nameservers for subdomain delegation"
  value       = aws_route53_zone.personalos.name_servers
}

output "ssh_command" {
  description = "SSH command to connect"
  value       = "ssh -i ${var.ssh_private_key_path} ubuntu@${aws_eip.personalos.public_ip}"
}

output "vnc_url" {
  description = "Direct noVNC access URL"
  value       = "http://${aws_eip.personalos.public_ip}:6080/vnc.html"
}

output "subdomain_url" {
  description = "Subdomain URL (after DNS delegation)"
  value       = "http://${var.subdomain}"
}
